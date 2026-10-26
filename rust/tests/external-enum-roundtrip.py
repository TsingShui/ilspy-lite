#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute recovered external-enum storage; widths, loads and stores must match."""
import argparse, importlib.util, json, re, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/external-enum-roundtrip"
SOURCES = ([ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"]
           + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs")))


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec); spec.loader.exec_module(result); return result

support = module("object_roundtrip", ROOT / "tests/object-roundtrip.py")
generic = module("generic_declaring", ROOT / "tests/generic-declaring-roundtrip.py")


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
    print(support.run([sys.executable, ROOT / "tests/build-external-enum-fixtures.py"]).stdout, end="", flush=True)
    harness = OUT / "ExternalEnumRoundtrip.dll"
    compile_ms = support.compile_cs(harness, SOURCES, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/external-enum-{config}.bin"
        library = ROOT / f"tests/fixtures/external-enum-library-{config}.bin"
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
                    if record["status"] == "Decompiled" and owners.get(record["token"]) == "ExternalEnumCases.Ops"]
        if not selected: raise ValueError("external-enum selection is empty")
        snippets = []
        for token, name in selected:
            details = support.run([args.cli, "decompile", fixture, "--method", token, "--details",
                                   "--no-auto-references", "--reference", library])
            if details.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0":
                raise ValueError("external-enum method not recovered: " + name)
            body = without_dependency_report(details.stdout)
            body = generic.inline_structural_types(body)
            for alias, declared in re.findall(r"external class token \S+: (\w+) = \S+ \[[^\]]+\](\S+) //", body):
                body = body.replace(alias, "global::" + generic.metadata_to_csharp(declared))
            (OUT / f"{config}-{name}.details.txt").write_text(details.stdout)
            code = "\n".join(line for line in body.splitlines()
                              if not line.lstrip().startswith("//"))
            if "__cil_" in support.without_boolean_helpers(code):
                raise ValueError("Unbound recovered external-enum method: " + name)
            snippets.append(body)
        if not snippets: raise ValueError("external-enum carrier is empty")
        carrier = ("namespace ExternalEnumCases\n{\npublic static class Recovered\n{\n"
                   + "\n".join(snippets) + "\n}\n}\n")
        source = OUT / f"Recovered-{config}.cs"
        support.write_changed(source, carrier)
        recovered = source.with_suffix(".dll")
        compile_ms += support.compile_cs(recovered, [source], references=[fixture, library])
        result = support.run([support.DOTNET / "dotnet", harness, fixture,
                              "--external-enum-check", recovered, library])
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip(), "skipped": []})
        if args.mutation_control:
            # Changing the load extension from the declared unsigned width to a
            # signed one must change a value whose high bit is set, because this
            # method's result escapes as a 64-bit integer instead of being
            # narrowed again at the return.
            anchor = carrier.index("@Widen(")
            head, tail = carrier[:anchor], carrier[anchor:]
            tail, count = re.subn(r"unchecked\(\(byte\)\(",
                                  "unchecked((sbyte)(", tail, count=1)
            if count != 1: raise ValueError("external-enum mutation target missing")
            bad = head + tail
            path = OUT / f"WidenedStorage-{config}.cs"
            support.write_changed(path, bad)
            binary = path.with_suffix(".dll")
            compile_ms += support.compile_cs(binary, [path], references=[fixture, library])
            mutated = subprocess.run([str(support.DOTNET / "dotnet"), str(harness), str(fixture),
                                      "--external-enum-check", str(binary), str(library)],
                                     text=True, capture_output=True, timeout=300)
            (OUT / f"SignedLoad-mutation-{config}.log").write_text(mutated.stdout + mutated.stderr)
            if mutated.returncode == 0 or "Recovered external-enum differs:" not in mutated.stderr:
                raise ValueError("Signed-load semantic mutant was not detected: " + mutated.stdout + mutated.stderr)
            rows.append({"configuration": config, "mutation": "signed-load-extension",
                         "detected": mutated.stderr.strip().splitlines()[0] if mutated.stderr.strip() else "nonzero exit"})
            print(config + " mutation: signed load extension rejected", flush=True)
            for name, replacement in [
                ("UnsupportedStruct", "return default(global::ExternalEnums.Point);"),
                ("UnsupportedClass", "return a0 == null ? null : new global::ExternalEnums.Widget { Value = a0.Value };"),
            ]:
                pattern = r"(public static [^\n]+ @" + name + r"\([^\n]+\)\n\{)(.*?)(\n\})"
                target = re.search(pattern, carrier, re.S)
                if target is None: raise ValueError("Copy/identity mutation target missing: " + name)
                body, count = re.subn(r"return [^;]+;", replacement, target[2], count=1)
                if count != 1: raise ValueError("Copy/identity return missing: " + name)
                bad = carrier[:target.start(2)] + body + carrier[target.end(2):]
                path = OUT / f"Wrong-{name}-{config}.cs"
                support.write_changed(path, bad)
                binary = path.with_suffix(".dll")
                compile_ms += support.compile_cs(binary, [path], references=[fixture, library])
                negative = subprocess.run([str(support.DOTNET / "dotnet"), str(harness), str(fixture),
                    "--external-enum-check", str(binary), str(library)], text=True, capture_output=True, timeout=300)
                (OUT / f"Wrong-{name}-{config}.log").write_text(negative.stdout + negative.stderr)
                if negative.returncode == 0 or "Recovered external-enum differs:" not in negative.stderr:
                    raise ValueError("Copy/identity mutant was not detected: " + name + negative.stdout + negative.stderr)
                print(config + ": rejected " + name + " mutant", flush=True)
                rows.append({"configuration": config, "mutation": name, "detected": True})
    (OUT / "behavior-results.json").write_text(json.dumps(
        {"configurations": rows, "compile_ms": compile_ms,
         "mutation_control": args.mutation_control, "third_party_execution": False}, indent=2) + "\n")
    print(json.dumps({"compile_ms": compile_ms, "rows": rows}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
