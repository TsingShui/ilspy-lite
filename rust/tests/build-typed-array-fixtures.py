#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Build owned typed vector cases without restore."""
import os
import subprocess
import tempfile
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
DOTNET = Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10"))
def latest(path):
    return max((p for p in path.iterdir() if p.name.startswith("10.")), key=lambda p: tuple(map(int, p.name.split("."))))
sdk = latest(DOTNET / "sdk")
refs = latest(DOTNET / "packs/Microsoft.NETCore.App.Ref") / "ref/net10.0"
for config in ("release", "debug"):
    output = ROOT / f"tests/fixtures/typed-array-{config}.bin"
    with tempfile.TemporaryDirectory() as directory:
        response = Path(directory) / "compile.rsp"
        response.write_text("\n".join(["-nologo", "-target:library", "-langversion:14", "-warnaserror+", "-deterministic+",
            "-optimize+" if config == "release" else "-optimize-", f'-out:"{output}"',
            *[f'-r:"{p}"' for p in sorted(refs.glob("*.dll"))], f'"{ROOT / "tests/dotnet/TypedArrayCases.cs"}"']))
        subprocess.run([str(DOTNET / "dotnet"), str(sdk / "Roslyn/bincore/csc.dll"), "@" + str(response)], check=True)
