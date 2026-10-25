#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Optional static oracle checks using owned fixtures and the installed .NET 10 reader.

Run inventory.py once to build ClassInventory.dll. No input assembly is executed,
loaded for reflection, acquired or rehashed. Invalid bytes use a temporary copy.
"""
import importlib.util
import json
from pathlib import Path
import os
import struct
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("inventory", ROOT / "acceptance/openra/inventory.py")
inventory = importlib.util.module_from_spec(spec)
spec.loader.exec_module(inventory)
DOTNET = Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10")) / "dotnet"
ORACLE = ROOT / "report/openra/class-inventory/ClassInventory.dll"
CLI = ROOT / "target/release/ilspy-lite"


def snapshot(assembly, name):
    result = subprocess.run([str(DOTNET), str(ORACLE), str(assembly), name], text=True, capture_output=True)
    if result.returncode != 0:
        raise AssertionError(f"Static metadata oracle failed for {name}: {result.stderr}")
    return json.loads(result.stdout)


for config in ("release", "debug"):
    assembly = ROOT / f"tests/fixtures/class-{config}.bin"
    for name in ("ClassCases.MarshalMethods", "ClassCases.LayoutFields", "ClassCases.Mixed"):
        original = snapshot(assembly, name)
        actual = subprocess.run([str(CLI), "decompile", str(assembly), "--type", name], text=True, capture_output=True)
        inventory.check_output(original, actual.stdout, actual.stderr, actual.returncode)
        print(f"{config} {name}: field/parameter/return marshal output matches independent oracle")
    original = snapshot(assembly, "ClassCases.MarshalMethods")
    row = bytes.fromhex(original["types"][0]["marshalling"][0]["raw_row"])
    image = bytearray(assembly.read_bytes())
    assert len(row) == 4
    positions = [i for i in range(len(image) - len(row) + 1) if image[i:i + len(row)] == row]
    assert len(positions) == 1, "owned marshal row needs an unambiguous fixture offset"
    struct.pack_into("<H", image, positions[0] + 2, 0xffff)
    with tempfile.TemporaryDirectory() as directory:
        patched = Path(directory) / "unrelated-invalid-marshal.bin"
        patched.write_bytes(image)
        for name in ("ClassCases.LayoutFields", "ClassCases.Mixed"):
            expected = snapshot(assembly, name)
            actual = snapshot(patched, name)
            assert actual == expected, "unrelated invalid blob must not affect selected static snapshot"
            print(f"{config} {name}: unrelated invalid marshal blob excluded before heap decoding")
