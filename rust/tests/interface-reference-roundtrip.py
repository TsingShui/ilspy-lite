#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute local interface reference calls recovered by the production CLI."""
import argparse
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/reference-checks/interface-behavior"


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


support = module("object_roundtrip", ROOT / "tests/object-roundtrip.py")
generic = module("generic_declaring", ROOT / "tests/generic-declaring-roundtrip.py")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--mutation-control", action="store_true")
    args = parser.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    support.run([sys.executable, ROOT / "tests/build-interface-reference-fixtures.py"])
    sources = [ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"] + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs"))
    harness = OUT / "InterfaceReferenceRoundtrip.dll"
    support.compile_cs(harness, sources, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ["release", "debug"]:
        fixture = ROOT / f"tests/fixtures/interface-reference-{config}.bin"
        names = {}
        targets = {}
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, sig = line.split(" ", 1)
            target = re.search(r"(InterfaceReferenceCases\.\w+)::([^\s(]+)\(", sig)
            if target: targets[token[2:]] = (target.group(1), target.group(2))
            match = re.search(r"InterfaceReferenceCases\.Ops::([^\s(]+)\(", sig)
            if match: names[token] = match[1]
        methods = {}
        for row in map(json.loads, support.run([args.cli, "scan", fixture]).stdout.splitlines()):
            if row["status"] != "Decompiled" or row["token"] not in names: continue
            name = names[row["token"]]
            readable = support.run([args.cli, "decompile", fixture, "--method", row["token"], "--no-auto-references"])
            body = generic.inline_structural_types(generic.without_dependency_report(readable.stdout))
            def allocate(match):
                owner, member = targets[match.group(1)]
                if member != ".ctor": raise ValueError("Allocation target is not a constructor")
                return f"new {owner}({match.group(2)})"
            body = re.sub(r"__cil_newobj_([0-9A-F]{8})\(([^()]*)\)", allocate, body)
            def dispatch(match):
                owner, member = targets[match.group(1)]
                arguments = [part.strip() for part in match.group(2).split(",")]
                tail = ', '.join(arguments[1:])
                return f"(({owner}){arguments[0]}).{member}({tail})"
            body = re.sub(r"__cil_callvirt_([0-9A-F]{8})\(([^()]*)\)", dispatch, body)
            code = "\n".join(line for line in body.splitlines() if not line.strip().startswith("//"))
            if "__cil_" in support.without_boolean_helpers(code): raise ValueError("Unexecutable selected method: " + name)
            methods[name] = body
            (OUT / f"{config}-{name}.cs").write_text(body)
        if not methods: raise ValueError("Owned interface-reference selection is empty")
        def compile_carrier(label, snippets):
            source = OUT / f"{label}-{config}.cs"
            support.write_changed(source, "namespace InterfaceReferenceCases { public static class Recovered {\n"
                + "\n".join(snippets.values()) + "\n} }\n")
            output = source.with_suffix(".dll")
            support.compile_cs(output, [source], references=[fixture])
            return output
        binary = compile_carrier("Recovered", methods)
        command = [support.DOTNET / "dotnet", harness, fixture, "--interface-reference-check", binary]
        result = support.run(command)
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip(), "methods": list(methods)})
        if args.mutation_control:
            for name in ["Invoke", "Twice", "ArgumentOrder", "As"]:
                if name not in methods: raise ValueError("Mutation target missing: " + name)
                if name == "As":
                    bad, count = re.subn(r"\(\(object\)(\w+) as ([A-Za-z0-9_@:.]+)\)",
                                         r"((\2)(object)\1)", methods[name], count=1)
                elif name == "ArgumentOrder":
                    bad, count = re.subn(r"\(\(InterfaceReferenceCases.Counter\)\w+\)\.Record\([^()]*\)",
                                         "1", methods[name], count=1)
                else:
                    bad, count = re.subn(r"\(\(InterfaceReferenceCases.IValue\)\w+\)\.Compute\([^()]*\)",
                                         "0", methods[name], count=1)
                if count != 1: raise ValueError("Mutation target expression missing: " + name)
                mutated = dict(methods); mutated[name] = bad
                binary = compile_carrier("Wrong-" + name, mutated)
                run = subprocess.run([str(x) for x in [*command[:4], binary]],
                                     capture_output=True, text=True, timeout=120)
                log = run.stdout + run.stderr
                (OUT / f"Wrong-{name}-{config}.log").write_text(log)
                if run.returncode == 0 or "Recovered interface differs: " + name not in log:
                    raise ValueError("Mutation was not detected: " + name + "\n" + log)
                rows.append({"configuration": config, "mutation": name, "detected": True})
                print(config + ": rejected " + name + " mutant", flush=True)
    (OUT / "results.json").write_text(json.dumps({"rows": rows, "third_party_execution": False}, indent=2)+"\n")


if __name__ == "__main__":
    main()
