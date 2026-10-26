#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Build owned ordinary constrained-call fixtures without package restore."""
from pathlib import Path
import os
import re
import struct
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
DOTNET = Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10"))
def latest(path):
    return max((p for p in path.iterdir() if p.name.startswith("10.")),
               key=lambda p: tuple(map(int, p.name.split("."))))
sdk = latest(DOTNET / "sdk")
refs = latest(DOTNET / "packs/Microsoft.NETCore.App.Ref") / "ref/net10.0"
for config in ("release", "debug"):
    output = ROOT / f"tests/fixtures/constrained-{config}.bin"
    with tempfile.TemporaryDirectory() as directory:
        response = Path(directory) / "compile.rsp"
        response.write_text("\n".join([
            "-nologo", "-target:library", "-langversion:14", "-warnaserror+", "-deterministic+",
            "-optimize+" if config == "release" else "-optimize-", f'-out:"{output}"',
            *[f'-r:"{p}"' for p in sorted(refs.glob("*.dll"))],
            f'"{ROOT / "tests/dotnet/ConstrainedCases.cs"}"',
        ]))
        subprocess.run([str(DOTNET / "dotnet"), str(sdk / "Roslyn/bincore/csc.dll"), "@" + str(response)], check=True)
    cli = ROOT / "target/debug/ilspy-lite"
    listing = subprocess.run([str(cli), "list", str(output)], check=True, capture_output=True, text=True).stdout
    tokens = {line.split("::", 1)[1].split("(", 1)[0]: line.split()[0] for line in listing.splitlines()}
    def il(name):
        return subprocess.run([str(cli), "il", str(output), "--method", tokens[name]], check=True, capture_output=True, text=True).stdout
    element = int(re.search(r"ldelema (0x[0-9A-F]{8})", il("ArrayAnchor"))[1], 16)
    original = bytes(int(byte, 16) for chunk in re.findall(r"^\.code-bytes IL_[0-9A-F]+: \[([^]]*)\]", il("ArrayCarry"), re.M) for byte in chunk.split(", "))
    # An acquired element pointer stays on the evaluation stack across both
    # branch edges; the selected path stores through that original location.
    code = b"\x02\x03\x8f" + struct.pack("<I", element) + b"\x04\x2d\x04\x1f\x11\x2b\x02\x1f\x1d\x54\x02\x03\x95\x2a"
    image = bytearray(output.read_bytes())
    start = image.find(original)
    assert start > 0 and image.find(original, start + 1) == -1 and len(code) <= len(original)
    if image[start-1] & 3 == 2:
        assert image[start-1] >> 2 == len(original) and len(code) < 64
        image[start-1] = (len(code) << 2) | 2
    else:
        assert struct.unpack_from("<H", image, start-12)[0] & 3 == 3
        struct.pack_into("<H", image, start-10, 8)
        struct.pack_into("<I", image, start-8, len(code))
    image[start:start+len(code)] = code
    output.write_bytes(image)
