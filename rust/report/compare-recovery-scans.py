#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Compare two saved corpus baselines and report recovered and lost methods.

The scans must cover the same target images (id, origins and body count) and the
same MethodDef token set per image. A method is *gained* when it becomes
``Decompiled`` and *lost* when it stops being ``Decompiled``. The previous
reason of every gained method is counted so a gain is attributed to the blocker
it actually removed, not to the next one that may still block it. Exit 0 only
when there are no losses and no target mismatches; 1 otherwise.
"""
import argparse
from collections import Counter
import json
import sys
from pathlib import Path


def load_results(baseline_path, image):
    path = Path(image.get("results_path", ""))
    if not path.is_absolute():
        path = baseline_path.parent / path
    if not path.is_file():
        path = baseline_path.parent / "images" / f"{int(image['id']):04d}" / "results.json"
    if not path.is_file():
        raise ValueError(f"image {image.get('id')} has no results file")
    data = json.loads(path.read_text())
    if isinstance(data, dict):
        data = data.get("methods", [])
    results = {}
    for record in data:
        token = record.get("token")
        if token in results:
            raise ValueError(f"image {image.get('id')} repeats MethodDef {token}")
        results[token] = (record.get("status"), record.get("reason"))
    return results


def index_baseline(path):
    report = json.loads(path.read_text())
    images = {}
    for image in report.get("images") or []:
        images[image["id"]] = image
    return report, images


def compare(before_path, after_path):
    before_report, before_images = index_baseline(before_path)
    after_report, after_images = index_baseline(after_path)
    mismatches = []
    if set(before_images) != set(after_images):
        mismatches.append({"kind": "image-set", "before": sorted(set(before_images) - set(after_images)),
                           "after": sorted(set(after_images) - set(before_images))})

    previous_reasons = Counter()
    lost_reasons = Counter()
    rows = []
    gained = lost = changed = 0
    images_gained = images_lost = 0
    for image_id in sorted(set(before_images) & set(after_images)):
        before_image = before_images[image_id]
        after_image = after_images[image_id]
        if list(before_image.get("origins", [])) != list(after_image.get("origins", [])):
            mismatches.append({"kind": "origins", "id": image_id,
                               "before": before_image.get("origins"), "after": after_image.get("origins")})
        if int(before_image.get("bodies", 0)) != int(after_image.get("bodies", 0)):
            mismatches.append({"kind": "bodies", "id": image_id,
                               "before": before_image.get("bodies"), "after": after_image.get("bodies")})
        before_results = load_results(before_path, before_image)
        after_results = load_results(after_path, after_image)
        if set(before_results) != set(after_results):
            mismatches.append({"kind": "method-set", "id": image_id,
                               "missing_after": sorted(set(before_results) - set(after_results))[:20],
                               "missing_before": sorted(set(after_results) - set(before_results))[:20]})
            continue
        image_gained = image_lost = image_changed = 0
        image_previous = Counter()
        image_lost_reasons = Counter()
        for token, (before_status, before_reason) in before_results.items():
            after_status, after_reason = after_results[token]
            before_decompiled = before_status == "Decompiled"
            after_decompiled = after_status == "Decompiled"
            if after_decompiled and not before_decompiled:
                image_gained += 1
                previous_reasons[before_reason or "unreported failure"] += 1
                image_previous[before_reason or "unreported failure"] += 1
            elif before_decompiled and not after_decompiled:
                image_lost += 1
                lost_reasons[after_reason or "unreported failure"] += 1
                image_lost_reasons[after_reason or "unreported failure"] += 1
            elif before_status != after_status:
                image_changed += 1
        gained += image_gained
        lost += image_lost
        changed += image_changed
        if image_gained:
            images_gained += 1
        if image_lost:
            images_lost += 1
        if image_gained or image_lost or image_changed:
            rows.append({
                "id": image_id,
                "assembly": after_image.get("assembly"),
                "origins": after_image.get("origins", []),
                "bodies": after_image.get("bodies", 0),
                "before_decompiled": int(before_image.get("counts", {}).get("Decompiled", 0)),
                "after_decompiled": int(after_image.get("counts", {}).get("Decompiled", 0)),
                "gained": image_gained,
                "lost": image_lost,
                "changed": image_changed,
                "previous_reasons": dict(image_previous),
                "lost_reasons": dict(image_lost_reasons),
            })

    before_decompiled = sum(int(image.get("counts", {}).get("Decompiled", 0))
                            for image in before_images.values())
    after_decompiled = sum(int(image.get("counts", {}).get("Decompiled", 0))
                           for image in after_images.values())
    result = {
        "schema": 1,
        "before": {"path": str(before_path), "summary": before_report.get("summary")},
        "after": {"path": str(after_path), "summary": after_report.get("summary")},
        "summary": {
            "images": len(set(before_images) & set(after_images)),
            "before_decompiled": before_decompiled,
            "after_decompiled": after_decompiled,
            "net": after_decompiled - before_decompiled,
            "gained": gained,
            "lost": lost,
            "changed_status": changed,
            "images_gained": images_gained,
            "images_lost": images_lost,
            "target_mismatches": len(mismatches),
        },
        "previous_reasons": dict(previous_reasons.most_common()),
        "lost_reasons": dict(lost_reasons.most_common()),
        "images": rows,
        "mismatches": mismatches,
    }
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("before", type=Path, help="previous baseline.json")
    parser.add_argument("after", type=Path, help="current baseline.json")
    parser.add_argument("output", type=Path, help="delta JSON to write")
    parser.add_argument("--limit", type=int, default=15, help="previous reasons to print (default 15)")
    args = parser.parse_args()

    result = compare(args.before, args.after)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=1) + "\n")
    summary = result["summary"]
    print(f"images={summary['images']} before={summary['before_decompiled']} "
          f"after={summary['after_decompiled']} net={summary['net']:+d} "
          f"gained={summary['gained']} lost={summary['lost']} "
          f"changed={summary['changed_status']} "
          f"images_gained={summary['images_gained']} images_lost={summary['images_lost']} "
          f"mismatches={summary['target_mismatches']}")
    for reason, count in list(result["previous_reasons"].items())[: max(0, args.limit)]:
        print(f"  previous {count:7d}  {reason}")
    return 1 if summary["lost"] or summary["target_mismatches"] else 0


if __name__ == "__main__":
    sys.exit(main())
