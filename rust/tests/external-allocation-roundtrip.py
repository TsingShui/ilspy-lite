#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute recovered external-value storage; widths, loads and stores must match."""
import argparse, importlib.util, json, re, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/external-allocation-roundtrip"
SOURCES = ([ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"]
           + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs")))


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec); spec.loader.exec_module(result); return result

support = module("object_roundtrip", ROOT / "tests/object-roundtrip.py")
generic = module("generic_value_roundtrip", ROOT / "tests/generic-value-roundtrip.py")


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
    print(support.run([sys.executable, ROOT / "tests/build-external-allocation-fixtures.py"]).stdout, end="", flush=True)
    harness = OUT / "ExternalAllocationRoundtrip.dll"
    compile_ms = support.compile_cs(harness, SOURCES, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/external-allocation-{config}.bin"
        library = ROOT / f"tests/fixtures/external-allocation-library-{config}.bin"
        names, owners, signatures = {}, {}, {}
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            found = re.search(r"(\S+)::([^\s(]+)\(", signature)
            if found:
                owners[token] = found.group(1); names[token] = found.group(2)
                signatures[token] = signature
        scan = support.run([args.cli, "scan", fixture, "--no-auto-references",
                            "--reference", library])
        selected = [(record["token"], names[record["token"]])
                    for record in map(json.loads, scan.stdout.splitlines())
                    if record["status"] == "Decompiled" and owners.get(record["token"]) == "ExternalAllocationCases.Ops"]
        if not selected: raise ValueError("external-allocation selection is empty")
        snippets, skipped = [], []
        for token, name in selected:
            details = support.run([args.cli, "decompile", fixture, "--method", token, "--details",
                                   "--no-auto-references", "--reference", library])
            if details.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0":
                raise ValueError("external-allocation method not recovered: " + name)
            # The claim under test is the default readable output, so that is what
            # is compiled; the diagnostic view only has to agree on admission.
            readable = support.run([args.cli, "decompile", fixture, "--method", token,
                                    "--no-auto-references", "--reference", library])
            if readable.stderr != details.stderr:
                raise ValueError("view changed admission for " + name)
            body = generic.inline_structural_types(without_dependency_report(readable.stdout))
            # These methods return the object they allocate, so the declared return
            # type is the owner: `__cil_type_X` and `__cil_newobj_X` become it.
            owner = re.match(
                r"(?:class|valuetype) \[[^\]]+\](.+?) ExternalAllocationCases\.Ops::",
                signatures[token],
            )
            if owner is None:
                raise ValueError("external-allocation selection is not an object return: " + name)
            owner = owner.group(1).replace("/", ".")
            body = re.sub(r"__cil_type_[0-9A-F]{8}", owner, body)
            body = re.sub(r"__cil_newobj_[0-9A-F]{8}\(", "new " + owner + "(", body)
            (OUT / f"{config}-{name}.details.txt").write_text(details.stdout)
            (OUT / f"{config}-{name}.readable.txt").write_text(readable.stdout)
            code = "\n".join(line for line in body.splitlines()
                              if not line.lstrip().startswith("//"))
            if "__cil_" in code:
                skipped.append(name)
                continue
            snippets.append(body)
        # Every selected body here is compilable; an unusable alias would mean the
        # readable view regressed, so it must fail rather than shrink the selection.
        if skipped:
            raise ValueError("unexpected unexecutable selection: " + repr(skipped))
        if not snippets: raise ValueError("external-allocation carrier is empty")
        carrier = ("namespace ExternalAllocationCases\n{\npublic static class Recovered\n{\n"
                   + "\n".join(snippets) + "\n}\n}\n")
        source = OUT / f"Recovered-{config}.cs"
        support.write_changed(source, carrier)
        recovered = source.with_suffix(".dll")
        compile_ms += support.compile_cs(recovered, [source], references=[fixture, library])
        result = support.run([support.DOTNET / "dotnet", harness, fixture,
                              "--external-allocation-check", recovered, library])
        print(config + ": " + result.stdout.strip() + f" [skipped {skipped}]", flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip(),
                     "skipped": skipped})
        if args.mutation_control:
            # Returning null where a fresh instance is constructed must change the
            # observed object, so the identity/field comparison is the gate.
            import re as _re
            pattern = r"(public static [^\n]*MakeNode\([^)]*\)\s*\{)(.*?)(\n\})"
            bad, count = _re.subn(pattern, r"\1\n        return null;\3", carrier, count=1, flags=_re.S)
            if count != 1: raise ValueError("external-allocation mutation target missing")
            path = OUT / f"NullAllocation-{config}.cs"
            support.write_changed(path, bad)
            binary = path.with_suffix(".dll")
            compile_ms += support.compile_cs(binary, [path], references=[fixture, library])
            mutated = subprocess.run([str(support.DOTNET / "dotnet"), str(harness), str(fixture),
                                      "--external-allocation-check", str(binary), str(library)],
                                     text=True, capture_output=True, timeout=300)
            (OUT / f"NullAllocation-mutation-{config}.log").write_text(mutated.stdout + mutated.stderr)
            if mutated.returncode == 0:
                raise ValueError("mutation control survived: null allocation accepted")
            rows.append({"configuration": config, "mutation": "null-allocation",
                         "detected": mutated.stderr.strip().splitlines()[0] if mutated.stderr.strip() else "nonzero exit"})
            print(config + " mutation: null allocation rejected", flush=True)
    (OUT / "behavior-results.json").write_text(json.dumps(
        {"configurations": rows, "compile_ms": compile_ms,
         "mutation_control": args.mutation_control, "third_party_execution": False}, indent=2) + "\n")
    print(json.dumps({"compile_ms": compile_ms, "rows": rows}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
