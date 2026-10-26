#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute recovered reference relationships without adding runtime conversions."""
import argparse
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/reference-relations/behavior"


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
    support.run([sys.executable, ROOT / "tests/build-reference-relation-fixtures.py"])
    harness = OUT / "ReferenceRelationRoundtrip.dll"
    support.compile_cs(harness, [ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"]
                       + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs")), executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    results = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/reference-relation-{config}.bin"
        library = ROOT / f"tests/fixtures/relation-library-{config}.bin"
        names = {}
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            match = re.search(r"(RelationCases\.(?:Ops|Cell\S*))::([^\s(<]+)", signature)
            if match and not match[2].startswith("."): names[token] = (match[1], match[2])
        methods = []
        for row in map(json.loads, support.run([args.cli, "scan", fixture, "--no-auto-references", "--reference", library]).stdout.splitlines()):
            if row["token"] not in names: continue
            owner, name = names[row["token"]]
            if row["status"] != "Decompiled": raise ValueError("Owned relation not recovered: " + owner + "." + name)
            text = support.run([args.cli, "decompile", fixture, "--method", row["token"], "--no-auto-references", "--reference", library]).stdout
            (OUT / f"{config}-{row['token']}.readable.txt").write_text(text)
            text = generic.inline_structural_types(text)
            parameters = ["A", "B"] if name in ["Swap", "SwappedInterface"] else ["T"]
            for position, parameter in enumerate(parameters):
                text = text.replace("!!" + str(position), parameter).replace("!" + str(position), parameter)
            text = re.sub(r"\b(?:global::)?@?RelationCases\.@?Ops\.", "RelationCases.Recovered.", text)
            if name == "ReadCast":
                text = "public class RecoveredCell<T> {\n" + text + "\n}\n"
            executable = "\n".join(line for line in text.splitlines() if not line.strip().startswith("//"))
            if "__cil_" in support.without_boolean_helpers(executable): raise ValueError("Unbound relation body: " + name)
            methods.append(text)
        if not methods: raise ValueError("Empty relation selection")
        if not any("class RecoveredCell<T>" in method for method in methods): raise ValueError("Owned self-cast selection missing")
        def compile_carrier(label, bodies):
            source = OUT / f"{label}-{config}.cs"
            support.write_changed(source, "namespace RelationCases { public static class Recovered {\n"
                                  + "\n".join(bodies) + "\n} }\n")
            binary = source.with_suffix(".dll")
            support.compile_cs(binary, [source], references=[fixture, library])
            return binary
        binary = compile_carrier("Recovered", methods)
        command = [support.DOTNET / "dotnet", harness, fixture, "--reference-relation-check", binary, library]
        result = support.run(command)
        print(config + ": " + result.stdout.strip(), flush=True)
        results.append({"configuration": config, "behavior": result.stdout.strip()})
        if args.mutation_control:
            for name, replacement in [("DeclaredOverload", "return input == null ? 29 : 29;"),
                                       ("Swap", "return null;"), ("Foreign", "return null;"),
                                       ("ReadCast", "return default;")]:
                selected = [i for i, text in enumerate(methods) if re.search(r"\b" + name + r"(?:<[^>]+>)?\(", text)]
                if len(selected) != 1: raise ValueError("Mutation target differs: " + name)
                changed = list(methods)
                if name == "DeclaredOverload":
                    changed[selected[0]], count = re.subn(r"\(\(RelationCases\.Base<string>\)\(input\)\)", "input", changed[selected[0]], count=1)
                else:
                    changed[selected[0]], count = re.subn(r"return [^;]+;", replacement, changed[selected[0]], count=1)
                if count != 1: raise ValueError("Mutation return missing")
                mutant = compile_carrier("Wrong-" + name, changed)
                negative = subprocess.run([str(x) for x in [*command[:4], mutant, library]], text=True, capture_output=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Wrong-{name}-{config}.log").write_text(log)
                if negative.returncode == 0 or "Recovered reference relation differs:" not in log:
                    raise ValueError("Relation semantic mutant survived: " + name + log)
                print(config + ": rejected " + name + " mutant", flush=True)
                results.append({"configuration": config, "mutation": name, "detected": True})
    (OUT / "results.json").write_text(json.dumps({"rows": results, "third_party_execution": False}, indent=2) + "\n")


if __name__ == "__main__":
    main()
