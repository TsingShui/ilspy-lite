#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute abstract constructed-owner calls recovered by the production CLI."""
import argparse
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/abstract-members/behavior"


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
    support.run([sys.executable, ROOT / "tests/build-abstract-member-fixtures.py"])
    sources = [ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"] + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs"))
    harness = OUT / "AbstractMemberRoundtrip.dll"
    support.compile_cs(harness, sources, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ["release", "debug"]:
        fixture = ROOT / f"tests/fixtures/abstract-member-{config}.bin"
        names = {}
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, sig = line.split(" ", 1)
            match = re.search(r"AbstractMemberCases\.Ops::([^\s(]+)\(", sig)
            if match: names[token] = match[1]
        methods = {}
        for row in map(json.loads, support.run([args.cli, "scan", fixture]).stdout.splitlines()):
            if row["status"] != "Decompiled" or row["token"] not in names: continue
            name = names[row["token"]]
            readable = support.run([args.cli, "decompile", fixture, "--method", row["token"], "--no-auto-references"])
            body = generic.inline_allocation(generic.inline_structural_types(
                generic.without_dependency_report(readable.stdout)))
            code = "\n".join(line for line in body.splitlines() if not line.strip().startswith("//"))
            if "__cil_" in support.without_boolean_helpers(code): raise ValueError("Unexecutable selected method: " + name)
            methods[name] = body
            (OUT / f"{config}-{name}.cs").write_text(body)
        if not methods: raise ValueError("Owned abstract-member selection is empty")
        def compile_carrier(label, snippets):
            source = OUT / f"{label}-{config}.cs"
            support.write_changed(source, "namespace AbstractMemberCases { public static class Recovered {\n"
                + "\n".join(snippets.values()) + "\n} }\n")
            output = source.with_suffix(".dll")
            support.compile_cs(output, [source], references=[fixture])
            return output
        binary = compile_carrier("Recovered", methods)
        command = [support.DOTNET / "dotnet", harness, fixture, "--abstract-member-check", binary]
        result = support.run(command)
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip(), "methods": list(methods)})
        if args.mutation_control:
            for name in ["Virtual", "Twice"]:
                if name not in methods: raise ValueError("Mutation target missing: " + name)
                bad, count = re.subn(r"\.Compute\\?\([^()]*\)", ".Fixed()", methods[name], count=1)
                if count != 1: raise ValueError("Mutation target expression missing: " + name)
                mutated = dict(methods); mutated[name] = bad
                binary = compile_carrier("WrongDispatch-" + name, mutated)
                run = subprocess.run([str(x) for x in [*command[:4], binary]],
                                     capture_output=True, text=True, timeout=120)
                log = run.stdout + run.stderr
                (OUT / f"WrongDispatch-{name}-{config}.log").write_text(log)
                if run.returncode == 0 or "Recovered abstract member differs: " + name not in log:
                    raise ValueError("Mutation was not detected: " + name + "\n" + log)
                rows.append({"configuration": config, "mutation": name, "detected": True})
                print(config + ": rejected " + name + " mutant", flush=True)
    (OUT / "results.json").write_text(json.dumps({"rows": rows, "third_party_execution": False}, indent=2)+"\n")


if __name__ == "__main__":
    main()
