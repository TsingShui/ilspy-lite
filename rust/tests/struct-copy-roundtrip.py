#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute actual recovered whole-struct copies using owned type and field definitions."""
import argparse
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/struct-copy-roundtrip"


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
    print(support.run([sys.executable, ROOT / "tests/build-struct-copy-fixtures.py"]).stdout, end="", flush=True)
    harness = OUT / "StructCopyRoundtrip.dll"
    compile_ms = support.compile_cs(harness, SOURCES, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/struct-copy-{config}.bin"
        selected = []
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            match = re.search(r" StructCopyCases\.(\w+Ops)::([A-Za-z][A-Za-z0-9_]*)\(", signature)
            if match and (not match[2].startswith("Unsupported") or match[2] in {"UnsupportedDefault", "UnsupportedMember", "UnsupportedNew"}):
                selected.append((token, match[1], match[2]))
        if not selected:
            raise ValueError("Owned struct selection drift")
        groups, identities = {}, {}
        for token, owner, name in selected:
            details = support.run([args.cli, "decompile", fixture, "--method", token, "--details", "--no-auto-references"])
            if details.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0":
                raise ValueError("Struct method not recovered: " + owner + "." + name)
            (OUT / f"{config}-{owner}-{name}.details.txt").write_text(details.stdout)
            for alias, signature in re.findall(r"// structural signature (__cil_value_\w+) bytes=([0-9A-F]+):", details.stdout):
                if alias in identities and identities[alias] != signature:
                    raise ValueError("Conflicting struct identity evidence")
                identities[alias] = signature
            statements = details.stdout
            if args.readable:
                readable = support.run([args.cli, "decompile", fixture, "--method", token, "--no-auto-references"])
                if readable.stderr != details.stderr:
                    raise ValueError("Struct view changed admission")
                (OUT / f"{config}-{owner}-{name}.readable.txt").write_text(readable.stdout)
                statements = adapter.adapt_readable_body(readable.stdout, details.stdout)
            rebound = re.sub(rf"\b(?:global::)?@?StructCopyCases\.@?{owner}\.(?=@?[A-Za-z_][A-Za-z_0-9]*\()", "Recovered.", statements)
            if name == "Forward" and not re.search(r"Recovered\.@?Identity\(", rebound):
                raise ValueError("Struct helper was not rebound")
            groups.setdefault(owner, []).append(rebound)
        if not identities or not groups:
            raise ValueError("Owned struct type coverage drift")
        directives = OUT / f"aliases-{config}.json"
        support.write_changed(directives, json.dumps([{"alias": key, "bytes": value} for key, value in identities.items()], indent=2))
        aliases = support.run([support.DOTNET / "dotnet", harness, fixture, "--struct-plan", directives]).stdout
        pieces = {owner: "namespace Struct_" + owner[:-3] + "\n{\npublic static class Recovered\n{\n"
                  + "\n".join(snippets) + "\n}\n}\n" for owner, snippets in groups.items()}
        text = aliases + "\n" + "\n".join(pieces.values())
        suffix = config + ("-readable" if args.readable else "-details")
        source = OUT / f"Recovered-{suffix}.cs"
        support.write_changed(source, text)
        recovered = source.with_suffix(".dll")
        compile_ms += support.compile_cs(recovered, [source], references=[fixture])
        result = support.run([support.DOTNET / "dotnet", harness, fixture, "--struct-check", recovered])
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip()})
        if args.mutation_control:
            mutants = []
            for label, owner, method, statement, mode in [
                ("Helper", "PairOps", "Identity", "a0 = default;", "--struct-forward"),
                ("Snapshot", "PairOps", "Snapshot", "a0 = a1;", "--struct-check"),
                ("Reference", "PairOps", "Identity", "if (a0.Tag is string text) a0.Tag = new string(text.ToCharArray());", "--struct-check"),
                ("Constructor", "WithCtorOps", "Identity", "var ignored = new global::StructCopyCases.WithCtor();", "--struct-check"),
                ("Bits", "UnionOps", "Identity", "a0.Bits ^= 0x80000000u;", "--struct-check"),
            ]:
                pattern = rf"(public static [^\n]+ @{method}\([^\n]+\)\n\{{\n)"
                altered, count = re.subn(pattern, lambda m: m[1] + "    " + statement + "\n", pieces[owner], count=1)
                if count != 1:
                    raise ValueError("Struct mutation target missing")
                mutant = aliases + "\n" + "\n".join(altered if key == owner else value for key, value in pieces.items())
                expected = owner + "." + ("Forward" if label == "Helper" else method)
                mutants.append((label, mutant, expected, mode))
            altered, count = re.subn(r"(?m)^\s*(?:global::)?@?StructCopyCases\.@?PairOps\.@?Global = [^\n]+;\n", "", pieces["PairOps"])
            if count != 1:
                raise ValueError("Struct static state mutation target missing")
            mutants.append(("State", aliases + "\n" + "\n".join(altered if key == "PairOps" else value for key, value in pieces.items()),
                            "PairOps.StaticExchange", "--struct-check"))
            for label, mutated, expected, mode in mutants:
                path = OUT / f"{label}Mutant-{suffix}.cs"
                support.write_changed(path, mutated)
                binary = path.with_suffix(".dll")
                compile_ms += support.compile_cs(binary, [path], references=[fixture])
                negative = subprocess.run([str(support.DOTNET / "dotnet"), str(harness), str(fixture), mode, str(binary)],
                                          text=True, capture_output=True, timeout=120)
                (OUT / f"{label}-mutation-{suffix}.log").write_text(negative.stdout + negative.stderr)
                if negative.returncode == 0 or "Recovered struct behavior differs: " + expected not in negative.stdout + negative.stderr:
                    raise ValueError("Struct mutation not detected: " + label)
                print(config + ": rejected struct " + label.lower() + " mutant", flush=True)
    view = "readable" if args.readable else "details"
    (OUT / f"behavior-results-{view}.json").write_text(json.dumps({"configurations": rows, "compile_ms": compile_ms,
        "view": view, "mutation_control": args.mutation_control, "third_party_execution": False}, indent=2) + "\n")
    print(f"Owned compilation: {compile_ms} ms; Rust compilation: none")


if __name__ == "__main__":
    main()
