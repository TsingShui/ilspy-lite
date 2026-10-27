#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Restore the committed per-method baseline anchor with local result paths.

`baseline-anchor-728704.zip` stores the 728,704/42,044 scan as one JSON file per
image plus the report header. Absolute paths from the machine that produced it
are not portable, so this rewrites every `results_path` to the extracted file.
"""
import argparse
import json
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_ZIP = ROOT / "report/fallback-corpus/baseline-anchor-728704.zip"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--zip", type=Path, default=DEFAULT_ZIP)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(args.zip) as archive:
        header = json.loads(archive.read("baseline.json"))
        for name in archive.namelist():
            if name == "baseline.json":
                continue
            target = args.output / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(archive.read(name))
    for image in header["images"]:
        image["results_path"] = str(args.output / "images" / f"{image['id']:04d}.json")
    (args.output / "baseline.json").write_text(json.dumps(header, indent=2) + "\n")
    print(json.dumps({"images": len(header["images"]), "path": str(args.output / "baseline.json")}))


if __name__ == "__main__":
    main()
