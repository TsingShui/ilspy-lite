#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Build owned typed-location and byref fixtures without package restore."""
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
        for output, define, extra in [
            (root / f"tests/fixtures/managed-location-{config}.bin", "", []),
        ]:
            response = Path(directory) / "compile.rsp"
            response.write_text("\n".join([
                "-nologo", "-target:library", "-langversion:14", "-warnaserror+", "-deterministic+",
                "-optimize+" if config == "release" else "-optimize-", define,
                f'-out:"{output}"', *[f'-r:"{p}"' for p in sorted(ref.glob("*.dll"))],
                *extra, f'"{root / "tests/dotnet/ManagedLocationCases.cs"}"',
            ]))
            subprocess.run([str(dotnet / "dotnet"), str(sdk / "Roslyn/bincore/csc.dll"), f"@{response}"], check=True)
