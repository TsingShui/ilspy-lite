#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute recovered general-array bodies on owned inputs."""
import argparse
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/multidim/behavior"
spec = importlib.util.spec_from_file_location("object_roundtrip", ROOT / "tests/object-roundtrip.py")
support = importlib.util.module_from_spec(spec)
spec.loader.exec_module(support)

SELECTED = ["Build", "Cell", "Store", "SumDouble", "Pick"]


def alias_type(encoded):
    data = bytes.fromhex(encoded)
    if data[0] != 0x14:
        raise ValueError("General array alias is not an ELEMENT_TYPE_ARRAY: " + encoded)
    element = {0x08: "int", 0x0D: "double", 0x0E: "string"}[data[1]]
    rank = data[2]
    return element + "[" + "," * (rank - 1) + "]"


def carrier_source(bodies):
    aliases = {}
    for body in bodies.values():
        for alias, encoded in re.findall(r"// structural signature (__cil_\w+) bytes=([0-9A-F]+)", body):
            aliases[alias] = alias_type(encoded)
    source = "using System;\n" + "\n".join(f"using {alias} = {kind};" for alias, kind in aliases.items())
    source += "\nnamespace MultidimCases {\npublic static class Grids {\n"
    for body in bodies.values():
        source += body + "\n"
    return source + "} }\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--mutation-control", action="store_true")
    args = parser.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    support.run([sys.executable, ROOT / "tests/build-multidim-fixtures.py"])
    harness = OUT / "MultidimRoundtrip.dll"
    entry = OUT / "MultidimEntry.cs"
    support.write_changed(entry, "internal static class MultidimEntry\n{\n    public static int Main(string[] arguments) => MultidimBehavior.Run(arguments);\n}\n")
    support.compile_cs(harness, [ROOT / "tests/dotnet/MultidimBehavior.cs", entry], executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/multidim-{config}.bin"
        bodies = {}
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            name = re.search(r"Grids::(\w+)", signature).group(1)
            if name not in SELECTED:
                continue
            result = support.run([args.cli, "decompile", fixture, "--method", token, "--no-auto-references"])
            if result.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0":
                raise ValueError("Owned general-array method not recovered: " + result.stdout)
            bodies[name] = result.stdout
            (OUT / f"{config}-{token}.readable.txt").write_text(result.stdout)
        if len(bodies) != len(SELECTED):
            raise ValueError("General-array selection drift")
        source = carrier_source(bodies)
        path = OUT / f"Recovered-{config}.cs"
        support.write_changed(path, source)
        carrier = path.with_suffix(".dll")
        support.compile_cs(carrier, [path])
        command = [support.DOTNET / "dotnet", harness, fixture, carrier]
        result = support.run(command)
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip()})
        if args.mutation_control:
            for label, pattern, replacement in [
                ("DroppedStore", r"grid\[i, j\] = value;", "grid[i, j] = 0;"),
                ("SwappedDimensions", r"new int\[rows, columns\]", "new int[columns, rows]"),
            ]:
                changed, count = re.subn(pattern, replacement, source, count=1)
                if count != 1:
                    raise ValueError(label + " mutation missing")
                mutant_path = OUT / f"Wrong-{label}-{config}.cs"
                support.write_changed(mutant_path, changed)
                mutant = mutant_path.with_suffix(".dll")
                support.compile_cs(mutant, [mutant_path])
                negative = subprocess.run([str(x) for x in [*command[:2], fixture, mutant]],
                                          capture_output=True, text=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Wrong-{label}-{config}.log").write_text(log)
                if negative.returncode == 0 or "!=" not in log:
                    raise ValueError(label + " mutant survived: " + log)
                print(config + ": rejected " + label, flush=True)
                rows.append({"configuration": config, "mutation": label, "detected": True})
    support.write_changed(OUT / "results.json", json.dumps(rows, indent=2) + "\n")


if __name__ == "__main__":
    main()
