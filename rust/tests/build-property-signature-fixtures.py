#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Build three owned metadata-scope fixtures without restore or input execution."""
from pathlib import Path
import os
import shutil
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
dotnet = Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10"))
def latest(path):
    return max((p for p in path.iterdir() if p.name.startswith("10.")),
               key=lambda p: tuple(map(int, p.name.split("."))))
sdk = latest(dotnet / "sdk")
ref = latest(dotnet / "packs/Microsoft.NETCore.App.Ref") / "ref/net10.0"
with tempfile.TemporaryDirectory() as temporary:
    directory = Path(temporary)
    for name, define, dependencies in [
        ("PropertyPayload", "PAYLOAD", []),
        ("PropertyMiddle", "MIDDLE", ["PropertyPayload"]),
        ("PropertyConsumer", "", ["PropertyPayload", "PropertyMiddle"]),
    ]:
        output = directory / (name + ".dll")
        response = directory / (name + ".rsp")
        response.write_text("\n".join([
            "-nologo", "-target:library", "-langversion:14", "-warnaserror+", "-deterministic+", "-optimize+",
            *(["-define:" + define] if define else []), f'-out:"{output}"',
            *[f'-r:"{p}"' for p in sorted(ref.glob("*.dll"))],
            *[f'-r:"{directory / (dependency + ".dll")}"' for dependency in dependencies],
            f'"{root / "tests/dotnet/PropertySignatureCases.cs"}"',
        ]))
        subprocess.run([str(dotnet / "dotnet"), str(sdk / "Roslyn/bincore/csc.dll"), f"@{response}"],
                       check=True, timeout=120)
        shutil.copyfile(output, root / ("tests/fixtures/property-signature-" + name + ".bin"))
