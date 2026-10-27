#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Rescan selected fixed-corpus images with the production CLI and diff statuses.

This reuses the acceptance scanner so a targeted scan has exactly the reference
policy, thread count and per-method accounting of a complete scan. It writes a
mini baseline in the same schema plus a per-image delta against a saved scan.
"""
import argparse
import importlib.util
import json
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def load(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


corpus = load(ROOT / "acceptance/corpus/corpus.py", "corpus_scan")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", type=Path,
                        default=ROOT / "report/fallback-corpus/inventory-final/inventory.json")
    parser.add_argument("--before", type=Path,
                        default=ROOT / "report/foreign-hidden-batch/corpus-scan-fixed/baseline.json")
    parser.add_argument("--cli", type=Path, default=ROOT / "target/release/ilspy-lite")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--timeout", type=int, default=900)
    parser.add_argument("--threads", type=int, default=1)
    parser.add_argument("ids", nargs="+", type=int)
    args = parser.parse_args()
    if not args.cli.is_file():
        parser.error("existing production CLI required")
    before = json.loads(args.before.read_text())
    assert before["complete"], "comparison scan is incomplete"
    inventory = json.loads(args.inventory.read_text())
    prior = {i["id"]: i for i in before["images"]}
    images = {i["id"]: i for i in inventory["images"]}
    stamp = {"path": str(args.cli), "mtime_ns": args.cli.stat().st_mtime_ns,
             "bytes": args.cli.stat().st_size}
    args.output.mkdir(parents=True, exist_ok=True)
    started = time.monotonic()
    results, rows = [], []
    for ident in args.ids:
        source = images[ident]
        image = {"id": source["id"], "assembly": source["assembly"], "kind": source["kind"],
                 "path": source["path"], "origins": source["origins"],
                 "sha256": source.get("sha256"), "inventory_path": source["inventory_path"]}
        record = corpus.scan_one(image, args.cli, stamp, args.output, "ilspy", "ilspy",
                                 "tie-break", 0, False, args.timeout, args.threads)
        record = {"id": record["id"], "assembly": record["assembly"], "origins": record["origins"],
                  "bodies": record["bodies"], "counts": record["counts"],
                  "scan_complete": record["scan_complete"], "reasons": record["reasons"],
                  "results_path": record["results_path"], "stamp": record["stamp"]}
        old = prior[ident]
        assert old["bodies"] == record["bodies"], "body denominator drifted"
        assert old["origins"] == record["origins"], "image origins drifted"
        previous = {m["token"]: m for m in json.loads(Path(old["results_path"]).read_text())}
        current = {m["token"]: m for m in json.loads(Path(record["results_path"]).read_text())}
        assert previous.keys() == current.keys(), "scan token set drifted"
        gains = sorted(t for t in previous if previous[t]["status"] != "Decompiled"
                       and current[t]["status"] == "Decompiled")
        losses = sorted(t for t in previous if previous[t]["status"] == "Decompiled"
                        and current[t]["status"] != "Decompiled")
        reasons = sorted({previous[t].get("reason") or "" for t in gains})
        rows.append({"id": ident, "assembly": record["assembly"], "bodies": record["bodies"],
                     "complete": record["scan_complete"], "gains": len(gains), "losses": len(losses),
                     "gain_tokens": gains, "lost_tokens": losses,
                     "previous_reasons": reasons,
                     "counts_before": old["counts"], "counts_after": record["counts"]})
        results.append(record)
        print(f"{ident} {record['assembly']}: +{len(gains)} -{len(losses)} complete={record['scan_complete']}",
              flush=True)
    after = {"schema": 1, "scope": "targeted", "complete": all(r["scan_complete"] for r in results),
             "cli": stamp, "references": before["references"],
             "source_baseline": str(args.before.resolve()), "images": results}
    (args.output / "baseline.json").write_text(json.dumps(after, indent=2) + "\n")
    delta = {"source_baseline": str(args.before.resolve()), "revision_scan": str(args.output),
             "images": rows, "gains": sum(r["gains"] for r in rows),
             "losses": sum(r["losses"] for r in rows),
             "elapsed_ms": round((time.monotonic() - started) * 1000)}
    (args.output / "delta.json").write_text(json.dumps(delta, indent=2) + "\n")
    print(json.dumps({k: delta[k] for k in ("gains", "losses", "elapsed_ms")}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
