#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Inspect the pinned OpenRA corpus through the public CLI; never execute it."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

HERE = Path(__file__).resolve().parent
RUST = HERE.parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def checked_file(path, expected):
    if digest(path) != expected:
        raise ValueError(f"SHA-256 mismatch: {path}")


def run(cli, *args):
    result = subprocess.run([str(cli), *map(str, args)], capture_output=True,
                            text=True, timeout=30)
    if result.returncode not in (0, 1, 2):
        raise RuntimeError(f"CLI terminated unexpectedly: {result.returncode}")
    return result


def inspect(args):
    manifest = json.loads((HERE / "manifest.json").read_text())
    inventories = {}
    # Fail before probing when artifacts are missing, modified or from another release.
    for name, entry in manifest["assemblies"].items():
        path = args.root / "assemblies" / name
        checked_file(path, entry["sha256"])
        result = run(args.cli, "list", path)
        if result.returncode or result.stderr.strip():
            raise RuntimeError(f"Inventory failed for {name}: {result.stderr}")
        lines = [line.split(" ", 1) for line in result.stdout.splitlines()]
        inventory = dict(lines)
        if len(lines) != len(inventory) or len(lines) != entry["method_count"]:
            raise ValueError(f"Method inventory mismatch: {name}")
        inventories[name] = inventory
    for path, expected in manifest["sources"].items():
        checked_file(args.root / "source" / path, expected)

    entries = manifest["methods"] + manifest["support_methods"] + manifest["fallback_sentinels"]
    results = []
    args.output.mkdir(parents=True, exist_ok=True)
    for entry in entries:
        name, token = entry["assembly"], entry["token"]
        if inventories[name].get(token) != entry["signature"]:
            raise ValueError(f"Token/signature mismatch: {entry['id']}")
        path = args.root / "assemblies" / name
        il = run(args.cli, "il", path, "--method", token)
        if il.returncode or il.stderr.strip():
            raise RuntimeError(f"IL decoding failed: {entry['id']}: {il.stderr}")
        instructions = [line for line in il.stdout.splitlines() if line.startswith("IL_")]
        if not instructions or ".code-bytes " not in il.stdout:
            raise ValueError(f"Missing body evidence: {entry['id']}")
        if "instruction_count" in entry and len(instructions) != entry["instruction_count"]:
            raise ValueError(f"Instruction count mismatch: {entry['id']}")
        for marker in entry.get("required_il", []):
            if marker not in il.stdout:
                raise ValueError(f"Missing IL evidence {marker}: {entry['id']}")
        result = run(args.cli, "decompile", path, "--method", token, "--details")
        summary = re.fullmatch(r"decompiled=(\d+) declarations=(\d+) partial=(\d+) failed=(\d+)\s*",
                               result.stderr)
        if not summary:
            raise ValueError(f"Missing/unexpected CLI diagnostics: {entry['id']}: {result.stderr}")
        counts = tuple(map(int, summary.groups()))
        statuses = {(1, 0, 0, 0): ("decompiled", 0), (0, 1, 0, 0): ("declaration", 0),
                    (0, 0, 1, 0): ("partial", 2), (0, 0, 0, 1): ("failed", 1)}
        if counts not in statuses or result.returncode != statuses[counts][1]:
            raise ValueError(f"Inconsistent status/exit: {entry['id']}")
        status = statuses[counts][0]
        if status == "declaration":
            raise ValueError(f"An IL body was misclassified as a declaration: {entry['id']}")
        if token not in result.stdout:
            raise ValueError(f"Output lost method identity: {entry['id']}")
        if status == "partial":
            if "// IL FALLBACK:" not in result.stdout:
                raise ValueError(f"Unmarked fallback: {entry['id']}")
            for line in il.stdout.splitlines():
                if f"// {line}\n" not in result.stdout:
                    raise ValueError(f"Fallback lost IL evidence: {entry['id']}")
        if status == "decompiled" and ("IL FALLBACK" in result.stdout or "// FAILED" in result.stdout):
            raise ValueError(f"Fallback counted as success: {entry['id']}")
        (args.output / f"{entry['id']}.il.txt").write_text(il.stdout)
        (args.output / f"{entry['id']}.cs.txt").write_text(result.stdout)
        (args.output / f"{entry['id']}.stderr.txt").write_text(result.stderr)
        opcodes = sorted({line.split(": ", 1)[1].split()[0] for line in instructions})
        reasons = [line for line in result.stdout.splitlines() if "// IL FALLBACK:" in line or "// FAILED" in line]
        results.append({"id": entry["id"], "stage": entry.get("stage"), "status": status,
                        "instructions": len(instructions), "opcodes": opcodes, "diagnostics": reasons})
        print(f"{entry['id']}: {status} ({len(instructions)} instructions)")
    report = {"release": manifest["release"], "commit": manifest["commit"],
              "cli_sha256": digest(args.cli), "assemblies": manifest["assemblies"],
              "results": results, "behavioral_validation": "not performed"}
    (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    if any(r["status"] == "failed" for r in results):
        return 1
    required = [r for r in results if r["stage"] is not None and r["stage"] <= args.require_stage]
    if any(r["status"] != "decompiled" for r in required):
        print("Requested stage is NOT met; IL fallback is not decompilation success.", file=sys.stderr)
        return 2
    print("Inspection complete. This is not a behavioral-equivalence gate.")
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=RUST / "report/openra")
    parser.add_argument("--cli", type=Path, default=RUST / "target/release/ilspy-lite")
    parser.add_argument("--output", type=Path, default=RUST / "report/openra/current")
    parser.add_argument("--require-stage", type=int, choices=(0, 1, 2, 3), default=0,
                        help="0 records coverage; 1..3 requires that stage and all earlier stages, including support methods")
    args = parser.parse_args()
    try:
        return inspect(args)
    except (OSError, ValueError, RuntimeError, subprocess.TimeoutExpired) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
