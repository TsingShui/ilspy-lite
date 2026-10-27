#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute owned recovered token and allocation bodies with exact one-op leaves."""
import importlib.util
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("object_roundtrip", ROOT / "tests/object-roundtrip.py")
support = importlib.util.module_from_spec(spec)
spec.loader.exec_module(support)
CLI = ROOT / "target/release/ilspy-lite"


def recover(fixture, name):
    listing = support.run([CLI, "list", fixture]).stdout.splitlines()
    line = next(line for line in listing if "TokenCases.Tokens::" + name in line)
    token = line.split(" ", 1)[0]
    return support.run([CLI, "decompile", fixture, "--method", token]).stdout


def one(pattern, replacement, text):
    result, count = re.subn(pattern, replacement, text)
    if not count:
        raise ValueError("Missing expected identity binding: " + pattern)
    return result


def prepare(fixture):
    methods = {name: recover(fixture, name) for name in
               ("TypeName", "ClosedName", "Dates", "Days", "StaticDelegate", "InstanceDelegate")}
    for name, target in (("TypeName", "TokenLeaves.GenericHandle<T>()"),
                         ("ClosedName", "TokenLeaves.StringHandle()")):
        text = methods[name]
        text = one(r"__cil_value_[0-9a-f]+", "System.RuntimeTypeHandle", text)
        text = one(r"__cil_type_[0-9A-F]+", "System.Type", text)
        methods[name] = one(r"__cil_ldtoken_type_[0-9A-F]+", target, text)
    for name, ty, leaf in (("Dates", "System.DateTime[]", "TokenLeaves.NewDates"),
                           ("Days", "System.DayOfWeek[]", "TokenLeaves.NewDays")):
        text = one(r"__cil_ref_[0-9a-f]+", ty, methods[name])
        methods[name] = one(r"__cil_newarr_[0-9A-F]+", leaf, text)
    tokens = {}
    for name, leaf in (("StaticDelegate", "TokenLeaves.StaticPointer()"),
                       ("InstanceDelegate", "TokenLeaves.InstancePointer()")):
        text = one(r"__cil_ref_[0-9a-f]+", "System.Func<int, int>", methods[name])
        found = re.search(r"__cil_ldftn_([0-9A-F]+)", text)
        if not found:
            raise ValueError("No function-pointer leaf in " + name)
        tokens[name] = int(found.group(1), 16)
        text = one(r"__cil_ldftn_[0-9A-F]+", leaf, text)
        text = one(r"__cil_newobj_[0-9A-F]+", "TokenLeaves.New", text)
        if name == "StaticDelegate":
            text = one(r"TokenCases\.Tokens\.__name_[0-9A-F]+\.__il_field_[0-9A-F]+",
                       "TokenCases.RecoveredTokens.Cached", text)
        methods[name] = text
    template = (OUT / "TokenBehaviorTemplate.cs.txt").read_text()
    template = template.replace("__STATIC_METHOD__", str(tokens["StaticDelegate"]))
    template = template.replace("__INSTANCE_METHOD__", str(tokens["InstanceDelegate"]))
    return template.replace("__RECOVERED_METHODS__", "\n".join(methods.values()))


def execute(config, fixture, label, source):
    directory = OUT / config / label
    directory.mkdir(parents=True, exist_ok=True)
    cs = directory / "TokenBehavior.cs"
    cs.write_text(source)
    binary = directory / "TokenBehavior.dll"
    support.compile_cs(binary, [cs], executable=True, references=[fixture])
    shutil.copyfile(fixture, directory / (fixture.stem + ".dll"))
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    binary.with_suffix(".runtimeconfig.json").write_text(json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    result = subprocess.run([str(support.DOTNET / "dotnet"), str(binary)], text=True,
                            capture_output=True, timeout=120)
    (directory / "execution.log").write_text(result.stdout + result.stderr)
    return result


def main():
    rows = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/token-{config}.bin"
        source = prepare(fixture)
        result = execute(config, fixture, "original", source)
        if result.returncode or "Owned token behavior:" not in result.stdout:
            raise ValueError("Owned behavior failed: " + result.stdout + result.stderr)
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip()})
        for name, old, new in [
            ("GenericScope", "TokenLeaves.GenericHandle<T>()", "TokenLeaves.StringHandle()"),
            ("CachedIdentity", "TokenCases.RecoveredTokens.Cached = v2;", "// no cached delegate"),
            ("ArrayLength", "TokenLeaves.NewDates(2)", "TokenLeaves.NewDates(3)"),
        ]:
            if old not in source:
                raise ValueError("Mutation target missing: " + name)
            mutant = execute(config, fixture, "wrong-" + name, source.replace(old, new, 1))
            if mutant.returncode == 0 or "Recovered token behavior differs:" not in mutant.stderr:
                raise ValueError("Semantic mutant survived or did not execute: " + name + mutant.stdout + mutant.stderr)
            print(config + ": rejected " + name, flush=True)
            rows.append({"configuration": config, "mutation": name, "detected": True})
    (OUT / "behavior-manifest.json").write_text(json.dumps({"rows": rows, "third_party_execution": False}, indent=2) + "\n")


if __name__ == "__main__":
    main()
