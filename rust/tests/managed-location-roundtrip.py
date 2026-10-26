#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute recovered byref bodies using actual typed shared-cell locations."""
import argparse
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/managed-locations/behavior"


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
    support.run([sys.executable, ROOT / "tests/build-managed-location-fixtures.py"])
    sources = [ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"] + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs"))
    harness = OUT / "ManagedLocationRoundtrip.dll"
    support.compile_cs(harness, sources, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    configurations = []
    for config in ["release", "debug"]:
        fixture = ROOT / f"tests/fixtures/managed-location-{config}.bin"
        configurations.append((config, fixture))
        variant = OUT / f"managed-location-{config}-readonly.bin"
        support.run([support.DOTNET / "dotnet", harness, fixture, "--managed-initonly-variant", variant])
        configurations.append((config + "-readonly", variant))
        bits = OUT / f"managed-location-{config}-bool-bits.bin"
        support.run([support.DOTNET / "dotnet", harness, fixture, "--managed-bool-variant", bits])
        configurations.append((config + "-bool-bits", bits))
    for config, fixture in configurations:
        names = {}
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            match = re.search(r"ManagedLocationCases\.Ops::([A-Za-z_][A-Za-z_0-9]*)(?:<\d+>)?\(", signature)
            if match: names[token] = match[1]
        methods = {}
        field_tokens = set()
        for row in map(json.loads, support.run([args.cli, "scan", fixture]).stdout.splitlines()):
            if row["status"] != "Decompiled" or row["token"] not in names: continue
            name = names[row["token"]]
            text = support.run([args.cli, "decompile", fixture, "--method", row["token"], "--no-auto-references"]).stdout
            (OUT / f"{config}-{name}.readable.txt").write_text(text)
            text = text.replace("!!0", "T")
            enum_names = re.findall(r"enum storage token=\S+ alias=(\w+) underlying=\S+: ([^;]+);", text)
            text = generic.inline_structural_types(text)
            for alias, declared in enum_names: text = text.replace(alias, declared)
            text = re.sub(r"\b(?:global::)?@?ManagedLocationCases\.@?Ops\.", "ManagedLocationCases.Recovered.", text)
            # ref/out share the same CLR ByRef ABI. Use ref carrier headers so a
            # ref local can represent an out pointer without inventing an eager
            # value initialization solely for C# definite-assignment rules.
            header, rest = text.split("\n", 1)
            text = header.replace("out ", "ref ") + "\n" + rest
            if name == "ScalarReceiver":
                il = support.run([args.cli, "il", fixture, "--method", row["token"]]).stdout
                if "System.Int32::ToString" not in il: raise ValueError("Unexpected scalar leaf")
                text, count = re.subn(r"__cil_memberref_call_[0-9A-F]{8}\(ref (\w+)\)", r"\1.ToString()", text, count=1)
                if count != 1: raise ValueError("Scalar location leaf missing")
            field_tokens.update(re.findall(r"__cil_(ldsflda|ldflda)_([0-9A-F]{8})", text))
            executable = "\n".join(line for line in text.splitlines() if not line.strip().startswith("//"))
            executable = re.sub(r"__cil_(?:ldsflda|ldflda)_[0-9A-F]{8}", "", executable)
            executable = re.sub(r"__cil_(?:F|arg_r4|load_r4|store_r4|return_r4)\b", "", executable)
            if "__cil_" in support.without_boolean_helpers(executable):
                raise ValueError("Unbound managed-location body: " + name)
            methods[name] = text
        if not methods: raise ValueError("Owned managed-location selection empty")
        field_plan = OUT / f"fields-{config}.json"
        support.write_changed(field_plan, json.dumps([{"opcode": opcode, "token": token} for opcode, token in sorted(field_tokens)]))
        field_leaves = support.run([support.DOTNET / "dotnet", harness, fixture, "--managed-field-plan", field_plan]).stdout
        def compile_carrier(label, bodies, leaves=field_leaves):
            source = OUT / f"{label}-{config}.cs"
            support.write_changed(source, "using System.Linq;\nusing __cil_F = System.Single;\nnamespace ManagedLocationCases { public static class Recovered {\n"
                + "public static int Trace; public static int GlobalNumber; public static Cell Global;\n"
                + "static float __cil_arg_r4(float value) => value;\n"
                + "static float __cil_load_r4(float value) => value;\n"
                + "static float __cil_store_r4(float value) => value;\n"
                + "static float __cil_return_r4(float value) => value;\n"
                + leaves + "\n".join(bodies.values()) + "\n} }\n")
            binary = source.with_suffix(".dll")
            support.compile_cs(binary, [source], references=[fixture])
            return binary
        binary = compile_carrier("Recovered", methods)
        command = [support.DOTNET / "dotnet", harness, fixture, "--managed-location-check", binary]
        result = support.run(command)
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip(), "methods": list(methods)})
        if args.mutation_control:
            for name in ["Update", "Text", "Aliased", "Captured", "ThrowAfterWrite"]:
                if name not in methods: raise ValueError("Mutation method missing: " + name)
                text = methods[name]
                if name in ["Update", "Text"]:
                    bad, count = re.subn(r"return [^;]+;", "return next;", text, count=1)
                elif name == "Captured":
                    field = re.search(r"(__cil_ldflda_[0-9A-F]{8})\(", text)
                    location = field[1] + "(ManagedLocationCases.Recovered.Global)" if field else "ManagedLocationCases.Recovered.Global.Number"
                    bad, count = re.subn(r"ref (\w+), (\w+)\)", lambda m: "ref " + location + ", " + m[2] + ")", text, count=1)
                else:
                    bad, count = re.subn(r"(?m)^\s*(v\d+) = (?:17|v\d+);", "", text, count=1)
                if count != 1: raise ValueError("Mutation expression missing: " + name)
                mutated = dict(methods); mutated[name] = bad
                binary = compile_carrier("Wrong-" + name, mutated)
                negative = subprocess.run([str(x) for x in [*command[:4], binary]],
                                          text=True, capture_output=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Wrong-{name}-{config}.log").write_text(log)
                if negative.returncode == 0 or "Recovered managed location differs:" not in log:
                    raise ValueError("Semantic mutant not detected: " + name + "\n" + log)
                print(config + ": rejected " + name + " mutant", flush=True)
                rows.append({"configuration": config, "mutation": name, "detected": True})
            if config.endswith("-bool-bits"):
                for name, source_name, pattern, replacement in [
                    ("BoolStore", "Flag", r"__cil_bool_store\((\w+)\)", r"(\1 != 0)"),
                    ("BoolLoad", "BooleanBits", r"__cil_bool_load\((\w+)\)", r"(\1 ? 1 : 0)"),
                ]:
                    bad, count = re.subn(pattern, replacement, methods[source_name], count=1)
                    if count != 1: raise ValueError("Boolean bit mutation missing: " + name)
                    changed = dict(methods); changed[source_name] = bad
                    binary = compile_carrier("Wrong-" + name, changed)
                    negative = subprocess.run([str(x) for x in [*command[:4], binary]], text=True, capture_output=True, timeout=120)
                    log = negative.stdout + negative.stderr
                    (OUT / f"Wrong-{name}-{config}.log").write_text(log)
                    if negative.returncode == 0 or "Recovered managed location differs:" not in log:
                        raise ValueError("Boolean normalization mutant survived: " + name + log)
                    print(config + ": rejected " + name + " mutant", flush=True)
                    rows.append({"configuration": config, "mutation": name, "detected": True})
            if config.endswith("-readonly"):
                name = re.search(r"(__cil_ldflda_[0-9A-F]{8})\(", methods["Captured"])[1]
                pattern = r"private static ref global::System.Int32 " + name + r"\(global::ManagedLocationCases.Cell receiver\) => ref [^;]+;"
                copied, count = re.subn(pattern, "private static int copiedField; private static ref int " + name
                    + "(global::ManagedLocationCases.Cell receiver) { copiedField = receiver.Number; return ref copiedField; }", field_leaves)
                if count != 1: raise ValueError("Field-copy mutation target missing")
                binary = compile_carrier("Wrong-FieldCopy", methods, copied)
                negative = subprocess.run([str(x) for x in [*command[:4], binary]], text=True, capture_output=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Wrong-FieldCopy-{config}.log").write_text(log)
                if negative.returncode == 0 or "Recovered managed location differs:" not in log:
                    raise ValueError("Copied field location was not detected: " + log)
                print(config + ": rejected FieldCopy mutant", flush=True)
                rows.append({"configuration": config, "mutation": "FieldCopy", "detected": True})
    (OUT / "results.json").write_text(json.dumps({"rows": rows, "third_party_execution": False}, indent=2) + "\n")


if __name__ == "__main__":
    main()
