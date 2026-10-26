#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute recovered runtime type checks, preserving identity and primitive results."""
import argparse, importlib.util, json, re, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/cast-roundtrip"
SOURCES = ([ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"]
           + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs")))



def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec); spec.loader.exec_module(result); return result

support = module("object_roundtrip", ROOT / "tests/object-roundtrip.py")
adapter = module("reference_roundtrip", ROOT / "tests/reference-roundtrip.py")
structural = module("generic_declaring", ROOT / "tests/generic-declaring-roundtrip.py")
leaves = module("exact_reference_leaves", ROOT / "tests/support/exact_reference_leaves.py")

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--readable", action="store_true")
    parser.add_argument("--mutation-control", action="store_true")
    args = parser.parse_args()
    if not args.cli.is_file(): parser.error("Existing Rust CLI required")
    OUT.mkdir(parents=True, exist_ok=True)
    print(support.run([sys.executable, ROOT / "tests/build-cast-fixtures.py"]).stdout, end="", flush=True)
    harness = OUT / "CastRoundtrip.dll"
    compile_ms = support.compile_cs(harness, SOURCES, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/cast-{config}.bin"
        names, owners = {}, {}
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            found = re.search(r"(\S+)::([^\s(]+)\(", signature)
            if found:
                owners[token] = found.group(1); names[token] = found.group(2)
        selected = [(record["token"], names[record["token"]])
                    for record in map(json.loads, support.run([args.cli, "scan", fixture]).stdout.splitlines())
                    if record["status"] == "Decompiled" and owners.get(record["token"]) == "CastCases.Ops"]
        if not selected: raise ValueError("equality selection is empty")
        snippets = []
        bodies = {}
        for token, name in selected:
            details = support.run([args.cli, "decompile", fixture, "--method", token, "--details", "--no-auto-references"])
            if details.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0":
                raise ValueError("equality method not recovered: " + name)
            (OUT / f"{config}-{name}.details.txt").write_text(details.stdout)
            statements = details.stdout
            if args.readable:
                readable = support.run([args.cli, "decompile", fixture, "--method", token, "--no-auto-references"])
                if readable.stderr != details.stderr: raise ValueError("view changed admission")
                (OUT / f"{config}-{name}.readable.txt").write_text(readable.stdout)
                statements = adapter.adapt_readable_body(readable.stdout, details.stdout)
            statements = structural.inline_structural_types(statements)
            snippets.append(statements)
            bodies[name] = statements
        aliases, helpers = leaves.plan(support, harness, fixture, OUT, snippets, "CastCases.Ops")
        carrier = aliases + "namespace CastCases\n{\npublic static class Recovered\n{\n" + helpers + "\n".join(snippets) + "\n}\n}\n"
        source = OUT / f"Recovered-{config}{'-readable' if args.readable else '-details'}.cs"
        support.write_changed(source, carrier)
        recovered = source.with_suffix(".dll")
        compile_ms += support.compile_cs(recovered, [source], references=[fixture])
        result = support.run([support.DOTNET / "dotnet", harness, fixture, "--cast-check", recovered])
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip()})
        if args.mutation_control:
            # Widening the cast target must be detected: a rejected value would
            # then be accepted instead of throwing InvalidCastException.
            # Turning the null-yielding type test into a throwing cast keeps the
            # types intact, so the mutant still compiles but changes behaviour.
            bad, count = re.subn(r"\(\(object\)(\w+) as ([A-Za-z0-9_@:.]+)\)",
                                 r"((\2)(object)\1)", carrier, count=1)
            if count != 1: raise ValueError("cast mutation target missing")
            path = OUT / f"WidenedCast-{config}.cs"
            support.write_changed(path, bad)
            binary = path.with_suffix(".dll")
            compile_ms += support.compile_cs(binary, [path], references=[fixture])
            negative = subprocess.run([str(support.DOTNET / "dotnet"), str(harness), str(fixture), "--cast-check", str(binary)],
                                      text=True, capture_output=True, timeout=120)
            (OUT / f"WidenedCast-mutation-{config}.log").write_text(negative.stdout + negative.stderr)
            if negative.returncode == 0: raise ValueError("cast mutation was not detected")
            if "Recovered cast differs:" not in negative.stdout + negative.stderr:
                raise ValueError("Mutation failed outside the behavior comparison")
            print(config + ": rejected widened-cast mutant", flush=True)
            for name in ["AsAbstract", "IsAbstract", "Guard", "UnsupportedString"]:
                if name not in bodies: raise ValueError("Missing cast mutation method: " + name)
                body = bodies[name]
                if name in ("AsAbstract", "UnsupportedString"):
                    changed, count = re.subn(r"\(\(object\)(\w+) as ([A-Za-z0-9_@:.]+)\)",
                                             r"((\2)(object)\1)", body, count=1)
                else:
                    changed, count = re.subn(r"return [^;]+;", "return " + ("false" if name == "IsAbstract" else "0") + ";",
                                             body, count=1)
                if count != 1: raise ValueError("Missing targeted mutation expression: " + name)
                path = OUT / f"Mutant-{name}-{config}.cs"
                support.write_changed(path, carrier.replace(body, changed, 1))
                binary = path.with_suffix(".dll")
                compile_ms += support.compile_cs(binary, [path], references=[fixture])
                negative = subprocess.run([str(support.DOTNET / "dotnet"), str(harness), str(fixture),
                                           "--cast-check", str(binary)], text=True, capture_output=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Mutant-{name}-{config}.log").write_text(log)
                if negative.returncode == 0 or "Recovered cast differs: " + name not in log:
                    raise ValueError("Targeted mutation was not detected: " + name + "\n" + log)
                print(config + ": rejected " + name + " mutant", flush=True)
    view = "readable" if args.readable else "details"
    (OUT / f"behavior-results-{view}.json").write_text(json.dumps({"configurations": rows, "compile_ms": compile_ms,
        "view": view, "mutation_control": args.mutation_control, "third_party_execution": False}, indent=2) + "\n")

if __name__ == "__main__":
    main()
