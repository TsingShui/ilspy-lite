#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute recovered enum transfers against owned fixture bodies and exact metadata."""
import argparse
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/enum-roundtrip"


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


support = module("object_roundtrip", ROOT / "tests/object-roundtrip.py")
adapter = module("reference_roundtrip", ROOT / "tests/reference-roundtrip.py")


SOURCES = ([ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"]
           + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs")))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--readable", action="store_true")
    parser.add_argument("--mutation-control", action="store_true")
    args = parser.parse_args()
    if not args.cli.is_file():
        parser.error("Existing Rust CLI required")
    OUT.mkdir(parents=True, exist_ok=True)
    print(support.run([sys.executable, ROOT / "tests/build-enum-fixtures.py"]).stdout, end="", flush=True)
    harness = OUT / "EnumRoundtrip.dll"
    compile_ms = support.compile_cs(harness, SOURCES, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/enum-{config}.bin"
        selected = []
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            match = re.search(r" EnumCases\.((?:S|U)(?:8|16|32|64)Ops)::([A-Za-z][A-Za-z0-9_]*)\(", signature)
            if match and not match[2].startswith("Unsupported"):
                selected.append((token, match[1], match[2]))
        if len(selected) != 96:
            raise ValueError("Owned enum selection drift")
        scan = {record["token"]: record
                for record in map(json.loads, support.run(
                    [args.cli, "scan", fixture, "--no-auto-references"]).stdout.splitlines())}
        # An enum unbox is only executable where its storage conversion is
        # written; the two 32/64-bit storage owners are the recovered ones.
        unbox = []
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            match = re.search(r" EnumCases\.((?:S|U)(?:8|16|32|64)Ops)::UnsupportedUnbox\(", signature)
            if match and scan[token]["status"] == "Decompiled":
                unbox.append((token, match[1], "UnsupportedUnbox"))
        if sorted(owner for _, owner, _ in unbox) != ["S32Ops", "S64Ops"]:
            raise ValueError("Owned enum unbox selection drift")
        groups, identities = {}, {}
        for token, owner, name in selected + unbox:
            details = support.run([args.cli, "decompile", fixture, "--method", token, "--details", "--no-auto-references"])
            if details.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0":
                raise ValueError("Enum method not recovered: " + owner + "." + name)
            (OUT / f"{config}-{owner}-{name}.details.txt").write_text(details.stdout)
            for raw, alias, storage in re.findall(r"// enum storage token=0x([0-9A-F]{8}) alias=(\w+) underlying=(\w+):", details.stdout):
                entry = {"token": int(raw, 16), "alias": alias, "storage": storage}
                if alias in identities and identities[alias] != entry:
                    raise ValueError("Conflicting enum storage evidence")
                identities[alias] = entry
            statements = details.stdout
            if args.readable:
                readable = support.run([args.cli, "decompile", fixture, "--method", token, "--no-auto-references"])
                if readable.stderr != details.stderr:
                    raise ValueError("Enum view changed admission")
                (OUT / f"{config}-{owner}-{name}.readable.txt").write_text(readable.stdout)
                statements = adapter.adapt_readable_body(readable.stdout, details.stdout)
            # Rebind method targets only. Enum state remains in its original owned
            # fields, never accidentally redirected to an empty carrier field.
            rebound = re.sub(rf"\b(?:global::)?@?EnumCases\.@?{owner}\.(?=@?[A-Za-z_][A-Za-z_0-9]*\()", "Recovered.", statements)
            if name == "Forward" and not re.search(r"Recovered\.@?Identity\(", rebound):
                raise ValueError("Enum helper was not rebound")
            groups.setdefault(owner, []).append(rebound)
        if len(identities) != 8 or len(groups) != 8:
            raise ValueError("Owned enum storage coverage drift")
        directives = OUT / f"aliases-{config}.json"
        support.write_changed(directives, json.dumps(list(identities.values()), indent=2))
        aliases = support.run([support.DOTNET / "dotnet", harness, fixture, "--enum-plan", directives]).stdout
        pieces = {owner: "namespace Enum_" + owner[:-3] + "\n{\npublic static class Recovered\n{\n"
                  + "\n".join(snippets) + "\n}\n}\n" for owner, snippets in groups.items()}
        text = aliases + "\n" + "\n".join(pieces.values())
        suffix = config + ("-readable" if args.readable else "-details")
        source = OUT / f"Recovered-{suffix}.cs"
        support.write_changed(source, text)
        recovered = source.with_suffix(".dll")
        compile_ms += support.compile_cs(recovered, [source], references=[fixture])
        result = support.run([support.DOTNET / "dotnet", harness, fixture, "--enum-check", recovered])
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip()})
        if args.mutation_control:
            mutants = []
            for label, owner, method, replacement, mode in [
                ("Helper", "S8Ops", "Identity", "default", "--enum-forward"),
                ("Signed", "S8Ops", "Widen", "unchecked((long)(byte)a0)", "--enum-check"),
                ("Unsigned", "U32Ops", "Widen", "unchecked((long)(int)(uint)a0)", "--enum-check"),
            ]:
                pattern = rf"public static [^\n]+ @{method}\([^\n]+\)\n\{{\n.*?\n\}}"
                target = re.search(pattern, pieces[owner], re.S)
                if target is None:
                    raise ValueError("Enum mutation target missing")
                changed, count = re.subn(r"(?m)^(\s*)return [^;]+;", lambda m: m[1] + "return " + replacement + ";", target[0], count=1)
                if count != 1:
                    raise ValueError("Enum mutation return missing")
                altered = pieces[owner][:target.start()] + changed + pieces[owner][target.end():]
                mutant = aliases + "\n" + "\n".join(altered if key == owner else value for key, value in pieces.items())
                expected = owner + "." + ("Forward" if label == "Helper" else method)
                mutants.append((label, mutant, expected, mode))
            altered, count = re.subn(r"(?m)^\s*(?:global::)?@?EnumCases\.@?S8Ops\.@?Global = [^\n]+;\n", "", pieces["S8Ops"])
            if count != 1:
                raise ValueError("Enum static state mutation target missing")
            mutants.append(("State", aliases + "\n" + "\n".join(altered if key == "S8Ops" else value for key, value in pieces.items()),
                            "S8Ops.StaticExchange", "--enum-check"))
            # The unbox cast must name this enum: a sibling enum is a different
            # runtime type, so a wrong target has to be observable.
            target = re.search(r"public static [^\n]+ @?UnsupportedUnbox\([^\n]+\)\n\{\n.*?\n\}", pieces["S32Ops"], re.S)
            if target is None:
                raise ValueError("Enum unbox mutation target missing")
            # Only the cast itself, never the evidence comment that names the enum.
            changed, count = re.subn(r"\((?:global::)?@?EnumCases\.@?S32\)\(", "(global::EnumCases.U32)(", target[0], count=1)
            if count != 1:
                raise ValueError("Enum unbox cast missing")
            altered = pieces["S32Ops"][:target.start()] + changed + pieces["S32Ops"][target.end():]
            mutants.append(("UnboxTarget", aliases + "\n" + "\n".join(
                altered if key == "S32Ops" else value for key, value in pieces.items()),
                "unbox differs: S32", "--enum-check"))
            for label, mutated, expected, mode in mutants:
                path = OUT / f"{label}Mutant-{suffix}.cs"
                support.write_changed(path, mutated)
                binary = path.with_suffix(".dll")
                compile_ms += support.compile_cs(binary, [path], references=[fixture])
                negative = subprocess.run([str(support.DOTNET / "dotnet"), str(harness), str(fixture), mode, str(binary)],
                                          text=True, capture_output=True, timeout=120)
                (OUT / f"{label}-mutation-{suffix}.log").write_text(negative.stdout + negative.stderr)
                marker = expected if expected.startswith("unbox differs: ") else "behavior differs: " + expected
                if negative.returncode == 0 or "Recovered enum " + marker not in negative.stdout + negative.stderr:
                    raise ValueError("Enum mutation not detected: " + label)
                print(config + ": rejected enum " + label.lower() + " mutant", flush=True)
    view = "readable" if args.readable else "details"
    (OUT / f"behavior-results-{view}.json").write_text(json.dumps({"configurations": rows, "compile_ms": compile_ms,
        "view": view, "mutation_control": args.mutation_control, "third_party_execution": False}, indent=2) + "\n")
    print(f"Owned compilation: {compile_ms} ms; Rust compilation: none")


if __name__ == "__main__":
    main()
