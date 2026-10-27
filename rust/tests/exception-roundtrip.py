#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute default readable EH bodies against owned originals, with mutation controls."""
import argparse
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/exception-regions/behavior"


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


support = module("object_roundtrip", ROOT / "tests/object-roundtrip.py")
SOURCES = [ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"] + sorted(
    (ROOT / "tests/dotnet").glob("*Behavior.cs")
)


def mutate_body(body, name):
    if name == "FinallyOrder":
        prefix, final = body.split("finally {", 1)
        final, count = re.subn(r"(ExceptionRegionCases\.Ops\.Trace) = [^;]+;", r"\1 = 0;", final, count=1)
        if count != 1:
            raise ValueError("finally store mutation target missing")
        return prefix + "finally {" + final
    if name in ("Catch", "GeneralCatch"):
        body, count = re.subn(r"= -1;", "= -9;", body, count=1)
    elif name == "OutOfMemory":
        body, count = re.subn(r"= -2;", "= -9;", body, count=1)
    elif name == "ForeignCatch":
        body, count = re.subn(r"global::RelationLibrary\.ForeignDerivedException", "global::System.InvalidOperationException", body)
        if count < 1:
            raise ValueError("foreign catch type mutation target missing")
        return body
    elif name == "ForeignThrow":
        body, count = re.subn(r"throw value;", "throw new global::RelationLibrary.ForeignException();", body, count=1)
    elif name == "Rethrow":
        body, count = re.subn(r"(?m)^(\s*)throw;", r"\1return 999;", body, count=1)
    elif name == "RethrowIdentity":
        body, count = re.subn(r"= __cil_exception_1;", "= new System.ArgumentOutOfRangeException(); _ = __cil_exception_1;", body, count=1)
    else:
        raise ValueError("unknown mutation: " + name)
    if count != 1:
        raise ValueError("mutation target missing: " + name)
    return body


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--mutation-control", action="store_true")
    parser.add_argument("--foreign", action="store_true", help="Exercise selected foreign exception definitions")
    args = parser.parse_args()
    if not args.cli.is_file():
        parser.error("Existing Rust CLI required")
    OUT.mkdir(parents=True, exist_ok=True)
    builder = "build-reference-relation-fixtures.py" if args.foreign else "build-exception-region-fixtures.py"
    support.run([sys.executable, ROOT / "tests" / builder])
    harness = OUT / "ExceptionRoundtrip.dll"
    support.compile_cs(harness, SOURCES, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/{'reference-relation' if args.foreign else 'exception-region'}-{config}.bin"
        library = ROOT / f"tests/fixtures/relation-library-{config}.bin" if args.foreign else None
        references = ["--no-auto-references"] + (["--reference", library] if library else [])
        names = {}
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            found = re.search(r"ExceptionRegionCases\.Ops::([^\s(]+)\(", signature)
            if found:
                names[token] = found.group(1)
        records = list(map(json.loads, support.run([args.cli, "scan", fixture, *references]).stdout.splitlines()))
        selected = [(r["token"], names[r["token"]]) for r in records
                    if r["status"] == "Decompiled" and r["token"] in names]
        if not selected:
            raise ValueError("Exception-region selection is empty")
        if args.foreign and {name for _, name in selected} != {"ForeignCatch", "ForeignThrow"}:
            raise ValueError("Foreign exception selection differs: " + str(selected))
        snippets = {}
        for token, name in selected:
            # Compile exactly the default readable output. No handler executor,
            # exception substitute, no-op cleanup or original-method proxy is added.
            readable = support.run([args.cli, "decompile", fixture, "--method", token, *references])
            if readable.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0":
                raise ValueError("Readable admission drift: " + name)
            code = "\n".join(line for line in readable.stdout.splitlines()
                             if not line.startswith("// Dependency"))
            if args.foreign and name == "ForeignCatch":
                il = support.run([args.cli, "il", fixture, "--method", token]).stdout
                catches = re.findall(r"catch 0x([0-9A-F]{8}) \[relation-library-[^]]+\](RelationLibrary\.[A-Za-z]+) ", il)
                if len(catches) != 2:
                    raise ValueError("Foreign catch identities differ")
                for type_token, type_name in catches:
                    code = code.replace("__cil_type_" + type_token, "global::" + type_name)
                if "__cil_type_" in code:
                    raise ValueError("Unbound foreign exception type")
            if name == "OutOfMemory":
                leaves = re.findall(r"__cil_newobj_([0-9A-F]{8})\(\)", code)
                il = support.run([args.cli, "il", fixture, "--method", token]).stdout
                if len(leaves) != 1 or not re.search(
                        rf"newobj 0x{leaves[0]} \[System\.Runtime\]System\.OutOfMemoryException::\.ctor", il):
                    raise ValueError("OutOfMemory constructor identity differs")
                code = code.replace(f"__cil_newobj_{leaves[0]}()", "new System.OutOfMemoryException()")
            forbidden = ("__cil_handler_exception(", "__cil_endfinally(", "__cil_endfilter(")
            if any(intrinsic in code for intrinsic in forbidden):
                raise ValueError("Unimplemented EH intrinsic in recovered output: " + name)
            snippets[name] = code
            (OUT / f"{'foreign-' if args.foreign else ''}{config}-{name}.readable.cs").write_text(code)

        def carrier(methods):
            return "namespace ExceptionRegionCases { public static class Recovered {\n" + "\n".join(methods.values()) + "\n} }\n"

        source = OUT / f"{'Foreign-' if args.foreign else ''}Recovered-{config}.cs"
        support.write_changed(source, carrier(snippets))
        recovered = source.with_suffix(".dll")
        fixture_references = [fixture] + ([library] if library else [])
        support.compile_cs(recovered, [source], references=fixture_references)
        command = [support.DOTNET / "dotnet", harness, fixture, "--exception-check", recovered] + ([library] if library else [])
        result = support.run(command)
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip(),
                     "selected": [name for _, name in selected]})
        if args.mutation_control:
            mutants = ["ForeignCatch", "ForeignThrow"] if args.foreign else ["FinallyOrder", "Catch", "Rethrow", "RethrowIdentity", "GeneralCatch", "OutOfMemory"]
            for name in mutants:
                if name not in snippets:
                    raise ValueError("Required behavior mutation not selected: " + name)
                modified = dict(snippets)
                modified[name] = mutate_body(modified[name], name)
                path = OUT / f"Mutant-{name}-{config}.cs"
                support.write_changed(path, carrier(modified))
                binary = path.with_suffix(".dll")
                support.compile_cs(binary, [path], references=fixture_references)
                mutated = subprocess.run([str(item) for item in [support.DOTNET / "dotnet", harness, fixture,
                                          "--exception-check", binary] + ([library] if library else [])],
                                         text=True, capture_output=True, timeout=120)
                log = mutated.stdout + mutated.stderr
                (OUT / f"Mutant-{name}-{config}.log").write_text(log)
                marker = "Recovered exception behavior differs: " + name
                if mutated.returncode == 0 or marker not in log:
                    raise ValueError("Mutation was not rejected for its semantic difference: " + name + "\n" + log)
                print(config + ": rejected " + name + " mutant", flush=True)
                rows.append({"configuration": config, "mutation": name, "detected": marker})
    (OUT / ("foreign-behavior-results.json" if args.foreign else "behavior-results.json")).write_text(json.dumps({
        "rows": rows, "executed_recovered_bodies": True, "third_party_execution": False,
        "mutation_control": args.mutation_control,
    }, indent=2) + "\n")


if __name__ == "__main__":
    main()
