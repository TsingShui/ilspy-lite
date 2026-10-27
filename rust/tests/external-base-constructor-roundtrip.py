#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute recovered external base constructors with exact constructor leaves."""
import argparse
import importlib.util
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/external-base-ctor/behavior"
spec = importlib.util.spec_from_file_location("object_roundtrip", ROOT / "tests/object-roundtrip.py")
support = importlib.util.module_from_spec(spec)
spec.loader.exec_module(support)


def selected(cli, fixture, name):
    listing = support.run([cli, "list", fixture]).stdout.splitlines()
    matches = [line.split(" ", 1)[0] for line in listing
               if f"RelationCases.ForeignOuter`2/{name}::.ctor(" in line]
    if len(matches) != 1:
        raise ValueError("Owned constructor selection differs: " + name)
    return matches[0]


def callvirt_fixture(cli, fixture, token, config):
    il = support.run([cli, "il", fixture, "--method", token]).stdout
    chunks = re.findall(r"^\.code-bytes IL_[0-9A-F]+: \[([^]]+)\]$", il, re.M)
    code = bytes.fromhex(" ".join(re.findall(r"\b[0-9A-F]{2}\b", " ".join(chunks))))
    call = re.findall(r"^IL_([0-9A-F]+): call 0x[0-9A-F]+ .*::\.ctor$", il, re.M)
    if not code or len(call) != 1:
        raise ValueError("Owned constructor IL shape differs")
    offset = int(call[0], 16)
    data = bytearray(fixture.read_bytes())
    at = data.find(code)
    if at < 0 or data.find(code, at + 1) >= 0 or code[offset] != 0x28:
        raise ValueError("Owned constructor code bytes are not unique")
    data[at + offset] = 0x6f
    patched = OUT / f"callvirt-reference-relation-{config}.bin"
    patched.write_bytes(data)
    return patched


def body(cli, fixture, library, token, name):
    result = support.run([cli, "decompile", fixture, "--method", token,
                          "--no-auto-references", "--reference", library])
    if result.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0":
        raise ValueError("Owned constructor was not recovered: " + name)
    (OUT / f"{fixture.stem}-{name}.readable.txt").write_text(result.stdout)
    match = re.search(r"^[^\n]+\n\{\n(.*?)\n\}\n", result.stdout, re.S)
    if not match:
        raise ValueError("Recovered constructor body missing: " + name)
    statements = re.sub(r"\bthis\b", "receiver", match.group(1))
    targets = re.findall(r"__cil_ctor_call_([0-9A-F]{8})\(", statements)
    if len(targets) != 1:
        raise ValueError("Expected exactly one constructor operation: " + name)
    return statements, "__cil_ctor_call_" + targets[0]


