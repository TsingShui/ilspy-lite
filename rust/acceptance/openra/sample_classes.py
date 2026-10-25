#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Freeze a reproducible class sample from independent --list-types populations.

Only metadata kind and explicitly excluded fixed regressions affect eligibility.
No production decompiler output or method complexity is inspected. Existing sample
files cannot be overwritten. Preserve the population JSON files beside the report.
"""
import argparse
import hashlib
import json
from pathlib import Path
import secrets


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", action="append", required=True, metavar="ASSEMBLY=JSON")
    parser.add_argument("--exclude", action="append", default=[], metavar="EXACT_TYPE")
    parser.add_argument("--count", type=int, default=20)
    parser.add_argument("--seed", help="16-byte hexadecimal seed; generated once if absent")
    parser.add_argument("--baseline-ref", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    seed = bytes.fromhex(args.seed) if args.seed is not None else secrets.token_bytes(16)
    if len(seed) != 16 or args.count < 1:
        raise ValueError("Expected a 16-byte seed and a positive sample count")
    sources, candidates, seen = [], [], set()
    for source in args.input:
        assembly, path = source.split("=", 1)
        population = json.loads(Path(path).read_text())
        sources.append({"assembly": assembly, "path": path, "mvid": population["mvid"]})
        for entry in population["types"]:
            identity = (assembly, population["mvid"], entry["token"])
            if identity in seen:
                raise ValueError(f"Duplicate metadata identity: {identity}")
            seen.add(identity)
            if entry["kind"] == "class" and entry["name"] not in args.exclude:
                key = seed + b"\0" + "\0".join(identity).encode("utf-8")
                candidates.append({"assembly": assembly, "mvid": population["mvid"],
                    "token": entry["token"], "name": entry["name"],
                    "selection_key": hashlib.sha256(key).hexdigest()})
    if len(candidates) < args.count:
        raise ValueError("Not enough eligible distinct classes; do not replace the population")
    candidates.sort(key=lambda c: (c["selection_key"], c["assembly"], c["mvid"], c["token"]))
    manifest = {"schema": 1, "baseline_ref": args.baseline_ref,
        "algorithm": "ascending SHA256(seed_bytes + NUL + UTF8(assembly + NUL + mvid + NUL + token)); identity tie-break",
        "seed": seed.hex(), "eligibility": "kind=class; no generic, nested, generated, visibility or complexity filter",
        "excluded_names": args.exclude, "population_sources": sources,
        "eligible_count": len(candidates), "count": args.count, "classes": candidates[:args.count]}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("x") as output:
        output.write(json.dumps(manifest, indent=2) + "\n")
    print(f"Frozen {args.count} classes before lowering evaluation; seed={seed.hex()}")


if __name__ == "__main__":
    main()
