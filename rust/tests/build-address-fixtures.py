#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Build owned address fixtures, then replace reserved bodies with authored IL.

Uses installed .NET 10 Roslyn and the existing IL decoder. No restore, external
input execution or hashing. Anchors supply core element TypeRefs, not method logic.
"""
import os
from pathlib import Path
import re
import struct
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
dotnet = Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10"))
def latest(path):
    return max((p for p in path.iterdir() if p.name.startswith("10.")),
               key=lambda p: tuple(map(int, p.name.split("."))))
sdk = latest(dotnet / "sdk")
ref = latest(dotnet / "packs/Microsoft.NETCore.App.Ref") / "ref/net10.0"
subprocess.run(["cargo", "build", "--locked"], cwd=ROOT, check=True)
cli = ROOT / "target/debug/ilspy-lite"
for config in ("release", "debug"):
    fixture = ROOT / f"tests/fixtures/address-{config}.bin"
    with tempfile.TemporaryDirectory() as directory:
        response = Path(directory) / "compile.rsp"
        response.write_text("\n".join([
            "-nologo", "-target:library", "-langversion:14", "-warnaserror+", "-deterministic+",
            "-optimize+" if config == "release" else "-optimize-",
            f'-out:"{fixture}"', *[f'-r:"{p}"' for p in sorted(ref.glob("*.dll"))],
            f'"{ROOT / "tests/dotnet/AddressCases.cs"}"',
        ]))
        subprocess.run([str(dotnet / "dotnet"), str(sdk / "Roslyn/bincore/csc.dll"), f"@{response}"], check=True)
    listing = subprocess.run([str(cli), "list", str(fixture)], check=True, capture_output=True, text=True).stdout
    tokens = {line.split("::", 1)[1].split("(", 1)[0]: line.split()[0] for line in listing.splitlines()}
    def il(name):
        return subprocess.run([str(cli), "il", str(fixture), "--method", tokens[name]],
                              check=True, capture_output=True, text=True).stdout
    u4 = int(re.search(r"ldelema (0x[0-9A-F]{8})", il("AnchorUnsigned"))[1], 16)
    i4 = int(re.search(r"ldelema (0x[0-9A-F]{8})", il("AnchorSigned"))[1], 16)
    effects = int(re.search(r"stfld (0x[0-9A-F]{8})", il("Take"))[1], 16)
    image = bytearray(fixture.read_bytes())
    pack = lambda token: struct.pack("<I", token)
    prefix = b"\x02\x17\x7d" + pack(effects)  # this.Effects = 1
    finish = b"\x02\x18\x7d" + pack(effects)  # this.Effects = 2
    unsigned = b"\x03\x04\x8f" + pack(u4)
    bodies = {
        "Take": prefix + unsigned + b"\x26" + finish + b"\x2a",
        "TakeSigned": prefix + b"\x03\x04\x8f" + pack(i4) + b"\x26" + finish + b"\x2a",
        "Duplicate": prefix + unsigned + b"\x25\x02\x28" + pack(int(tokens["Bump"], 16)) + b"\x26\x26\x26\x2a",
    }
    declarations = subprocess.run([str(cli), "decompile", str(fixture), "--type", "AddressCases.Cells"],
                                  capture_output=True, text=True, timeout=30)
    assert declarations.returncode in (0, 2), declarations.stderr
    values = int(re.search(r"^\.field (0x[0-9A-F]{8}) .*@Values ", declarations.stdout, re.M)[1], 16)
    bodies["Retarget"] = (prefix + b"\x02\x7b" + pack(values) + b"\x04\x8f" + pack(u4)
                          + b"\x02\x03\x7d" + pack(values) + finish + b"\x26\x2a")
    for name, code in bodies.items():
        original = bytes(int(byte, 16) for chunk in re.findall(r"^\.code-bytes IL_[0-9A-F]+: \[([^]]*)\]", il(name), re.M)
                         for byte in chunk.split(", "))
        assert original and len(code) <= len(original)
        start = image.find(original)
        assert start > 0 and image.find(original, start + 1) == -1, name
        header = image[start - 1]
        assert header & 3 == 2 and header >> 2 == len(original), name
        image[start - 1] = (len(code) << 2) | 2
        image[start:start + len(code)] = code
    fixture.write_bytes(image)
    print(f"{config}: reserved bodies replaced with authored ldelema IL")
