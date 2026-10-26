#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute recovered allocation bodies; each newobj must create one fresh identity."""
import argparse, importlib.util, json, re, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/new-object-roundtrip"
SOURCES = ([ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"]
           + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs")))



def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec); spec.loader.exec_module(result); return result

support = module("object_roundtrip", ROOT / "tests/object-roundtrip.py")
adapter = module("reference_roundtrip", ROOT / "tests/reference-roundtrip.py")

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--readable", action="store_true")
    parser.add_argument("--mutation-control", action="store_true")
    args = parser.parse_args()
    if not args.cli.is_file(): parser.error("Existing Rust CLI required")
    OUT.mkdir(parents=True, exist_ok=True)
    print(support.run([sys.executable, ROOT / "tests/build-new-object-fixtures.py"]).stdout, end="", flush=True)
    harness = OUT / "NewObjectRoundtrip.dll"
    compile_ms = support.compile_cs(harness, SOURCES, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/new-object-{config}.bin"
        names, owners, instance = {}, {}, {}
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            found = re.search(r"(\S+)::([^\s(]+)\(", signature)
            if found:
                owners[token] = found.group(1)
                names[token] = found.group(2)
                instance[token] = signature.split(" ", 1)[0] == "instance"
        selected = [(record["token"], names[record["token"]])
                    for record in map(json.loads, support.run([args.cli, "scan", fixture]).stdout.splitlines())
                    if record["status"] == "Decompiled" and owners.get(record["token"], "").startswith("NewObjectCases.Ops")]
        if not selected: raise ValueError("allocation selection is empty")
        snippets, shells = [], {}
        for token, name in selected:
            details = support.run([args.cli, "decompile", fixture, "--method", token, "--details", "--no-auto-references"])
            if details.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0":
                raise ValueError("allocation method not recovered: " + name)
            (OUT / f"{config}-{name}.details.txt").write_text(details.stdout)
            statements = details.stdout
            if args.readable and not name.startswith("UnsupportedGenericNew"):
                readable = support.run([args.cli, "decompile", fixture, "--method", token, "--no-auto-references"])
                if readable.stderr != details.stderr: raise ValueError("view changed admission")
                (OUT / f"{config}-{name}.readable.txt").write_text(readable.stdout)
                statements = adapter.adapt_readable_body(readable.stdout, details.stdout)
            if name.startswith("UnsupportedGenericNew"):
                # This generic allocation is the exact core Activator MethodSpec,
                # not an opaque newobj or a proxy for this recovered method.
                readable = support.run([args.cli, "decompile", fixture, "--method", token, "--no-auto-references"]).stdout
                rows = re.findall(r"structural signature (__cil_generic_\w+) bytes=1E00:", readable)
                if len(rows) != 1: raise ValueError("Generic factory alias missing")
                il = support.run([args.cli, "il", fixture, "--method", token]).stdout
                if "System.Activator::CreateInstance<!!0>" not in il:
                    raise ValueError("Generic factory is not the expected core leaf")
                statements = readable.replace(rows[0], "T")
                statements, count = re.subn(r"__cil_methodspec_call_[0-9A-F]{8}\(\)",
                                           "global::System.Activator.CreateInstance<T>()", statements)
                if count != 1: raise ValueError("Generic factory call missing")
            for token_hex in re.findall(r"__cil_newobj_([0-9A-F]{8})\(", statements):
                key = "0x" + token_hex
                owner = owners.get(key)
                if owner is None or not owner.startswith("NewObjectCases."):
                    raise ValueError("newobj target outside the owned fixture: " + key)
                shells[key] = owner
            def allocate(match):
                arguments = match.group(2).strip()
                packed = "new object[]{" + arguments + "}" if arguments else "new object[0]"
                return f"__cil_newobj_{match.group(1)}({packed})"
            statements = re.sub(r"__cil_newobj_([0-9A-F]{8})\(([^()]*)\)", allocate, statements)

            def direct(match):
                key = "0x" + match.group(1)
                if key not in names or not instance.get(key):
                    raise ValueError("direct call shell needed for " + key)
                parts = [part.strip() for part in match.group(2).split(",")]
                # An instance direct call is exactly the same non-virtual call in C#.
                return f"{parts[0]}.{names[key]}({', '.join(parts[1:])})"

            statements = re.sub(r"__cil_direct_([0-9A-F]{8})\(([^()]*)\)", direct, statements)
            snippets.append(statements)
        declared = "\n".join(
            f"static global::{owner} __cil_newobj_{key[2:]}(object[] a) => "
            f"(global::{owner})System.Activator.CreateInstance(typeof(global::{owner}), a);"
            for key, owner in sorted(shells.items()))
        carrier = ("namespace NewObjectCases\n{\npublic static class Recovered\n{\n" + declared + "\n"
                   + "\n".join(snippets) + "\n}\n}\n")
        source = OUT / f"Recovered-{config}{'-readable' if args.readable else '-details'}.cs"
        support.write_changed(source, carrier)
        recovered = source.with_suffix(".dll")
        compile_ms += support.compile_cs(recovered, [source], references=[fixture])
        result = support.run([support.DOTNET / "dotnet", harness, fixture, "--newobj-check", recovered])
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip()})
        if args.mutation_control:
            # Reusing one instance for every call must be detected.
            bad = carrier
            for key, owner in shells.items():
                marker = f"static global::{owner} __cil_newobj_{key[2:]}(object[] a) => (global::{owner})"
                if marker not in bad: raise ValueError("allocation mutation target missing: " + key)
                bad = bad.replace(marker,
                    f"static global::{owner} __cache_{key[2:]};\n"
                    f"static global::{owner} __cil_newobj_{key[2:]}(object[] a) => __cache_{key[2:]} ??= (global::{owner})", 1)
            path = OUT / f"CachedAllocation-{config}.cs"
            support.write_changed(path, bad)
            binary = path.with_suffix(".dll")
            compile_ms += support.compile_cs(binary, [path], references=[fixture])
            negative = subprocess.run([str(support.DOTNET / "dotnet"), str(harness), str(fixture), "--newobj-check", str(binary)],
                                      text=True, capture_output=True, timeout=120)
            (OUT / f"CachedAllocation-mutation-{config}.log").write_text(negative.stdout + negative.stderr)
            if negative.returncode == 0: raise ValueError("reused identity was not detected")
            print(config + ": rejected cached-identity mutant", flush=True)
    view = "readable" if args.readable else "details"
    (OUT / f"behavior-results-{view}.json").write_text(json.dumps({"configurations": rows, "compile_ms": compile_ms,
        "view": view, "mutation_control": args.mutation_control, "third_party_execution": False}, indent=2) + "\n")
    print(f"Owned compilation: {compile_ms} ms; Rust compilation: none")

if __name__ == "__main__":
    main()
