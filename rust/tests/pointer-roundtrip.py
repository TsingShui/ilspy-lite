#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute recovered unmanaged-pointer bodies on owned stack memory."""
import argparse
import importlib.util
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/pointers/behavior"
spec = importlib.util.spec_from_file_location("object_roundtrip", ROOT / "tests/object-roundtrip.py")
support = importlib.util.module_from_spec(spec)
spec.loader.exec_module(support)

SELECTED = ["ReadPointer", "ReadWrite", "SamePointer", "NullPointer", "OffsetValue", "Difference"]

PROGRAM = r'''
using System;

internal static unsafe class Program
{
    private static int Main()
    {
        int comparisons = 0;
        void Check(string name, bool equal)
        {
            comparisons++;
            if (!equal) { Console.Error.WriteLine("Recovered pointer behavior differs: " + name); Environment.Exit(1); }
        }
        int left = 41, right = 7;
        Check("ReadPointer", RelationCases.Ops.ReadPointer(&left) == RecoveredCases.Ops.ReadPointer(&left));
        int a = 41, b = 41;
        int beforeOurs = RelationCases.Ops.ReadWrite(&a);
        int beforeTheirs = RecoveredCases.Ops.ReadWrite(&b);
        Check("ReadWrite", beforeOurs == beforeTheirs && a == b);
        Check("SamePointer", RelationCases.Ops.SamePointer(&a, &a) == RecoveredCases.Ops.SamePointer(&b, &b));
        Check("DifferentPointer", RelationCases.Ops.SamePointer(&a, &right) == RecoveredCases.Ops.SamePointer(&b, &right));
        Check("NullPointer", RelationCases.Ops.NullPointer(null) == RecoveredCases.Ops.NullPointer(null));
        Check("NonNullPointer", RelationCases.Ops.NullPointer(&a) == RecoveredCases.Ops.NullPointer(&b));
        int* values = stackalloc int[3];
        values[0] = 10; values[1] = 20; values[2] = 30;
        int* other = stackalloc int[3];
        other[0] = 10; other[1] = 20; other[2] = 30;
        Check("OffsetValue", RelationCases.Ops.OffsetValue(values, 2) == RecoveredCases.Ops.OffsetValue(other, 2));
        Check("NegativeOffset", RelationCases.Ops.OffsetValue(values + 2, -1) == RecoveredCases.Ops.OffsetValue(other + 2, -1));
        Check("Difference", RelationCases.Ops.Difference(values + 2, values) == RecoveredCases.Ops.Difference(other + 2, other));
        Console.WriteLine($"General pointer behavior: {comparisons} comparisons; dereference, in-place write, null test, scaling and element difference match.");
        return 0;
    }
}
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--mutation-control", action="store_true")
    args = parser.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    support.run([sys.executable, ROOT / "tests/build-reference-relation-fixtures.py"])
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/reference-relation-{config}.bin"
        bodies = {}
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            match = re.search(r"RelationCases\.Ops::(\w+)\(", signature)
            if not match or match[1] not in SELECTED:
                continue
            result = support.run([args.cli, "decompile", fixture, "--method", token])
            if result.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0":
                raise ValueError("Owned pointer method not recovered: " + result.stdout)
            bodies[match[1]] = result.stdout
            (OUT / f"{config}-{token}.readable.txt").write_text(result.stdout)
        if sorted(bodies) != sorted(SELECTED):
            raise ValueError("Pointer selection drift: " + repr(sorted(bodies)))
        carrier = ("using System;\nnamespace RecoveredCases {\npublic static unsafe class Ops {\n"
                   + "\n".join(bodies[name] for name in SELECTED) + "\n} }\n")
        path = OUT / f"Recovered-{config}.cs"
        support.write_changed(path, carrier)
        program = OUT / "Program.cs"
        support.write_changed(program, PROGRAM)
        binary = OUT / f"Pointer-{config}.dll"
        support.compile_cs(binary, [path, program], executable=True, references=[fixture], unsafe=True)
        runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
        support.write_changed(binary.with_suffix(".runtimeconfig.json"), __import__("json").dumps({"runtimeOptions": {
            "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
        for source in (fixture, ROOT / f"tests/fixtures/relation-library-{config}.bin"):
            target = OUT / (source.stem + ".dll")
            if not target.is_file() or target.read_bytes() != source.read_bytes():
                target.write_bytes(source.read_bytes())
        result = subprocess.run([str(support.DOTNET / "dotnet"), str(binary)], capture_output=True, text=True, timeout=120)
        print(config + ": " + result.stdout.strip(), flush=True)
        if result.returncode:
            raise ValueError("pointer behavior failed: " + result.stdout + result.stderr)
        result = subprocess.run([str(support.DOTNET / "dotnet"), str(binary)], capture_output=True, text=True, timeout=120)
        print(config + ": " + result.stdout.strip(), flush=True)
        if result.returncode:
            raise ValueError("pointer behavior failed: " + result.stdout + result.stderr)
        if args.mutation_control:
            for label, pattern, replacement in [
                # Dropping the in-place write must change the observed memory.
                ("DroppedWrite", r"\(\*pointer\) = v5;", "v5 = unchecked(v5 + 0);"),
                # Scaling the byte offset again by the pointee must read the wrong slot.
                ("DoubleScaling", r"\(\((\w+)\*\)\(\(byte\*\)\((\w+)\) \+ \((\w+)\)\)\)",
                 lambda m: f"(({m.group(2)} + {m.group(3)}))"),
            ]:
                changed, count = re.subn(pattern, replacement, carrier, count=1)
                if count != 1:
                    raise ValueError(label + " mutation missing")
                mutant_source = OUT / f"Wrong-{label}-{config}.cs"
                support.write_changed(mutant_source, changed)
                mutant = OUT / f"Wrong-{label}-{config}.dll"
                support.compile_cs(mutant, [mutant_source, program], executable=True,
                                   references=[fixture], unsafe=True)
                support.write_changed(mutant.with_suffix(".runtimeconfig.json"), __import__("json").dumps({"runtimeOptions": {
                    "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
                negative = subprocess.run([str(support.DOTNET / "dotnet"), str(mutant)], capture_output=True, text=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Wrong-{label}-{config}.log").write_text(log)
                if negative.returncode == 0 or "differs" not in log:
                    raise ValueError(label + " mutant survived: " + log)
                print(config + ": rejected " + label, flush=True)
        result = subprocess.run([str(support.DOTNET / "dotnet"), str(binary)], capture_output=True, text=True, timeout=120)
        print(config + ": " + result.stdout.strip(), flush=True)
        if result.returncode:
            raise ValueError("pointer behavior failed: " + result.stdout + result.stderr)
        if args.mutation_control:
            for label, pattern, replacement in [
                # Dropping the in-place write must change the observed memory.
                ("DroppedWrite", r"\(\*pointer\) = v5;", "v5 = unchecked(v5 + 0);"),
                # Scaling the byte offset again by the pointee must read the wrong slot.
                ("DoubleScaling", r"\(\((\w+)\*\)\(\(byte\*\)\((\w+)\) \+ \((\w+)\)\)\)",
                 lambda m: f"(({m.group(2)} + {m.group(3)}))"),
            ]:
                changed, count = re.subn(pattern, replacement, carrier, count=1)
                if count != 1:
                    raise ValueError(label + " mutation missing")
                mutant_source = OUT / f"Wrong-{label}-{config}.cs"
                support.write_changed(mutant_source, changed)
                mutant = OUT / f"Wrong-{label}-{config}.dll"
                support.compile_cs(mutant, [mutant_source, program], executable=True,
                                   references=[fixture], unsafe=True)
                support.write_changed(mutant.with_suffix(".runtimeconfig.json"), __import__("json").dumps({"runtimeOptions": {
                    "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
                negative = subprocess.run([str(support.DOTNET / "dotnet"), str(mutant)], capture_output=True, text=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Wrong-{label}-{config}.log").write_text(log)
                if negative.returncode == 0 or "differs" not in log:
                    raise ValueError(label + " mutant survived: " + log)
                print(config + ": rejected " + label, flush=True)


if __name__ == "__main__":
    main()
