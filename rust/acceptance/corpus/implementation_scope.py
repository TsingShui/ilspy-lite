#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Apply the user-approved reference-input exclusion to saved static evidence.

This command reads reports only. It never acquires, extracts, hashes, compiles,
scans or executes a selected assembly. Original baseline records remain intact.
"""
import argparse
from collections import Counter
import json
from pathlib import Path
import sys
import time

from corpus import aggregate_gate, gate_image, save


def load(path):
    return json.loads(path.read_text())


def apply(args):
    started = time.monotonic()
    inventory = load(args.inventory)
    baseline = load(args.baseline)
    policy = load(args.exceptions)
    errors, images, skipped, not_applicable = [], [], [], []
    if not inventory["complete"]:
        raise ValueError("independent inventory is incomplete; scope classification is unsafe")
    originals = {i["id"]: i for i in baseline["images"]}
    if len(originals) != len(baseline["images"]):
        raise ValueError("duplicate original baseline image id")
    inventory_ids = [i["id"] for i in inventory["images"]]
    if len(inventory_ids) != len(set(inventory_ids)):
        raise ValueError("duplicate independent inventory image id")
    manifest = load(Path(inventory["manifest"]))
    if not manifest["complete"]:
        raise ValueError("acquisition manifest is incomplete")
    policy_matches_commit = manifest.get("commit") == policy["commit"]
    exceptions = {i["sha256"]: i for i in policy["inputs"]} if policy_matches_commit else {}
    counts, reasons, original_counts, skipped_counts = Counter(), Counter(), Counter(), Counter()
    for image in inventory["images"]:
        identity = {key: image.get(key) for key in ("id", "assembly", "sha256", "mvid", "origins")}
        if image["kind"] == "native":
            not_applicable.append({**identity, "reason": "native_image"})
            continue
        if image["kind"] != "managed":
            errors.append({**identity, "error": "image cannot be classified"})
            continue
        result = originals.pop(image["id"], None)
        if result is None:
            errors.append({**identity, "error": "managed image missing from original baseline"})
            continue
        if result["origins"] != image["origins"] or result["assembly"] != image["assembly"]:
            raise ValueError("inventory and baseline image identities differ")
        if result["bodies"] != image["counts"]["cil"]:
            raise ValueError("inventory and baseline CIL denominators differ")
        if result.get("stamp", {}).get("image_sha256") != image.get("sha256"):
            raise ValueError("inventory and baseline input hashes differ")
        original_counts.update(result["counts"])
        reason, evidence = None, None
        if image["reference_marker"]:
            reason = "verified_reference_contract"
            evidence = {"reference_marker": True, "inventory_path": image["inventory_path"],
                        "zero_code_methods": image["zero_code_methods"],
                        "cil_methods": image["counts"]["cil"]}
        elif image.get("sha256") in exceptions:
            exception = exceptions[image["sha256"]]
            valid = (exception["origin"] in image["origins"]
                     and exception["assembly"] == image["assembly"]
                     and exception["mvid"] == image.get("mvid")
                     and exception["method_rows"] == image["method_rows"]
                     and exception["cil_methods"] == image["counts"]["cil"]
                     and exception["zero_code_methods"] == image["zero_code_methods"])
            if not valid:
                errors.append({**identity, "error": "pinned reference exception evidence differs"})
            else:
                reason = "pinned_reference_contract_exception"
                evidence = {**exception, "commit": policy["commit"], "policy": str(args.exceptions.resolve())}
        if reason:
            skipped_counts.update(result["counts"])
            skipped.append({**identity, "reason": reason, "evidence": evidence,
                            "original_bodies": result["bodies"], "original_counts": result["counts"],
                            "original_results_path": result["results_path"],
                            "reference_metadata_gate_required": False})
            continue
        decision = gate_image(result)
        if not decision["complete"]:
            errors.append({**identity, "error": "target implementation scan is incomplete"})
        if result["bodies"] == 0:
            not_applicable.append({**identity, "reason": "no_cil_implementation_bodies"})
        images.append(result)
        counts.update(result["counts"])
        reasons.update(result["reasons"])
    if originals:
        errors.append({"error": "original baseline has images absent from inventory", "ids": sorted(originals)})
    original_bodies = sum(original_counts.values())
    implementation_bodies = sum(i["bodies"] for i in images)
    skipped_bodies = sum(i["original_bodies"] for i in skipped)
    if original_bodies != implementation_bodies + skipped_bodies:
        errors.append({"error": "original and scoped denominators do not reconcile"})
    if original_bodies != baseline["summary"]["bodies"]:
        errors.append({"error": "original summary differs from per-image accounting"})
    if original_counts != Counter(baseline["summary"]["counts"]):
        errors.append({"error": "original status summary differs from per-image accounting"})
    aggregate = aggregate_gate(images, not errors)
    summary = {"managed_images": len(images), "body_images": sum(i["bodies"] > 0 for i in images),
               "bodies": implementation_bodies, "counts": dict(counts),
               "incomplete_images": sum(not gate_image(i)["complete"] for i in images),
               "passing_body_images": sum(i["bodies"] > 0 and gate_image(i)["pass"] for i in images),
               "unrecovered": aggregate["unrecovered"],
               "aggregate_gate_pass": aggregate["pass"],
               "aggregate_gate_applicable": aggregate["applicable"],
               "aggregate_threshold_percent": aggregate["threshold_percent"],
               "skipped_reference_images": len(skipped),
               "native_images": sum(i["reason"] == "native_image" for i in not_applicable)}
    reconciliation = {"original_bodies": original_bodies, "original_counts": dict(original_counts),
                      "implementation_bodies": implementation_bodies, "implementation_counts": dict(counts),
                      "skipped_bodies": skipped_bodies, "skipped_counts": dict(skipped_counts),
                      "newly_recovered_by_scope_change": 0}
    output = args.output.resolve()
    report = {"schema": 1, "scope": "implementation_only", "complete": not errors,
              "input_executed": False, "inputs_reacquired": False,
              "original_baseline": str(args.baseline.resolve()), "inventory": str(args.inventory.resolve()),
              "cli": baseline["cli"], "summary": summary, "images": images, "skipped": skipped,
              "not_applicable": not_applicable, "errors": errors, "reconciliation": reconciliation,
              "reasons": dict(reasons.most_common()),
              "elapsed_ms": round((time.monotonic() - started) * 1000)}
    save(output / "implementation-baseline.json", report)
    excluded = {i["id"] for i in skipped}
    targets = [i for i in inventory["images"] if i["id"] not in excluded]
    target_inventory = {**inventory, "scope": "implementation_only", "complete": not errors,
                        "source_inventory": str(args.inventory.resolve()), "images": targets,
                        "excluded_reference_ids": sorted(excluded), "errors": errors,
                        "summary": {"candidate_origins": sum(len(i["origins"]) for i in targets),
                                    "unique_images": len(targets),
                                    "managed_images": sum(i["kind"] == "managed" for i in targets),
                                    "native_images": sum(i["kind"] == "native" for i in targets),
                                    "unreadable_images": sum(i["kind"] == "unreadable" for i in targets),
                                    "cil_methods": implementation_bodies}}
    save(output / "implementation-inventory.json", target_inventory)
    save(output / "skipped-reference-inputs.json", skipped)
    save(output / "reconciliation.json", reconciliation)
    unrecovered = aggregate["unrecovered"]
    rate = f"{100 * unrecovered / implementation_bodies:.2f}%" if implementation_bodies else "not applicable"
    lines = ["# Implementation-only scope", "",
             "Reference-only inputs are excluded under the user-approved goal revision.",
             "No separate reference metadata gate is required. Every original input and",
             "method record remains traceable; statuses have not been rewritten.", "",
             f"- Original CIL-marked bodies: {original_bodies:,}.",
             f"- Skipped reference snapshots: {len(skipped):,}; original bodies: {skipped_bodies:,}.",
             f"- Remaining managed snapshots: {len(images):,}; snapshots with CIL bodies: {summary['body_images']:,}.",
             f"- Target implementation bodies: {implementation_bodies:,}.",
             f"- Recovered: {counts.get('Decompiled', 0):,}; unrecovered: {unrecovered:,} ({rate}).",
             f"- Informational per-image <= 1%: {summary['passing_body_images']} / {summary['body_images']}.",
             f"- Aggregate <= 1% gate: {'pass' if aggregate['pass'] else 'not passed'}.",
             "- Newly recovered by classification alone: 0.",
             "- Acquisition, hashing, compilation, decompilation and input execution: none.", "",
             "## Leading implementation first failures", "", "| Reason | Bodies |", "|---|---:|"]
    for reason, count in reasons.most_common(15):
        lines.append(f"| {reason} | {count:,} |")
    lines += ["", "Full skip evidence and all original origins are in skipped-reference-inputs.json.",
              "Per-image target counts, gates and original method result paths are retained in",
              "implementation-baseline.json. reconciliation.json accounts for every original body.", ""]
    (output / "SCOPE.md").write_text("\n".join(lines))
    print(json.dumps({"summary": summary, "complete": not errors, "elapsed_ms": report["elapsed_ms"]}))
    return 0 if aggregate["pass"] else 2 if aggregate["complete"] else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--exceptions", type=Path, default=Path(__file__).with_name("reference-exceptions.json"))
    args = parser.parse_args()
    try:
        return apply(args)
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
