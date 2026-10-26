#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute recovered external-call storage; widths, loads and stores must match."""
import argparse, importlib.util, json, re, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/external-call-roundtrip"
SOURCES = ([ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"]
           + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs")))


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec); spec.loader.exec_module(result); return result

support = module("object_roundtrip", ROOT / "tests/object-roundtrip.py")
leaves = module("exact_reference_leaves", ROOT / "tests/support/exact_reference_leaves.py")


def without_dependency_report(text):
    """The diagnostic view appends dependency provenance; it is evidence, not C#."""
    kept = [line for line in text.splitlines()
            if not line.startswith(".dependency") and not line.startswith("// Dependency")]
    return "\n".join(kept) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--mutation-control", action="store_true")
    args = parser.parse_args()
    if not args.cli.is_file(): parser.error("Existing Rust CLI required")
    OUT.mkdir(parents=True, exist_ok=True)
    print(support.run([sys.executable, ROOT / "tests/build-external-call-fixtures.py"]).stdout, end="", flush=True)
    harness = OUT / "ExternalCallRoundtrip.dll"
    compile_ms = support.compile_cs(harness, SOURCES, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/external-call-{config}.bin"
        library = ROOT / f"tests/fixtures/external-call-library-{config}.bin"
        names, owners = {}, {}
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            found = re.search(r"(\S+)::([^\s(]+)\(", signature)
            if found:
                owners[token] = found.group(1); names[token] = found.group(2)
        scan = support.run([args.cli, "scan", fixture, "--no-auto-references",
                            "--reference", library])
        selected = [(record["token"], names[record["token"]])
                    for record in map(json.loads, scan.stdout.splitlines())
                    if record["status"] == "Decompiled" and owners.get(record["token"]) == "ExternalCallCases.Ops"]
        if not selected: raise ValueError("external-call selection is empty")
        snippets = []
        for token, name in selected:
            details = support.run([args.cli, "decompile", fixture, "--method", token, "--details",
                                   "--no-auto-references", "--reference", library])
            if details.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0":
                raise ValueError("external-call method not recovered: " + name)
            # The claim under test is the default readable output, so that is what
            # is compiled; the diagnostic view only has to agree on admission.
            readable = support.run([args.cli, "decompile", fixture, "--method", token,
                                    "--no-auto-references", "--reference", library])
            if readable.stderr != details.stderr:
                raise ValueError("view changed admission for " + name)
            body = without_dependency_report(readable.stdout)
            (OUT / f"{config}-{name}.details.txt").write_text(details.stdout)
            (OUT / f"{config}-{name}.readable.txt").write_text(readable.stdout)
            snippets.append(body)
        if not snippets: raise ValueError("external-call carrier is empty")
        aliases, helpers = leaves.plan(support, harness, fixture, OUT, snippets, "ExternalCallCases.Ops", library)
        carrier = (aliases + "namespace ExternalCallCases\n{\npublic static class Recovered\n{\n"
                   + helpers + "\n".join(snippets) + "\n}\n}\n")
        source = OUT / f"Recovered-{config}.cs"
        support.write_changed(source, carrier)
        recovered = source.with_suffix(".dll")
        compile_ms += support.compile_cs(recovered, [source], references=[fixture, library])
        result = support.run([support.DOTNET / "dotnet", harness, fixture,
                              "--external-call-check", recovered, library])
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip(),
                     "selected": [name for _, name in selected]})
        if args.mutation_control:
            # Pointing the dispatch at a different declared method on the same
            # receiver must change a result; that is what makes the target name
            # observable rather than cosmetic.
            bad, count = re.subn(r"\.Virtual\(", ".@Direct(", carrier, count=1)
            if count != 1: raise ValueError("external-call mutation target missing")
            path = OUT / f"WrongTarget-{config}.cs"
            support.write_changed(path, bad)
            binary = path.with_suffix(".dll")
            compile_ms += support.compile_cs(binary, [path], references=[fixture, library])
            mutated = subprocess.run([str(support.DOTNET / "dotnet"), str(harness), str(fixture),
                                      "--external-call-check", str(binary), str(library)],
                                     text=True, capture_output=True, timeout=300)
            (OUT / f"WrongTarget-mutation-{config}.log").write_text(mutated.stdout + mutated.stderr)
            if mutated.returncode == 0:
                raise ValueError("mutation control survived: wrong call target accepted")
            rows.append({"configuration": config, "mutation": "wrong-target",
                         "detected": mutated.stderr.strip().splitlines()[0] if mutated.stderr.strip() else "nonzero exit"})
            print(config + " mutation: wrong call target rejected", flush=True)
    (OUT / "behavior-results.json").write_text(json.dumps(
        {"configurations": rows, "compile_ms": compile_ms,
         "mutation_control": args.mutation_control, "third_party_execution": False}, indent=2) + "\n")
    print(json.dumps({"compile_ms": compile_ms, "rows": rows}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
