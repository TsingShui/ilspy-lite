#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Report what a static constructor body still needs, per image.

A bounded probe of three named images, not the acceptance scan: it answers "what
does a `.cctor` still fail on now that the blanket refusal is gone". Every covered
image is scanned with the CLI's default reference discovery, so the rows are
comparable to what the acceptance run would record for the same image.
"""
import argparse
import json
import subprocess
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def probe(cli, image):
    listing = subprocess.run([str(cli), "list", str(image)], capture_output=True, text=True, check=True).stdout
    cctors = {}
    for line in listing.splitlines():
        token, signature = line.split(" ", 1)
        if "::.cctor(" in signature:
            cctors[token] = signature
    scan = subprocess.run([str(cli), "scan", str(image)], capture_output=True, text=True, check=True).stdout
    rows = [json.loads(line) for line in scan.splitlines()]
    reasons, recovered = Counter(), 0
    for row in rows:
        if row["token"] not in cctors:
            continue
        if row["status"] == "Decompiled":
            recovered += 1
        else:
            reasons[row["reason"] or row["status"]] += 1
    return {
        "image": str(image),
        "methods": len(rows),
        "cctors": len(cctors),
        "cctors_recovered": recovered,
        "cctors_remaining": len(cctors) - recovered,
        "reasons": dict(sorted(reasons.items(), key=lambda item: (-item[1], item[0]))),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target-worker/release/ilspy-lite")
    parser.add_argument("--output", type=Path, default=ROOT / "report/static-constructor-roundtrip/cctor-probe.json")
    parser.add_argument("images", nargs="+", type=Path)
    args = parser.parse_args()
    report = [probe(args.cli, image) for image in args.images]
    total = Counter()
    for entry in report:
        total.update(entry["reasons"])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps({"images": report, "combined_reasons": dict(total.most_common())}, indent=2) + "\n")
    for entry in report:
        print(f"{Path(entry['image']).name}: {entry['cctors_recovered']}/{entry['cctors']} static constructors recovered")
        for reason, count in entry["reasons"].items():
            print(f"    {count:5d}  {reason}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
