#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute recovered MethodSpec call sites and recovered owned support bodies."""
import argparse
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/method-instantiations/behavior"


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
    support.run([sys.executable, ROOT / "tests/build-method-instantiation-fixtures.py"])
    sources = [ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"] + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs"))
    harness = OUT / "MethodInstantiationRoundtrip.dll"
    support.compile_cs(harness, sources, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ["release", "debug"]:
        fixture = ROOT / f"tests/fixtures/method-instantiation-{config}.bin"
        admitted = {row["token"] for row in map(json.loads, support.run([args.cli, "scan", fixture]).stdout.splitlines())
                    if row["status"] == "Decompiled"}
        selected = []
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            found = re.search(r"MethodInstantiationCases\.(Helpers|Store|Derived|Caller|Ops)(?:`1)?::([^\s(<]+)(?:<\d+>)?\(", signature)
            if found and token in admitted and found[2] != ".ctor":
                selected.append((token, found[1], found[2]))
        if not selected: raise ValueError("Owned MethodSpec selection is empty")
        methods = {}
        for token, family, name in selected:
            text = support.run([args.cli, "decompile", fixture, "--method", token, "--no-auto-references"]).stdout
            (OUT / f"{config}-{family}-{name}.readable.txt").write_text(text)
            method_parameters = re.search(r"\b" + re.escape(name) + r"<([^>]+)>\(", text.splitlines()[0])
            method_parameters = method_parameters[1].split(",") if method_parameters else []
            def variables(text):
                text = re.sub(r"!!(\d+)", lambda m: method_parameters[int(m[1])].strip(), text)
                return re.sub(r"(?<!!)!(\d+)", lambda m: ["T"][int(m[1])], text)
            text = generic.inline_structural_types(variables(generic.without_dependency_report(text)))
            il = support.run([args.cli, "il", fixture, "--method", token]).stdout
            targets = {}
            for kind, target, owner, member, arguments in re.findall(
                r"(call|callvirt) 0x([0-9A-F]{8}) (.+)::([^\s<]+)<([^\n]+)>", il):
                owner = generic.metadata_to_csharp(variables(owner))
                arguments = generic.metadata_to_csharp("Args<" + variables(arguments) + ">")[5:-1]
                targets[target] = (kind, owner, member, arguments)
            def call(match):
                target, arguments = match.groups()
                kind, owner, member, type_arguments = targets[target]
                parts = [p.strip() for p in arguments.split(",") if p.strip()]
                if "::" in owner: raise ValueError("Unparsed owner: " + owner)
                if owner.endswith(".Helpers") or owner in ("System.Array", "System.Activator"):
                    return owner + "." + member + "<" + type_arguments + ">(" + ", ".join(parts) + ")"
                receiver = parts.pop(0)
                return "((" + owner + ")" + receiver + ")." + member + "<" + type_arguments + ">(" + ", ".join(parts) + ")"
            text = re.sub(r"__cil_(?:methodspec_call|callvirt)_([0-9A-F]{8})\(([^()]*)\)", call, text)
            for family_name in ["Helpers", "Store", "Derived", "Caller", "Ops"]:
                text = re.sub(r"\bMethodInstantiationCases\." + family_name + r"\b",
                              "MethodInstantiationCases.Recovered" + family_name, text)
            text = re.sub(r'__cil_ldstr_[0-9A-F]{8}\(("(?:[^"\\]|\\.)*")\)', r"\1", text)
            code = "\n".join(line for line in text.splitlines() if not line.strip().startswith("//"))
            if "__cil_" in code: raise ValueError("Unbound recovered MethodSpec body: " + family + "." + name)
            methods[(family, name)] = text
        def compile_carrier(label, bodies):
            source = OUT / f"{label}-{config}.cs"
            declarations = []
            for family, declaration, fields in [
                ("Helpers", "public static class RecoveredHelpers", "public static int Trace;"),
                ("Store", "public class RecoveredStore<T>", "public T Value;"),
                ("Derived", "public class RecoveredDerived<T> : RecoveredStore<T>", "public int Calls;"),
                ("Caller", "public class RecoveredCaller<T>", ""),
                ("Ops", "public static class RecoveredOps", ""),
            ]:
                declarations.append(declaration + " { " + fields + "\n"
                    + "\n".join(text for (owner, _), text in bodies.items() if owner == family) + "\n}")
            support.write_changed(source, "namespace MethodInstantiationCases {\n" + "\n".join(declarations) + "\n}\n")
            binary = source.with_suffix(".dll")
            support.compile_cs(binary, [source], references=[fixture])
            return binary
        binary = compile_carrier("Recovered", methods)
        command = [support.DOTNET / "dotnet", harness, fixture, "--method-instantiation-check", binary]
        result = support.run(command)
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip(), "methods": [list(k) for k in methods]})
        if args.mutation_control:
            for name in ["Int", "Owner", "ExternalCreate", "NonvirtualOrder"]:
                key = ("Ops", name)
                if key not in methods: raise ValueError("Mutation method missing: " + name)
                if name == "Int":
                    bad, count = re.subn(r"\.Identity<int>\([^()]*\)", ".Identity<int>(0)", methods[key], count=1)
                elif name == "NonvirtualOrder":
                    bad, count = re.subn(r"MethodInstantiationCases\.RecoveredHelpers\.Argument\([^()]*\)", "1", methods[key], count=1)
                elif name == "Owner":
                    bad, count = re.subn(r"\.OwnerEcho<int>\([^()]*\)", ".OwnerEcho<int>(null, 0)", methods[key], count=1)
                else:
                    bad, count = re.subn(r"CreateInstance<[^>]+>", "CreateInstance<object>", methods[key], count=1)
                    bad = bad.replace("global::MethodInstantiationCases.NoDefault", "object").replace("MethodInstantiationCases.NoDefault", "object")
                if count != 1: raise ValueError("Mutation target missing: " + name)
                mutated = dict(methods); mutated[key] = bad
                binary = compile_carrier("Wrong-" + name, mutated)
                negative = subprocess.run([str(x) for x in [*command[:4], binary]], text=True,
                                          capture_output=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Wrong-{name}-{config}.log").write_text(log)
                if negative.returncode == 0 or "Recovered MethodSpec differs:" not in log:
                    raise ValueError("Mutation not detected: " + name + "\n" + log)
                print(config + ": rejected " + name + " mutant", flush=True)
                rows.append({"configuration": config, "mutation": name, "detected": True})
    (OUT / "results.json").write_text(json.dumps({"rows": rows, "third_party_execution": False}, indent=2) + "\n")


if __name__ == "__main__":
    main()
