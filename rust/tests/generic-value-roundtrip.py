#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute recovered generic-value storage; widths, loads and stores must match."""
import argparse, importlib.util, json, re, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/generic-value-roundtrip"
SOURCES = ([ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"]
           + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs")))


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec); spec.loader.exec_module(result); return result

support = module("object_roundtrip", ROOT / "tests/object-roundtrip.py")


def without_dependency_report(text):
    """The diagnostic view appends dependency provenance; it is evidence, not C#."""
    kept = [line for line in text.splitlines()
            if not line.startswith(".dependency") and not line.startswith("// Dependency")]
    return "\n".join(kept) + "\n"



PRIMITIVES = {"int8": "sbyte", "uint8": "byte", "int16": "short", "uint16": "ushort",
             "int32": "int", "uint32": "uint", "int64": "long", "uint64": "ulong",
             "bool": "bool", "char": "char", "float32": "float", "float64": "double",
             "string": "string", "object": "object", "void": "void"}


def metadata_to_csharp(description):
    """Render the evidence description of one structural signature as C#."""
    text = description.strip()
    if text.startswith("valuetype "):
        text = text[len("valuetype "):]
    elif text.startswith("class "):
        text = text[len("class "):]
    # A cross-assembly identity carries its scope; the scope is not part of C#.
    text = re.sub(r"\[[^\]]*\]", "", text).strip()
    if text in PRIMITIVES:
        return PRIMITIVES[text]
    open_bracket = text.find("<")
    if open_bracket < 0:
        return text.replace("`", "").replace("/", ".")
    head = text[:open_bracket]
    if "`" in head:
        head = head.split("`")[0]
    depth, start, arguments = 0, open_bracket + 1, []
    for index in range(open_bracket, len(text)):
        if text[index] == "<":
            depth += 1
        elif text[index] == ">":
            depth -= 1
            if depth == 0:
                arguments.append(text[start:index])
                break
        elif text[index] == "," and depth == 1:
            arguments.append(text[start:index])
            start = index + 1
    return head.replace("/", ".") + "<" + ", ".join(metadata_to_csharp(a) for a in arguments) + ">"


def inline_structural_types(body):
    """Replace structural aliases with the type their own evidence row describes."""
    names = {alias: metadata_to_csharp(description)
             for alias, description in re.findall(
                 r"structural signature (__cil_\w+) bytes=\S+: (.*?);", body)}
    for alias, name in names.items():
        body = body.replace(alias, name)
    return body


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--mutation-control", action="store_true")
    args = parser.parse_args()
    if not args.cli.is_file(): parser.error("Existing Rust CLI required")
    OUT.mkdir(parents=True, exist_ok=True)
    print(support.run([sys.executable, ROOT / "tests/build-generic-value-fixtures.py"]).stdout, end="", flush=True)
    harness = OUT / "GenericValueRoundtrip.dll"
    compile_ms = support.compile_cs(harness, SOURCES, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/generic-value-{config}.bin"
        names, owners = {}, {}
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            found = re.search(r"(\S+)::([^\s(]+)\(", signature)
            if found:
                owners[token] = found.group(1); names[token] = found.group(2)
        scan = support.run([args.cli, "scan", fixture])
        selected = [(record["token"], names[record["token"]])
                    for record in map(json.loads, scan.stdout.splitlines())
                    if record["status"] == "Decompiled" and owners.get(record["token"]) == "GenericValueCases.Ops"]
        if not selected: raise ValueError("generic-value selection is empty")
        snippets, skipped = [], []
        for token, name in selected:
            details = support.run([args.cli, "decompile", fixture, "--method", token, "--details"])
            if details.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0":
                raise ValueError("generic-value method not recovered: " + name)
            # The claim under test is the default readable output, so that is what
            # is compiled; the diagnostic view only has to agree on admission.
            readable = support.run([args.cli, "decompile", fixture, "--method", token])
            if readable.stderr != details.stderr:
                raise ValueError("view changed admission for " + name)
            body = inline_structural_types(without_dependency_report(readable.stdout))
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
        if not snippets: raise ValueError("generic-value carrier is empty")
        carrier = ("namespace GenericValueCases\n{\npublic static class Recovered\n{\n"
                   + "\n".join(snippets) + "\n}\n}\n")
        source = OUT / f"Recovered-{config}.cs"
        support.write_changed(source, carrier)
        recovered = source.with_suffix(".dll")
        compile_ms += support.compile_cs(recovered, [source], references=[fixture])
        result = support.run([support.DOTNET / "dotnet", harness, fixture,
                              "--generic-value-check", recovered])
        print(config + ": " + result.stdout.strip() + f" [skipped {skipped}]", flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip(),
                     "skipped": skipped})
        if args.mutation_control:
            # Returning a zeroed value instead of the copied one must change the
            # observed field contents, which is what makes the copy observable.
            if "    return pair;" not in carrier:
                raise ValueError("generic-value mutation target missing")
            bad = carrier.replace("    return pair;", "    return default;", 1)
            path = OUT / f"ZeroedCopy-{config}.cs"
            support.write_changed(path, bad)
            binary = path.with_suffix(".dll")
            compile_ms += support.compile_cs(binary, [path], references=[fixture])
            mutated = subprocess.run([str(support.DOTNET / "dotnet"), str(harness), str(fixture),
                                      "--generic-value-check", str(binary)],
                                     text=True, capture_output=True, timeout=300)
            (OUT / f"ZeroedCopy-mutation-{config}.log").write_text(mutated.stdout + mutated.stderr)
            if mutated.returncode == 0:
                raise ValueError("mutation control survived: zeroed copy accepted")
            rows.append({"configuration": config, "mutation": "zeroed-copy",
                         "detected": mutated.stderr.strip().splitlines()[0] if mutated.stderr.strip() else "nonzero exit"})
            print(config + " mutation: zeroed copy rejected", flush=True)
    (OUT / "behavior-results.json").write_text(json.dumps(
        {"configurations": rows, "compile_ms": compile_ms,
         "mutation_control": args.mutation_control, "third_party_execution": False}, indent=2) + "\n")
    print(json.dumps({"compile_ms": compile_ms, "rows": rows}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
