#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Build the owned integer conversion corpus using .NET 10's compiler, without restore."""
from pathlib import Path
import os
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
dotnet = Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10"))
def latest(path):
    return max((p for p in path.iterdir() if p.name.startswith("10.")),
               key=lambda p: tuple(map(int, p.name.split("."))))
sdk = latest(dotnet / "sdk")
ref = latest(dotnet / "packs/Microsoft.NETCore.App.Ref") / "ref/net10.0"
for config in ("release", "debug"):
    with tempfile.TemporaryDirectory() as directory:
        response = Path(directory) / "compile.rsp"
        response.write_text("\n".join([
            "-nologo", "-target:library", "-langversion:14", "-warnaserror+", "-deterministic+",
            "-optimize+" if config == "release" else "-optimize-",
            f'-out:"{root / f"tests/fixtures/conversion-{config}.bin"}"',
            *[f'-r:"{p}"' for p in sorted(ref.glob("*.dll"))],
            f'"{root / "tests/dotnet/ConversionCases.cs"}"',
        ]))
        subprocess.run([str(dotnet / "dotnet"), str(sdk / "Roslyn/bincore/csc.dll"), f"@{response}"], check=True)
