#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Build owned cross-assembly binding fixtures with .NET 10, without restore."""
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
for config in ("release", "debug"):
    with tempfile.TemporaryDirectory() as directory:
        dependency = Path(directory) / "BindingLibrary.dll"
        for output, define, extra in [
            (dependency, "-define:DEPENDENCY", []),
            (root / f"tests/fixtures/binding-{config}.bin", "", [f'-r:"{dependency}"']),
        ]:
            response = Path(directory) / "compile.rsp"
            response.write_text("\n".join([
                "-nologo", "-target:library", "-langversion:14", "-warnaserror+", "-deterministic+",
                "-optimize+" if config == "release" else "-optimize-", define,
                f'-out:"{output}"', *[f'-r:"{p}"' for p in sorted(ref.glob("*.dll"))],
                *extra, f'"{root / "tests/dotnet/BindingCases.cs"}"',
            ]))
            subprocess.run([str(dotnet / "dotnet"), str(sdk / "Roslyn/bincore/csc.dll"), f"@{response}"], check=True)
        shutil.copyfile(dependency, root / f"tests/fixtures/binding-library-{config}.bin")
        facade = Path(directory) / "BindingFacade.dll"
        response = Path(directory) / "forwarder.rsp"
        response.write_text("\n".join([
            "-nologo", "-target:library", "-langversion:14", "-warnaserror+", "-deterministic+",
            f'-out:"{facade}"', *[f'-r:"{p}"' for p in sorted(ref.glob("*.dll"))],
            f'-r:"{dependency}"', f'"{root / "tests/dotnet/BindingForwarder.cs"}"',
        ]))
        subprocess.run([str(dotnet / "dotnet"), str(sdk / "Roslyn/bincore/csc.dll"), f"@{response}"], check=True)
        shutil.copyfile(facade, root / f"tests/fixtures/binding-facade-{config}.bin")
        contract_dir = Path(directory) / "contract"
        contract_dir.mkdir()
        contract = contract_dir / "BindingLibrary.dll"
        response = Path(directory) / "contract.rsp"
        response.write_text("\n".join([
            "-nologo", "-target:library", "-refonly", "-define:DEPENDENCY", "-langversion:14", "-warnaserror+", "-deterministic+",
            f'-out:"{contract}"', *[f'-r:"{p}"' for p in sorted(ref.glob("*.dll"))],
            f'"{root / "tests/dotnet/BindingCases.cs"}"',
        ]))
        subprocess.run([str(dotnet / "dotnet"), str(sdk / "Roslyn/bincore/csc.dll"), f"@{response}"], check=True)
        shutil.copyfile(contract, root / f"tests/fixtures/binding-contract-{config}.bin")