def carrier(seed, seed_leaf, typed, typed_leaf):
    return f'''using System;
using System.Reflection;
using System.Reflection.Emit;

public static class Recovered<TKey, TValue>
{{
    static Action<RelationCases.ForeignOuter<TKey, TValue>.NestedForeign, int> SeedLeaf = MakeSeedLeaf();
    static Action<RelationCases.ForeignOuter<TKey, TValue>.NestedTyped, TValue, TKey> TypedLeaf = MakeTypedLeaf();

    static Action<RelationCases.ForeignOuter<TKey, TValue>.NestedForeign, int> MakeSeedLeaf()
    {{
        var receiver = typeof(RelationCases.ForeignOuter<TKey, TValue>.NestedForeign);
        var constructor = typeof(RelationLibrary.OpBase<TValue, TKey>).GetConstructor(
            BindingFlags.Instance | BindingFlags.NonPublic, null, new[] {{ typeof(int) }}, null)
            ?? throw new Exception("Owned base constructor missing");
        var method = new DynamicMethod("SeedLeaf", typeof(void), new[] {{ receiver, typeof(int) }}, typeof(Recovered<TKey, TValue>).Module, true);
        var il = method.GetILGenerator();
        il.Emit(OpCodes.Ldarg_0); il.Emit(OpCodes.Ldarg_1); il.Emit(OpCodes.Call, constructor); il.Emit(OpCodes.Ret);
        return method.CreateDelegate<Action<RelationCases.ForeignOuter<TKey, TValue>.NestedForeign, int>>();
    }}

    static Action<RelationCases.ForeignOuter<TKey, TValue>.NestedTyped, TValue, TKey> MakeTypedLeaf()
    {{
        var receiver = typeof(RelationCases.ForeignOuter<TKey, TValue>.NestedTyped);
        var constructor = typeof(RelationLibrary.GenericCtorBase<TValue, TKey>).GetConstructor(
            BindingFlags.Instance | BindingFlags.NonPublic, null, new[] {{ typeof(TValue), typeof(TKey) }}, null)
            ?? throw new Exception("Owned typed base constructor missing");
        var method = new DynamicMethod("TypedLeaf", typeof(void), new[] {{ receiver, typeof(TValue), typeof(TKey) }}, typeof(Recovered<TKey, TValue>).Module, true);
        var il = method.GetILGenerator();
        il.Emit(OpCodes.Ldarg_0); il.Emit(OpCodes.Ldarg_1); il.Emit(OpCodes.Ldarg_2);
        il.Emit(OpCodes.Call, constructor); il.Emit(OpCodes.Ret);
        return method.CreateDelegate<Action<RelationCases.ForeignOuter<TKey, TValue>.NestedTyped, TValue, TKey>>();
    }}

    static void {seed_leaf}(RelationCases.ForeignOuter<TKey, TValue>.NestedForeign receiver, int seed)
        => SeedLeaf(receiver, seed);
    static void {typed_leaf}(RelationCases.ForeignOuter<TKey, TValue>.NestedTyped receiver, TValue first, TKey second)
        => TypedLeaf(receiver, first, second);

    public static void RunSeed(RelationCases.ForeignOuter<TKey, TValue>.NestedForeign receiver, int seed)
    {{
{seed}
    }}
    public static void RunTyped(RelationCases.ForeignOuter<TKey, TValue>.NestedTyped receiver, TValue first, TKey second)
    {{
{typed}
    }}
}}
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--mutation-control", action="store_true")
    args = parser.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    support.run([sys.executable, ROOT / "tests/build-reference-relation-fixtures.py"])
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    results = []
    for config in ("release", "debug"):
        original = ROOT / f"tests/fixtures/reference-relation-{config}.bin"
        library = ROOT / f"tests/fixtures/relation-library-{config}.bin"
        token = selected(args.cli, original, "NestedForeign")
        typed_token = selected(args.cli, original, "NestedTyped")
        patched = callvirt_fixture(args.cli, original, token, config)
        for mode, fixture in (("call", original), ("callvirt", patched)):
            seed, seed_leaf = body(args.cli, fixture, library, token, "NestedForeign")
            typed, typed_leaf = body(args.cli, fixture, library, typed_token, "NestedTyped")
            mutations = [("original", seed, typed)]
            if args.mutation_control:
                seed_call = f"{seed_leaf}(receiver, seed);"
                derived = "receiver.Derived = unchecked(seed + 5);"
                typed_call = f"{typed_leaf}(receiver, first, second);"
                for fragment in (seed_call, derived, typed_call):
                    if (seed + typed).count(fragment) != 1:
                        raise ValueError("Mutation target is absent or ambiguous: " + fragment)
                mutations.extend([
                    ("drop-base", seed.replace(seed_call, "", 1), typed),
                    ("wrong-seed", seed.replace(seed_call, f"{seed_leaf}(receiver, unchecked(seed + 1));", 1), typed),
                    ("drop-derived", seed.replace(derived, "", 1), typed),
                    ("swap-arguments", seed, typed.replace(typed_call,
                        f"{typed_leaf}(receiver, (TValue)(object)second, (TKey)(object)first);", 1)),
                ])
            shutil.copyfile(fixture, OUT / f"reference-relation-{config}.dll")
            shutil.copyfile(library, OUT / f"relation-library-{config}.dll")
            for label, seed_body, typed_body in mutations:
                source = OUT / f"{config}-{mode}-{label}.cs"
                support.write_changed(source, carrier(seed_body, seed_leaf, typed_body, typed_leaf))
                executable = source.with_suffix(".dll")
                support.compile_cs(executable, [source, ROOT / "tests/dotnet/ExternalBaseConstructorCheck.cs"],
                                   executable=True, references=[fixture, library])
                executable.with_suffix(".runtimeconfig.json").write_text(json.dumps({"runtimeOptions": {
                    "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
                run = subprocess.run([str(support.DOTNET / "dotnet"), str(executable)],
                                     capture_output=True, text=True, timeout=120)
                (OUT / f"{config}-{mode}-{label}.log").write_text(run.stdout + run.stderr)
                if label == "original":
                    if run.returncode != 0 or "12 comparisons;" not in run.stdout:
                        raise ValueError(f"Owned behavioral comparison failed: {config} {mode}\n{run.stdout}{run.stderr}")
                    print(f"{config} {mode}: {run.stdout.strip()}", flush=True)
                else:
                    if run.returncode == 0 or "Recovered external constructor differs:" not in run.stderr:
                        raise ValueError(f"Semantic mutant not rejected: {config} {mode} {label}\n{run.stdout}{run.stderr}")
                    print(f"{config} {mode}: rejected {label}", flush=True)
                results.append({"configuration": config, "opcode": mode, "case": label, "exit": run.returncode})
    (OUT / "results.json").write_text(json.dumps({"rows": results, "third_party_execution": False}, indent=2) + "\n")


if __name__ == "__main__":
    main()
