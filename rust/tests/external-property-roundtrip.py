#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute default readable property bodies; compare independent receiver states."""
import argparse
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/external-properties/behavior"
spec = importlib.util.spec_from_file_location("object_roundtrip", ROOT / "tests/object-roundtrip.py")
support = importlib.util.module_from_spec(spec)
spec.loader.exec_module(support)


def mutate(name, body):
    if name == "Get":
        changed, count = re.subn(r"\.Number;", ".Value;", body, count=1)
    elif name == "Set":
        changed, count = re.subn(r"\.Number = ", ".Value = ", body, count=1)
    elif name == "SetComputed":
        index = body.index("{") + 1
        return body[:index] + "\nif (cell == null) throw new System.NullReferenceException();\n" + body[index:]
    else:
        raise ValueError("Unknown mutation")
    if count != 1:
        raise ValueError("Mutation target missing: " + name)
    return changed


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--mutation-control", action="store_true")
    args = parser.parse_args()
    if not args.cli.is_file():
        parser.error("Existing Rust CLI required")
    OUT.mkdir(parents=True, exist_ok=True)
    support.run([sys.executable, ROOT / "tests/build-external-property-fixtures.py"])
    sources = [ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"] + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs"))
    harness = OUT / "PropertyRoundtrip.dll"
    support.compile_cs(harness, sources, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ["release", "debug"]:
        fixture = ROOT / f"tests/fixtures/external-property-{config}.bin"
        library = ROOT / f"tests/fixtures/external-property-library-{config}.bin"
        reference = ["--no-auto-references", "--reference", library]
        names = {}
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, sig = line.split(" ", 1)
            match = re.search(r"ExternalPropertyCases\.Ops::([^\s(]+)\(", sig)
            if match:
                names[token] = match[1]
        scan = support.run([args.cli, "scan", fixture, *reference])
        methods = {}
        for row in map(json.loads, scan.stdout.splitlines()):
            if row["status"] != "Decompiled":
                continue
            if row["token"] not in names:
                raise ValueError("Unclassified recovered method")
            name = names[row["token"]]
            result = support.run([args.cli, "decompile", fixture, "--method", row["token"], *reference])
            if result.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0":
                raise ValueError("Scan/default-view admission drift")
            body = "\n".join(line for line in result.stdout.splitlines()
                if not line.startswith(("// Dependency", "// Dependencies")))
            methods[name] = body
            (OUT / f"{config}-{name}.cs").write_text(body)
        if not methods:
            raise ValueError("Owned property selection is empty")

        def compile_carrier(label, bodies):
            source = OUT / f"{label}-{config}.cs"
            support.write_changed(source, "namespace ExternalPropertyCases { public static class Recovered {\n"
                + "\n".join(bodies.values()) + "\n} }\n")
            binary = source.with_suffix(".dll")
            support.compile_cs(binary, [source], references=[fixture, library])
            return binary

        binary = compile_carrier("Recovered", methods)
        command = [support.DOTNET / "dotnet", harness, fixture, "--external-property-check", binary, library]
        result = support.run(command)
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip(), "selected": list(methods)})
        if args.mutation_control:
            for name in ["Get", "Set", "SetComputed"]:
                if name not in methods:
                    raise ValueError("Required mutation method missing: " + name)
                modified = dict(methods)
                modified[name] = mutate(name, modified[name])
                mutant = compile_carrier("Mutant-" + name, modified)
                run = subprocess.run([str(x) for x in [*command[:4], mutant, library]],
                                     capture_output=True, text=True, timeout=120)
                log = run.stdout + run.stderr
                (OUT / f"Mutant-{name}-{config}.log").write_text(log)
                if run.returncode == 0 or "Recovered property differs: " + name not in log:
                    raise ValueError("Mutation did not fail on its semantic difference: " + name + "\n" + log)
                rows.append({"configuration": config, "mutation": name, "detected": True})
                print(config + ": rejected " + name + " mutant", flush=True)
    (OUT / "results.json").write_text(json.dumps({"rows": rows, "third_party_execution": False}, indent=2)+"\n")


if __name__ == "__main__":
    main()
