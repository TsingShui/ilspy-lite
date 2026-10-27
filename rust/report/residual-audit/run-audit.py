#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Run the instrumented production CLI over residual-bucket images and collect audit lines.

Temporary audit tooling: the instrumented build logs the exact sub-cause at the
gate that produced a body's stated first blocker. Output is one JSON file of
aggregated counts plus the raw lines for the sampled methods.
"""
import argparse
import collections
import json
import os
import subprocess
import sys
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def run_one(payload):
    ident, path, cli, audit = payload
    env = dict(os.environ)
    env["ILSPY_LITE_AUDIT"] = "1"
    process = subprocess.run([str(cli), "scan", path, "--details"], text=True,
                             capture_output=True, env=env, timeout=900)
    lines = [line for line in process.stderr.splitlines()
             if line.startswith("AUDIT\t") or line.startswith(" -> ")]
    return ident, process.returncode, lines


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path,
                        default=ROOT / "report/foreign-hidden-batch/corpus-scan-fixed/baseline.json")
    parser.add_argument("--inventory", type=Path,
                        default=ROOT / "report/fallback-corpus/inventory-final/inventory.json")
    parser.add_argument("--cli", type=Path, default=ROOT / "target/release/ilspy-lite")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--workers", type=int, default=6)
    parser.add_argument("--reasons", nargs="+", required=True)
    args = parser.parse_args()
    baseline = json.loads(args.baseline.read_text())
    paths = {i["id"]: i["path"] for i in json.loads(args.inventory.read_text())["images"]}
    selected = []
    for image in baseline["images"]:
        if any(image["reasons"].get(reason) for reason in args.reasons):
            selected.append((image["id"], paths[image["id"]]))
    args.output.mkdir(parents=True, exist_ok=True)
    totals = collections.Counter()
    with ProcessPoolExecutor(max_workers=args.workers) as pool:
        for ident, code, lines in pool.map(run_one,
                                           [(i, p, args.cli, True) for i, p in selected]):
            (args.output / f"{ident:04d}.audit.log").write_text("\n".join(lines) + "\n")
            tags = collections.Counter(line.split("\t")[1] for line in lines
                                       if line.startswith("AUDIT\t"))
            for tag, count in tags.items():
                totals[f"{ident}:{tag}"] = count
            print(f"{ident}: exit={code} " + " ".join(f"{t}={c}" for t, c in sorted(tags.items())),
                  flush=True)
    (args.output / "index.json").write_text(json.dumps({"images": [i for i, _ in selected],
                                                        "counts": dict(totals)}, indent=2) + "\n")
    print(json.dumps({"images": len(selected), "lines": sum(totals.values())}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
