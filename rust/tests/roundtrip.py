#!/usr/bin/env python3
"""Recompile supported fixture output with .NET 10 and compare executions.

Uses the SDK's Roslyn compiler directly; no repository SDK override or restore.
Never runs arbitrary user assemblies. Inputs are the two pinned test fixtures.
"""
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT

import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
DOTNET_ROOT = Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10"))
DOTNET = DOTNET_ROOT / "dotnet"


def latest(directory, prefix):
    candidates = [p for p in directory.iterdir() if p.name.startswith(prefix)]
    if not candidates:
        raise RuntimeError(f"Missing .NET 10 installation in {directory}")
    return max(candidates, key=lambda p: tuple(int(n) for n in p.name.split(".")))


def run(*args, **kwargs):
    return subprocess.run(args, check=True, text=True, **kwargs)


sdk = latest(DOTNET_ROOT / "sdk", "10.")
ref = latest(DOTNET_ROOT / "packs/Microsoft.NETCore.App.Ref", "10.") / "ref/net10.0"
runtime = latest(DOTNET_ROOT / "shared/Microsoft.NETCore.App", "10.")
run("cargo", "build", "--release", "--locked", cwd=ROOT)
cli = ROOT / "target/release/ilspy-lite"
with tempfile.TemporaryDirectory(prefix="ilspy-rust-roundtrip-") as directory:
    temp = Path(directory)
    for configuration in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/{configuration}.bin"
        listing = run(str(cli), "list", str(fixture), capture_output=True).stdout
        integer_fixture = ROOT / f"tests/fixtures/integer-{configuration}.bin"
        integer_listing = run(str(cli), "list", str(integer_fixture), capture_output=True).stdout
        integer_methods = []
        for line in integer_listing.splitlines():
            integer_methods.append(run(str(cli), "decompile", str(integer_fixture), "--method", line.split()[0], "--details", "--no-auto-references", capture_output=True).stdout)
        if len(integer_methods) != 25:
            raise AssertionError("Integer fixture inventory changed")
        methods = []
        for line in listing.splitlines():
            if "LiteSamples.ControlFlowSample::" not in line or "::Divide(" in line:
                continue
            token = line.split()[0]
            methods.append(run(str(cli), "decompile", str(fixture), "--method", token, "--details", "--no-auto-references",
                               capture_output=True).stdout)
        if len(methods) != 9:
            raise AssertionError(f"Expected 9 supported methods, found {len(methods)}")
        source = temp / "Recovered.cs"
        source.write_text("namespace LiteSamples { public static class ControlFlowSample {\n"
                          "public static int state;\n" + "\n".join(methods) + "\n} }\n"
                          "namespace IntegerCases { public static class Cases {\n"
                          "public static int state;\n" + "\n".join(integer_methods) + "\n} }\n")
        runner = temp / "Roundtrip.dll"
        response = temp / "compile.rsp"
        response.write_text("\n".join([
            "-nologo", "-target:exe", "-langversion:14", "-warnaserror+",
            # Labels, stack snapshots, and spill slots are intentionally verbose.
            "-nowarn:0164,0219", f'-out:"{runner}"',
            *[f'-r:"{p}"' for p in sorted(ref.glob("*.dll"))],
            f'"{source}"', f'"{ROOT / "tests/dotnet/Roundtrip.cs"}"',
        ]))
        (temp / "Roundtrip.runtimeconfig.json").write_text(json.dumps({"runtimeOptions": {
            "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}
        }}))
        run(str(DOTNET), str(sdk / "Roslyn/bincore/csc.dll"), f"@{response}")
        run(str(DOTNET), str(runner), str(fixture), configuration, str(integer_fixture))
