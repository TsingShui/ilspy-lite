#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute reconstructed generic bodies and their recovered support methods."""
import argparse
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/generic-binding/behavior"


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


support = module("object_roundtrip", ROOT / "tests/object-roundtrip.py")
generic = module("generic_declaring", ROOT / "tests/generic-declaring-roundtrip.py")
FIELDS = {"Bag": "public T Value;", "Proxy": "public RecoveredBag<T> Target;",
          "Pair": "public T First; public U Second;", "Swapped": "public RecoveredPair<U,T> Target;"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--mutation-control", action="store_true")
    args = parser.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    support.run([sys.executable, ROOT / "tests/build-generic-binding-fixtures.py"])
    harness = OUT / "GenericBindingRoundtrip.dll"
    sources = [ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"] + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs"))
    support.compile_cs(harness, sources, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ["release", "debug"]:
        fixture = ROOT / f"tests/fixtures/generic-binding-{config}.bin"
        selected = []
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            found = re.search(r"GenericBindingCases\.(Bag|Proxy|Pair|Swapped)`[12]::([^\s(]+)\(", signature)
            if found and found[2] != ".ctor":
                selected.append((token, found[1], found[2]))
        if not selected: raise ValueError("Owned generic-binding selection is empty")
        methods = {}
        for token, family, name in selected:
            text = support.run([args.cli, "decompile", fixture, "--method", token, "--no-auto-references"]).stdout
            (OUT / f"{config}-{family}-{name}.readable.txt").write_text(text)
            # Generic positions are rebound only within this owned type declaration.
            text = re.sub(r"!(\d+)", lambda m: ["T", "U"][int(m[1])], text)
            text = generic.inline_structural_types(generic.without_dependency_report(text))
            il = support.run([args.cli, "il", fixture, "--method", token]).stdout
            targets = {}
            for kind, target, owner, member in re.findall(
                r"(call|callvirt) 0x([0-9A-F]{8}) class (GenericBindingCases\.\S+)::([^\s]+)", il):
                owner = generic.metadata_to_csharp("class " + re.sub(
                    r"!(\d+)", lambda m: ["T", "U"][int(m[1])], owner))
                targets[target] = (kind, owner, member)
            def call(match):
                operation, target, arguments = match.groups()
                kind, owner, member = targets[target]
                parts = [p.strip() for p in arguments.split(",") if p.strip()]
                if operation == "static_call":
                    return owner + "." + member + "(" + ", ".join(parts) + ")"
                if kind == "callvirt": raise ValueError("Unexpected virtual leaf in this owned fixture")
                return parts[0] + "." + member + "(" + ", ".join(parts[1:]) + ")"
            text = re.sub(r"__cil_(direct|static_call)_([0-9A-F]{8})\(([^()]*)\)", call, text)
            for owner in FIELDS:
                text = re.sub(r"\bGenericBindingCases\." + owner + r"\b", "GenericBindingCases.Recovered" + owner, text)
            if name == ".cctor": text = text.replace("static Bag()", "static RecoveredBag()")
            code = "\n".join(line for line in text.splitlines() if not line.strip().startswith("//"))
            if "__cil_" in support.without_boolean_helpers(code): raise ValueError("Unbound recovered body: " + family + "." + name)
            methods[(family, name)] = text
        def compile_carrier(label, bodies):
            source = OUT / f"{label}-{config}.cs"
            declarations = []
            for family, fields in FIELDS.items():
                arguments = "<T,U>" if family in ("Pair", "Swapped") else "<T>"
                declarations.append("public class Recovered" + family + arguments + " { " + fields + "\n"
                    + "\n".join(text for (owner, _), text in bodies.items() if owner == family) + "\n}")
            support.write_changed(source, "namespace GenericBindingCases {\n" + "\n".join(declarations) + "\n}\n")
            binary = source.with_suffix(".dll")
            support.compile_cs(binary, [source], references=[fixture])
            return binary
        recovered = compile_carrier("Recovered", methods)
        command = [support.DOTNET / "dotnet", harness, fixture, "--generic-binding-check", recovered]
        result = support.run(command)
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip(), "methods": [list(k) for k in methods]})
        if args.mutation_control:
            for family, name in [("Bag", "ForwardSet"), ("Bag", "ForwardGet"), ("Proxy", "Exchange"), ("Bag", ".cctor")]:
                key = (family, name)
                if key not in methods: raise ValueError("Mutation target missing: " + str(key))
                if name == "ForwardSet":
                    bad, count = re.subn(r"\.Set\([^()]*\)", ".Set(default(T))", methods[key], count=1)
                elif name == ".cctor":
                    bad, count = re.subn(r"(GenericBindingCases\.Tracker\.Count) = [^;]+;", r"\1 = 0;", methods[key], count=1)
                else:
                    bad, count = re.subn(r"return [^;]+;", "return default(T);", methods[key], count=1)
                if count != 1: raise ValueError("Mutation expression missing: " + str(key))
                mutated = dict(methods); mutated[key] = bad
                binary = compile_carrier("Wrong-" + family + "-" + name, mutated)
                negative = subprocess.run([str(x) for x in [*command[:4], binary]], text=True,
                                          capture_output=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Wrong-{family}-{name}-{config}.log").write_text(log)
                if negative.returncode == 0 or "Recovered generic binding differs:" not in log:
                    raise ValueError("Semantic mutant not detected: " + str(key) + "\n" + log)
                print(config + ": rejected " + family + "." + name + " mutant", flush=True)
                rows.append({"configuration": config, "mutation": list(key), "detected": True})
    (OUT / "results.json").write_text(json.dumps({"rows": rows, "third_party_execution": False}, indent=2) + "\n")


if __name__ == "__main__":
    main()
