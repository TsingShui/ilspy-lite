#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Reuse the independent static collector for the separate fixed naming samples."""
import argparse
import json
import os
from pathlib import Path
import subprocess

from validate_random_classes import HERE, compile_oracle, inspect_class, write_json


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", required=True, type=Path)
    parser.add_argument("--assemblies", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--oracle", type=Path)
    parser.add_argument("--dotnet", type=Path, default=Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10")) / "dotnet")
    args = parser.parse_args()
    for key in ("cli", "assemblies", "output", "dotnet"):
        setattr(args, key, getattr(args, key).absolute())
    args.reference, args.reference_dir = [], []
    args.output.mkdir(parents=True, exist_ok=False)
    frozen = json.loads((HERE / "random-classes.json").read_text())
    samples = json.loads((HERE / "naming-samples.json").read_text())
    report = {"schema": 1, "scope": "Supplemental static evidence only; not the frozen twenty or runtime proof",
              "noRuntimeProof": True, "manifest": samples, "classes": [], "errors": []}
    head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=HERE, text=True, capture_output=True)
    report["collector_checkout_head"] = head.stdout.strip() if head.returncode == 0 else None
    try:
        selections = samples["classes"]
        if len(selections) != 3 or len({(e["assembly"], e["mvid"], e["token"]) for e in selections}) != 3:
            raise ValueError("Expected all three fixed supplemental identities; no resampling")
        oracle = compile_oracle(args)
        for index, entry in enumerate(selections, 1):
            try:
                row = inspect_class(args, oracle, frozen, entry, index)
            except (OSError, ValueError, KeyError, TypeError) as error:
                row = {"selection": entry, "static_status": "failed", "errors": [str(error)], "path": None}
            report["classes"].append(row)
            print(f"{entry['token']} {entry['name']}: {row['static_status']}")
    except (OSError, ValueError, KeyError, TypeError) as error:
        report["errors"].append(str(error))
    for entry in samples["classes"][len(report["classes"]):]:
        report["classes"].append({"selection": entry, "path": None, "static_status": "failed",
                                  "errors": ["Not collected because setup failed"], "noRuntimeProof": True})
    report["static_pass_count"] = sum(row["static_status"] == "passed" for row in report["classes"])
    write_json(args.output / "report.json", report)
    print(f"Static checks: {report['static_pass_count']}/3; no runtime proof. Evidence: {args.output / 'report.json'}")
    return 0 if report["static_pass_count"] == 3 and not report["errors"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
