#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute default reference-call bodies with exact original MemberRef leaves."""
import argparse
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/reference-call-foundation/behavior"


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
    support.run([sys.executable, ROOT / "tests/build-reference-call-fixtures.py"])
    sources = [ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"] + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs"))
    harness = OUT / "ReferenceCallRoundtrip.dll"
    support.compile_cs(harness, sources, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ["release", "debug"]:
        fixture = ROOT / f"tests/fixtures/reference-call-{config}.bin"
        library = ROOT / f"tests/fixtures/reference-call-library-{config}.bin"
        names = {line.split(" ", 1)[0]: line.split("::", 1)[1].split("(", 1)[0]
                 for line in support.run([args.cli, "list", fixture]).stdout.splitlines()}
        for variant in ["ordinary", "reference-unbox"]:
            image = fixture
            if variant == "reference-unbox":
                data = fixture.read_bytes()
                for name in ["ArrayClone", "Cast"]:
                    token = next(token for token, method in names.items() if method == name)
                    il = support.run([args.cli, "il", fixture, "--method", token]).stdout
                    blocks = re.findall(r"\.code-bytes IL_[0-9A-F]+: \[([0-9A-F, ]+)\]", il)
                    body = bytes(int(byte, 16) for block in blocks for byte in block.split(", "))
                    if not body or data.count(body) != 1: raise ValueError("Owned cast body not unique")
                    cast = body.index(0x74)
                    start = data.index(body)
                    data = data[:start+cast] + bytes([0xa5]) + data[start+cast+1:]
                image = OUT / f"reference-unbox-{config}.bin"
                image.write_bytes(data)
            methods = {}
            for row in map(json.loads, support.run([args.cli, "scan", image, "--no-auto-references"]).stdout.splitlines()):
                if row["status"] != "Decompiled": continue
                token, name = row["token"], names[row["token"]]
                readable = support.run([args.cli, "decompile", image, "--method", token, "--no-auto-references"]).stdout
                (OUT / f"{variant}-{config}-{name}.readable.txt").write_text(readable)
                body = generic.inline_structural_types(generic.without_dependency_report(readable))
                methods[name] = body
            if not methods: raise ValueError("Owned reference-call selection is empty")
            calls = sorted({(op, token) for text in methods.values() for op, token in
                            re.findall(r"__cil_(callvirt|reference_call|memberref_call)_([0-9A-F]{8})\(", text)})
            aliases = sorted({token for text in methods.values() for token in re.findall(r"__cil_type_([0-9A-F]{8})", text)})
            directives = OUT / f"{variant}-{config}-leaves.json"
            directives.write_text(json.dumps({"calls": [{"operation": op, "token": token} for op, token in calls],
                                              "aliases": aliases}, indent=2) + "\n")
            plan = json.loads(support.run([support.DOTNET / "dotnet", harness, image,
                                           "--reference-call-plan", directives, library]).stdout)
            def compile_carrier(label, bodies):
                source = OUT / f"{label}-{variant}-{config}.cs"
                support.write_changed(source, "\n".join(plan["aliases"]) + "\nnamespace ReferenceCallCases { public static class Recovered {\n"
                    + plan["support"] + "\n" + "\n".join(plan["helpers"]) + "\n" + "\n".join(bodies.values()) + "\n} }\n")
                binary = source.with_suffix(".dll")
                support.compile_cs(binary, [source], references=[image, library])
                return binary
            binary = compile_carrier("Recovered", methods)
            command = [support.DOTNET / "dotnet", harness, image, "--reference-call-check", binary, library]
            result = support.run(command)
            print(config + "/" + variant + ": " + result.stdout.strip(), flush=True)
            rows.append({"configuration": config, "variant": variant, "behavior": result.stdout.strip(), "methods": list(methods)})
            if args.mutation_control:
                for name in ["IntGet", "IntTake", "ArgumentOrder", "ArrayClone", "As"]:
                    if name not in methods: raise ValueError("Mutation method missing: " + name)
                    if name in ("ArrayClone", "As"):
                        bad, count = re.subn(r"return [^;]+;", "return " + ("value" if name == "ArrayClone" else "null") + ";", methods[name], count=1)
                    elif name == "ArgumentOrder":
                        bad, count = re.subn(r"__cil_callvirt_[0-9A-F]{8}\(log, [^()]*\)", "1", methods[name], count=1)
                    else:
                        bad, count = re.subn(r"return [^;]+;", "return 0;", methods[name], count=1)
                    if count != 1: raise ValueError("Mutation expression missing: " + name)
                    mutated = dict(methods); mutated[name] = bad
                    binary = compile_carrier("Wrong-" + name, mutated)
                    negative = subprocess.run([str(x) for x in [*command[:4], binary, library]],
                                              text=True, capture_output=True, timeout=120)
                    log = negative.stdout + negative.stderr
                    (OUT / f"Wrong-{name}-{variant}-{config}.log").write_text(log)
                    if negative.returncode == 0 or "Recovered reference call differs:" not in log:
                        raise ValueError("Semantic mutant not detected: " + name + "\n" + log)
                    rows.append({"configuration": config, "variant": variant, "mutation": name, "detected": True})
                print(config + "/" + variant + ": rejected five semantic mutants", flush=True)
    (OUT / "results.json").write_text(json.dumps({"rows": rows, "third_party_execution": False}, indent=2) + "\n")


if __name__ == "__main__":
    main()
