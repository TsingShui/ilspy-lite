#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute recovered identity comparisons; null and aliasing must not dereference."""
import argparse, importlib.util, json, re, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/equality-roundtrip"
SOURCES = ([ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"]
           + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs")))



def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec); spec.loader.exec_module(result); return result

support = module("object_roundtrip", ROOT / "tests/object-roundtrip.py")
adapter = module("reference_roundtrip", ROOT / "tests/reference-roundtrip.py")
leaves = module("exact_reference_leaves", ROOT / "tests/support/exact_reference_leaves.py")

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--readable", action="store_true")
    parser.add_argument("--mutation-control", action="store_true")
    args = parser.parse_args()
    if not args.cli.is_file(): parser.error("Existing Rust CLI required")
    OUT.mkdir(parents=True, exist_ok=True)
    print(support.run([sys.executable, ROOT / "tests/build-equality-fixtures.py"]).stdout, end="", flush=True)
    harness = OUT / "EqualityRoundtrip.dll"
    compile_ms = support.compile_cs(harness, SOURCES, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/equality-{config}.bin"
        names, owners = {}, {}
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            found = re.search(r"(\S+)::([^\s(]+)\(", signature)
            if found:
                owners[token] = found.group(1); names[token] = found.group(2)
        selected = [(record["token"], names[record["token"]])
                    for record in map(json.loads, support.run([args.cli, "scan", fixture]).stdout.splitlines())
                    if record["status"] == "Decompiled" and owners.get(record["token"]) == "EqualityCases.Ops"]
        if not selected: raise ValueError("equality selection is empty")
        snippets = []
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
            snippets.append(statements)
        aliases, helpers = leaves.plan(support, harness, fixture, OUT, snippets, "EqualityCases.Ops")
        carrier = aliases + "namespace EqualityCases\n{\npublic static class Recovered\n{\n" + helpers + "\n".join(snippets) + "\n}\n}\n"
        source = OUT / f"Recovered-{config}{'-readable' if args.readable else '-details'}.cs"
        support.write_changed(source, carrier)
        recovered = source.with_suffix(".dll")
        compile_ms += support.compile_cs(recovered, [source], references=[fixture])
        result = support.run([support.DOTNET / "dotnet", harness, fixture, "--equality-check", recovered])
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip()})
        if args.mutation_control:
            # Comparing values instead of identities must be detected.
            # The readable view may drop the `global::System.Object.` qualifier;
            # accept either spelling and require exactly one replacement.
            bad, count = re.subn(r"(?:(?:global::)?System\.Object\.)?ReferenceEquals\(([^,]+), ([^)]+)\)",
                                 r"global::System.Object.Equals(\1, \2)", carrier, count=1)
            if count != 1: raise ValueError("equality mutation target missing")
            path = OUT / f"ValueEquality-{config}.cs"
            support.write_changed(path, bad)
            binary = path.with_suffix(".dll")
            compile_ms += support.compile_cs(binary, [path], references=[fixture])
            negative = subprocess.run([str(support.DOTNET / "dotnet"), str(harness), str(fixture), "--equality-check", str(binary)],
                                      text=True, capture_output=True, timeout=120)
            (OUT / f"ValueEquality-mutation-{config}.log").write_text(negative.stdout + negative.stderr)
            if negative.returncode == 0: raise ValueError("value-equality mutant was not detected")
            print(config + ": rejected value-equality mutant", flush=True)
    view = "readable" if args.readable else "details"
    (OUT / f"behavior-results-{view}.json").write_text(json.dumps({"configurations": rows, "compile_ms": compile_ms,
        "view": view, "mutation_control": args.mutation_control, "third_party_execution": False}, indent=2) + "\n")

if __name__ == "__main__":
    main()
