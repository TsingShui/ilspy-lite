#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Compile owned virtual dispatch fixtures only when their source or toolchain changes."""
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
DOTNET = Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10"))


def latest(path):
    return max((p for p in path.iterdir() if p.name.startswith("10.")),
               key=lambda p: tuple(map(int, p.name.split("."))))


sdk = latest(DOTNET / "sdk")
refs = latest(DOTNET / "packs/Microsoft.NETCore.App.Ref") / "ref/net10.0"
source = ROOT / "tests/dotnet/VirtualCallCases.cs"
cache = ROOT / "report/virtual-call-roundtrip"
cache.mkdir(parents=True, exist_ok=True)
stamp = {"source": hashlib.sha256(source.read_bytes()).hexdigest(), "sdk": str(sdk), "refs": str(refs)}
for config in ("release", "debug"):
    output = ROOT / f"tests/fixtures/virtual-call-{config}.bin"
    previous = cache / f"fixture-{config}.json"
    if output.is_file() and previous.is_file() and json.loads(previous.read_text()) == stamp:
        print(f"{config}: cached owned fixture")
        continue
    response = cache / f"fixture-{config}.rsp"
    response.write_text("\n".join([
        "-nologo", "-target:library", "-langversion:14", "-warnaserror+", "-deterministic+",
        "-optimize+" if config == "release" else "-optimize-", f'-out:"{output}"',
        *[f'-r:"{p}"' for p in sorted(refs.glob("*.dll"))], f'"{source}"']) + "\n")
    subprocess.run([str(DOTNET / "dotnet"), str(sdk / "Roslyn/bincore/csc.dll"),
                    "@" + str(response)], check=True)
    previous.write_text(json.dumps(stamp, indent=2) + "\n")
    print(f"{config}: compiled owned fixture")
