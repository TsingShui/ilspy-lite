#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Report the integer per-assembly gate for a saved corpus baseline.

A body image passes iff ``100 * unrecovered <= bodies``. Unrecovered is every
result other than ``Decompiled`` plus any body the image did not account for.
Bodyless images have no recovery rate and neither pass nor fail the gate.
Exit 0 when every body image passes, 2 when the scan is complete but some image
is above 1%, and 1 when the scan is incomplete or malformed.
"""
import argparse
import json
import sys
from pathlib import Path


def gate_image(image):
    bodies = int(image.get("bodies", 0))
    counts = image.get("counts") or {}
    recovered = int(counts.get("Decompiled", 0))
    unrecovered = sum(int(value) for key, value in counts.items() if key != "Decompiled")
    accounted = recovered + unrecovered
    if accounted < bodies:
        unrecovered += bodies - accounted
    complete = bool(image.get("scan_complete")) and accounted == bodies
    return {
        "id": image.get("id"),
        "assembly": image.get("assembly"),
        "origins": image.get("origins", []),
        "bodies": bodies,
        "unrecovered": unrecovered,
        "rate": (100.0 * unrecovered / bodies) if bodies else 0.0,
        "complete": complete,
        "pass": complete and 100 * unrecovered <= bodies,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path, help="baseline.json from corpus.py baseline")
    parser.add_argument("--limit", type=int, default=20, help="failing rows to print (default 20)")
    parser.add_argument("--json", action="store_true", help="print the complete JSON decision")
    args = parser.parse_args()

    report = json.loads(args.baseline.read_text())
    images = report.get("images") or []
    decisions = [gate_image(image) for image in images]
    body = [decision for decision in decisions if decision["bodies"] > 0]
    passed = [decision for decision in body if decision["pass"]]
    complete = bool(report.get("complete")) and all(decision["complete"] for decision in decisions)
    summary = {
        "body_images": len(body),
        "passing": len(passed),
        "failing": len(body) - len(passed),
        "unrecovered": sum(decision["unrecovered"] for decision in body),
        "bodies": sum(decision["bodies"] for decision in body),
        "complete": complete,
        "pass": complete and len(passed) == len(body),
        "images": decisions,
    }

    if args.json:
        print(json.dumps(summary, indent=1))
    else:
        print(f"body_images={summary['body_images']} passing={summary['passing']} "
              f"failing={summary['failing']} unrecovered={summary['unrecovered']} "
              f"bodies={summary['bodies']} complete={summary['complete']}")
        failing = sorted((decision for decision in body if not decision["pass"]),
                         key=lambda decision: (-decision["rate"], decision["unrecovered"]))
        for decision in failing[: max(0, args.limit)]:
            origins = ", ".join(decision["origins"])
            print(f"{decision['rate']:9.3f}% {decision['unrecovered']:7d}/{decision['bodies']:<7d} "
                  f"{decision['assembly'] or '?'} [{origins}]")

    if not complete:
        return 1
    return 0 if summary["pass"] else 2


if __name__ == "__main__":
    sys.exit(main())
