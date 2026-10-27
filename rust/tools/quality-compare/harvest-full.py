#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Harvest per-class quality samples for every body-bearing image in the corpus.

For each image it picks the classes with the most decompiled method bodies, runs
ilspy-lite and ILSpy on each of them, and writes a bounded bundle (one JSON per
class holding both outputs, truncated to a per-class byte budget) plus an index.
Nothing is executed: both tools only read the assembly.

The bundle is the input for parallel review agents; it is deliberately
self-contained so a reviewer never needs the corpus or a .NET toolchain.
"""
import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CLI = ROOT / "rust/target/release/ilspy-lite"
ILSPY = ROOT / "ICSharpCode.ILSpyCmd/bin/Release/net10.0/ilspycmd.dll"
DOTNET = Path.home() / ".dotnet10/dotnet"
BUNDLE = ROOT / "rust/report/quality-full/bundle"


def run(command, timeout=900):
    return subprocess.run(command, capture_output=True, text=True, timeout=timeout)


def pick_classes(image_id, inventory, count):
    methods = json.loads(Path(inventory).read_text())["methods"]
    bodies = {}
    for method in methods:
        if method["body_kind"] != "cil":
            continue
        bodies[method["owner"]] = bodies.get(method["owner"], 0) + 1
    ranked = sorted(bodies.items(), key=lambda kv: (-kv[1], kv[0]))
    keep = []
    for owner, methods_in_class in ranked:
        if owner.startswith("<") or "AnonymousType" in owner:
            continue
        keep.append({"type": owner, "bodies": methods_in_class})
        if len(keep) == count:
            break
    return keep


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--images", type=Path,
                        default=ROOT / "rust/report/quality-full/images.json")
    parser.add_argument("--inventory", type=Path,
                        default=ROOT / "rust/report/fallback-corpus/inventory-final/inventory.json")
    parser.add_argument("--bundle", type=Path, default=BUNDLE)
    parser.add_argument("--classes", type=int, default=6)
    parser.add_argument("--bytes-per-class", type=int, default=120_000)
    parser.add_argument("--only", nargs="*", type=int)
    args = parser.parse_args()
    inventory = json.loads(args.inventory.read_text())
    snapshots = {i["id"]: i["inventory_path"] for i in inventory["images"]}
    images = json.loads(args.images.read_text())
    if args.only:
        images = [i for i in images if i["id"] in set(args.only)]
    args.bundle.mkdir(parents=True, exist_ok=True)
    index = []
    for image in images:
        classes = pick_classes(image["id"], snapshots[image["id"]], args.classes)
        folder = args.bundle / f"{image['id']:04d}"
        folder.mkdir(parents=True, exist_ok=True)
        reference_dirs = sorted({str(Path(image["path"]).parent)})
        ours_refs = [arg for d in reference_dirs for arg in ("--reference-dir", d)]
        ilspy_refs = [arg for d in reference_dirs for arg in ("-r", d)]
        rows = []
        for entry in classes:
            name = entry["type"]
            stem = re.sub(r"[^A-Za-z0-9_.]+", "_", name)
            ours = run([str(CLI), "decompile", image["path"], "--type", name, *ours_refs])
            theirs = run([str(DOTNET), str(ILSPY), "-t", name, *ilspy_refs, image["path"]])
            ours_text = ours.stdout[: args.bytes_per_class]
            theirs_text = theirs.stdout[: args.bytes_per_class]
            payload = {
                "image": image["assembly"], "image_id": image["id"],
                "type": name, "bodies": entry["bodies"],
                "ours_status": ours.returncode, "ilspy_status": theirs.returncode,
                "ours_truncated": len(ours.stdout) > len(ours_text),
                "ilspy_truncated": len(theirs.stdout) > len(theirs_text),
                "ours": ours_text, "ilspy": theirs_text,
            }
            (folder / f"{stem}.json").write_text(json.dumps(payload, indent=1))
            rows.append({"type": name, "file": f"{image['id']:04d}/{stem}.json",
                         "bodies": entry["bodies"], "ours_bytes": len(ours.stdout),
                         "ilspy_bytes": len(theirs.stdout),
                         "ours_status": ours.returncode, "ilspy_status": theirs.returncode})
        index.append({"id": image["id"], "assembly": image["assembly"],
                      "decompiled": image["decompiled"], "classes": rows})
        print(f"{image['id']:04d} {image['assembly'][:34]:36s} classes={len(rows)}", flush=True)
    (args.bundle / "index.json").write_text(json.dumps(index, indent=1) + "\n")
    print(json.dumps({"images": len(index), "classes": sum(len(i["classes"]) for i in index)}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
