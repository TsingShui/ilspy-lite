#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Optional owned static import oracle regression: wide string indexes, small heap.

The authored metadata uses the table HeapSizes encoding, not heap length, to
select index widths. No input code or native library is executed.
"""
import importlib.util
import json
import os
from pathlib import Path
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


def authored_image(template):
    strings = bytearray(b"\0")
    def name(value):
        offset = len(strings)
        strings.extend(value.encode() + b"\0")
        return offset
    module, empty, owner, method, entry, native = [name(s) for s in ("owned", "Empty", "NativeOwner", "Read", "entry", "native")]
    counts = [(0, 1), (2, 2), (6, 1), (26, 1), (28, 1)]
    valid = sum(1 << table for table, _ in counts)
    tables = bytearray(struct.pack("<IBBBBQQ", 0, 2, 0, 1, 1, valid, 0))
    tables.extend(b"".join(struct.pack("<I", count) for _, count in counts))
    tables.extend(struct.pack("<HIHHH", 0, module, 1, 0, 0))
    for type_name in (empty, owner):
        tables.extend(struct.pack("<IIIHHH", 0x00100181, type_name, 0, 0, 1, 1))
    tables.extend(struct.pack("<IHHIHH", 0, 0x80, 0x2096, method, 1, 1))
    tables.extend(struct.pack("<I", native))
    tables.extend(struct.pack("<HHIH", 0x100, 3, entry, 1))
    streams = [(b"#~", tables), (b"#Strings", strings), (b"#Blob", b"\0\x03\0\0\x08"), (b"#GUID", bytes(range(16)))]
    metadata = bytearray(struct.pack("<IHHII", 0x424a5342, 1, 1, 0, 12) + b"v4.0.30319\0\0" + struct.pack("<HH", 0, len(streams)))
    headers = []
    for stream_name, data in streams:
        headers.append(len(metadata))
        metadata.extend(struct.pack("<II", 0, len(data)) + stream_name + b"\0")
        metadata.extend(bytes((-len(metadata)) % 4))
    for (_, data), header in zip(streams, headers):
        metadata.extend(bytes((-len(metadata)) % 4))
        struct.pack_into("<I", metadata, header, len(metadata))
        metadata.extend(data)
    image = bytearray(template.read_bytes())
    pe = struct.unpack_from("<I", image, 0x3c)[0]
    sections = struct.unpack_from("<H", image, pe + 6)[0]
    optional = pe + 24
    section_table = optional + struct.unpack_from("<H", image, pe + 20)[0]
    directory = optional + (112 if struct.unpack_from("<H", image, optional)[0] == 0x20b else 96)
    cli_rva = struct.unpack_from("<I", image, directory + 14 * 8)[0]
    cli = None
    for i in range(sections):
        section = section_table + i * 40
        size, rva, raw_size, raw = struct.unpack_from("<IIII", image, section + 8)
        if rva <= cli_rva < rva + max(size, raw_size):
            cli = raw + cli_rva - rva
    assert cli is not None
    last = section_table + (sections - 1) * 40
    rva = struct.unpack_from("<I", image, last + 12)[0]
    raw = struct.unpack_from("<I", image, last + 20)[0]
    image.extend(bytes((-len(image)) % 512))
    start = len(image)
    image.extend(metadata)
    image.extend(bytes((-len(image)) % 512))
    size = len(image) - raw
    struct.pack_into("<I", image, last + 8, size)
    struct.pack_into("<I", image, last + 16, size)
    alignment = struct.unpack_from("<I", image, optional + 32)[0]
    struct.pack_into("<I", image, optional + 56, ((rva + size + alignment - 1) // alignment) * alignment)
    struct.pack_into("<II", image, cli + 8, rva + start - raw, len(metadata))
    return image


for config in ("release", "debug"):
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "wide-string-import.bin"
        path.write_bytes(authored_image(ROOT / f"tests/fixtures/class-{config}.bin"))
        oracle = subprocess.run([str(DOTNET), str(ORACLE), str(path), "NativeOwner"], text=True, capture_output=True)
        assert oracle.returncode == 0, oracle.stderr
        snapshot = json.loads(oracle.stdout)
        imports = snapshot["types"][0]["imports"]
        assert len(imports) == 1 and imports[0]["name"] == "entry" and imports[0]["module"] == "native", imports
        assert len(bytes.fromhex(imports[0]["raw_row"])) == 10
        actual = subprocess.run([str(CLI), "decompile", str(path), "--type", "NativeOwner"], text=True, capture_output=True)
        inventory.check_output(snapshot, actual.stdout, actual.stderr, actual.returncode)
        print(f"{config}: small heap with encoded wide string indexes preserves exact native import/module evidence")
