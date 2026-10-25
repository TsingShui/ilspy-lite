#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Check the frozen class/member inventory using an independent .NET 10 reader.

This is static inspection, not a lowering or behavioral gate. Inputs and their
recorded provenance are reused; generated metadata stays in the ignored report.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess

RUST = Path(__file__).resolve().parents[2]


def identities(snapshot):
    result = {"root": snapshot["root"], "mvid": snapshot["mvid"]}
    result["types"] = [t["token"] for t in snapshot["types"]]
    for kind in ("fields", "methods", "properties", "events"):
        result[kind] = [member["token"] for t in snapshot["types"] for member in t[kind]]
    for kind in ("types", "fields", "methods", "properties", "events"):
        if len(result[kind]) != len(set(result[kind])):
            raise ValueError(f"Duplicate {kind} identity in metadata snapshot")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=RUST / "report/openra")
    parser.add_argument("--output", type=Path, default=RUST / "report/openra/class-inventory")
    args = parser.parse_args()
    dotnet = Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10"))

    def latest(path):
        return max((p for p in path.iterdir() if p.name.startswith("10.")),
                   key=lambda p: tuple(map(int, p.name.split("."))))

    sdk = latest(dotnet / "sdk")
    ref = latest(dotnet / "packs/Microsoft.NETCore.App.Ref") / "ref/net10.0"
    runtime = latest(dotnet / "shared/Microsoft.NETCore.App")
    args.output.mkdir(parents=True, exist_ok=True)
    oracle = args.output / "ClassInventory.dll"
    response = oracle.with_suffix(".rsp")
    response.write_text("\n".join([
        "-nologo", "-target:exe", "-langversion:14", "-warnaserror+",
        f'-out:"{oracle}"', *[f'-r:"{p}"' for p in sorted(ref.glob("*.dll"))],
        f'"{RUST / "tests/dotnet/ClassInventory.cs"}"',
    ]))
    oracle.with_suffix(".runtimeconfig.json").write_text(json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    subprocess.run([str(dotnet / "dotnet"), str(sdk / "Roslyn/bincore/csc.dll"), f"@{response}"],
                   check=True, timeout=120)
    manifest = json.loads((RUST / "acceptance/openra/classes.json").read_text())
    for entry in manifest["classes"]:
        result = subprocess.run([str(dotnet / "dotnet"), str(oracle),
                                 str(args.root / "assemblies" / entry["assembly"]), entry["name"]],
                                check=True, text=True, capture_output=True, timeout=30)
        snapshot = json.loads(result.stdout)
        actual = identities(snapshot)
        (args.output / f"{entry['id']}.json").write_text(result.stdout)
        if entry.get("frozen_inventory") != actual:
            raise ValueError(f"Frozen member inventory mismatch: {entry['name']}")
        if not set(entry["required_methods"]).issubset(actual["methods"]):
            raise ValueError(f"Required methods missing: {entry['name']}")
        print(f"{entry['name']}: frozen class/member inventory matches; no method executed")


if __name__ == "__main__":
    main()
