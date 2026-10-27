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
    args = parser.parse_args()
    if not args.cli.is_file():
        parser.error("Existing Rust CLI required")
    OUT.mkdir(parents=True, exist_ok=True)
    support.run([sys.executable, ROOT / "tests/build-exception-region-fixtures.py"])
    harness = OUT / "ExceptionRoundtrip.dll"
    support.compile_cs(harness, SOURCES, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/exception-region-{config}.bin"
        names = {}
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            found = re.search(r"ExceptionRegionCases\.Ops::([^\s(]+)\(", signature)
            if found:
                names[token] = found.group(1)
        records = list(map(json.loads, support.run([args.cli, "scan", fixture]).stdout.splitlines()))
        selected = [(r["token"], names[r["token"]]) for r in records
                    if r["status"] == "Decompiled" and r["token"] in names]
        if not selected:
            raise ValueError("Exception-region selection is empty")
        snippets = {}
        for token, name in selected:
            # Compile exactly the default readable output. No handler executor,
            # exception substitute, no-op cleanup or original-method proxy is added.
            readable = support.run([args.cli, "decompile", fixture, "--method", token, "--no-auto-references"])
            if readable.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0":
                raise ValueError("Readable admission drift: " + name)
            code = "\n".join(line for line in readable.stdout.splitlines()
                             if not line.startswith("// Dependency"))
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
            (OUT / f"{config}-{name}.readable.cs").write_text(code)

        def carrier(methods):
            return "namespace ExceptionRegionCases { public static class Recovered {\n" + "\n".join(methods.values()) + "\n} }\n"

        source = OUT / f"Recovered-{config}.cs"
        support.write_changed(source, carrier(snippets))
        recovered = source.with_suffix(".dll")
        support.compile_cs(recovered, [source], references=[fixture])
        command = [support.DOTNET / "dotnet", harness, fixture, "--exception-check", recovered]
        result = support.run(command)
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip(),
                     "selected": [name for _, name in selected]})
        if args.mutation_control:
            for name in ["FinallyOrder", "Catch", "Rethrow", "RethrowIdentity", "GeneralCatch", "OutOfMemory"]:
                if name not in snippets:
                    raise ValueError("Required behavior mutation not selected: " + name)
                modified = dict(snippets)
                modified[name] = mutate_body(modified[name], name)
                path = OUT / f"Mutant-{name}-{config}.cs"
                support.write_changed(path, carrier(modified))
                binary = path.with_suffix(".dll")
                support.compile_cs(binary, [path], references=[fixture])
                mutated = subprocess.run([str(support.DOTNET / "dotnet"), str(harness), str(fixture),
                                          "--exception-check", str(binary)],
                                         text=True, capture_output=True, timeout=120)
                log = mutated.stdout + mutated.stderr
                (OUT / f"Mutant-{name}-{config}.log").write_text(log)
                marker = "Recovered exception behavior differs: " + name
                if mutated.returncode == 0 or marker not in log:
                    raise ValueError("Mutation was not rejected for its semantic difference: " + name + "\n" + log)
                print(config + ": rejected " + name + " mutant", flush=True)
                rows.append({"configuration": config, "mutation": name, "detected": marker})
    (OUT / "behavior-results.json").write_text(json.dumps({
        "rows": rows, "executed_recovered_bodies": True, "third_party_execution": False,
        "mutation_control": args.mutation_control,
    }, indent=2) + "\n")


if __name__ == "__main__":
    main()
