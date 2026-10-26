#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Build owned core-identity metadata without loading the generated images."""
import importlib.util
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("owned_compiler", ROOT / "tests/object-roundtrip.py")
compiler = importlib.util.module_from_spec(spec)
spec.loader.exec_module(compiler)
out = ROOT / "report/core-type-roots"
out.mkdir(parents=True, exist_ok=True)
executable = out / "CoreTypeFixtures.dll"
compile_ms = compiler.compile_cs(executable, [ROOT / "tests/dotnet/CoreTypeFixtures.cs"], executable=True)
runtime = compiler.latest(compiler.DOTNET / "shared/Microsoft.NETCore.App")
executable.with_suffix(".runtimeconfig.json").write_text(json.dumps({"runtimeOptions": {
    "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
files = ["core-types.bin", "core-reference.bin", "core-fake-name.bin", "core-fake-marker.bin",
         "core-wrong-key.bin", "core-wrong-culture.bin", "core-generic-marker.bin"]
if compile_ms or not all((ROOT / "tests/fixtures" / name).is_file() for name in files):
    subprocess.run([str(compiler.DOTNET / "dotnet"), str(executable), str(ROOT / "tests/fixtures")], check=True)
    print(f"Generated {len(files)} owned metadata images; compiler={compile_ms} ms")
else:
    print("Owned core metadata fixtures cached; no compiler or generator execution")
