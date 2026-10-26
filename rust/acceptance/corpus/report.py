#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Summarize the unchanged goal denominator and explicit scope contradictions."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2] / "report/fallback-corpus"
inventory = json.loads((ROOT / "inventory-final/inventory.json").read_text())
baseline = json.loads((ROOT / "baseline/baseline.json").read_text())
lookup = {i["id"]: i for i in baseline["images"]}
assert inventory["complete"] and baseline["complete"]
assert inventory["summary"]["cil_methods"] == baseline["summary"]["bodies"]
counts, reasons, violations = {}, {}, []
for image in inventory["images"]:
    if image["kind"] != "managed":
        continue
    result = lookup[image["id"]]
    assert image["counts"]["cil"] == result["bodies"]
    if not image["reference_marker"] and not image["zero_code_methods"]:
        for key, value in result["counts"].items():
            counts[key] = counts.get(key, 0) + value
        for key, value in result["reasons"].items():
            reasons[key] = reasons.get(key, 0) + value
    if image["zero_code_methods"] * 100 > image["counts"]["cil"]:
        violations.append({"id": image["id"], "assembly": image["assembly"],
                           "origins": image["origins"], "bodies": image["counts"]["cil"],
                           "zero_code_methods": image["zero_code_methods"],
                           "reference_marker": image["reference_marker"]})
summary = {
    "original_bodies": baseline["summary"]["bodies"],
    "unrecovered": sum(v for k, v in baseline["summary"]["counts"].items() if k != "Decompiled"),
    "zero_code_methods": sum(i["zero_code_methods"] for i in inventory["images"]),
    "zero_code_reference_methods": sum(i["zero_code_methods"] for i in inventory["images"] if i["reference_marker"]),
    "reference_marker_images": sum(i["reference_marker"] for i in inventory["images"]),
    "zero_code_gate_violations": violations,
    "diagnostic_nonreference_nonempty_images": {
        "counts": counts, "reasons": dict(sorted(reasons.items(), key=lambda item: -item[1]))},
    "note": "Diagnostic grouping only. No input or method has been removed from the goal denominator.",
}
(ROOT / "scope-contradiction.json").write_text(json.dumps(summary, indent=2) + "\n")
rate = summary["unrecovered"] / summary["original_bodies"]
zero_rate = summary["zero_code_methods"] / summary["original_bodies"]
lines = [
    "# Initial full corpus baseline and scope evidence", "",
    "Historical raw-input metric. Reference-only exclusion is approved; do not reopen that decision.",
    "The current corrected implementation scope and later semantic results are in CORE_TYPE_ROOTS.md.", "",
    "Goal: mvouhw7y-679jgb. Fixed submodule commit: 9668d49b9f71b40045e5417b55d20e330be95921.",
    "No selected input was executed. Production lowering admission was not expanded.", "",
    "## Complete acquisition and independent accounting", "",
    "- 865 tracked submodule files; all five NuGet archives were enumerated and CRC-checked.",
    "- 1,499 file/member records including two unchanged OpenRA supplemental inputs.",
    "- 1,098 PE origins, retained individually; 1,043 unique content snapshots.",
    "- 976 managed snapshots, 67 native snapshots, zero unreadable snapshots.",
    "- 1,164,708 MethodDef rows independently inspected with System.Reflection.Metadata.",
    "- 1,098,128 CIL method bodies, including module, generated and nested-type methods.",
    "- Raw metadata uses MetadataReaderOptions.None: WinRT projection would hide three real RVAs.",
    "- Every independently known CIL body has exactly one production scan result.", "",
    "## Production baseline", "",
    f"- Recovered: {baseline['summary']['counts'].get('Decompiled', 0):,}.",
    f"- Unrecovered: {summary['unrecovered']:,} ({rate * 100:.2f}%).",
    "- 546 managed snapshots have CIL bodies; 32 currently satisfy the per-image <= 1% gate.",
    "- All scans completed. No failed process, omitted body, unknown or duplicated scan token.",
    "- Gates use integer counts, not rounded display percentages.", "",
    "### Leading first-failure reasons", "",
    "| Reason | Bodies |", "|---|---:|",
]
for reason, count in list(baseline["reasons"].items())[:12]:
    lines.append(f"| {reason} | {count:,} |")
lines += [
    "", "## Why the original all-RVA metric includes missing implementations", "",
    f"{summary['zero_code_methods']:,} CIL-marked RVAs in 124 images have zero IL bytes ({zero_rate * 100:.2f}% of the denominator).",
    f"Of these, {summary['zero_code_reference_methods']:,} are in independently verified ReferenceAssemblyAttribute-marked images.",
    "The remaining 20,854 are in the legacy mscorlib input. Absence of the marker is not implementation proof.",
    f"Zero-length bodies alone exceed 1% in {len(violations)} separate images.", "",
    "Example: dotnet/legacy/System.ServiceModel.dll has 23,537 CIL-marked body RVAs,",
    "all with zero code bytes, and an exact core-scoped ReferenceAssemblyAttribute marker.",
    "Its raw denominator is therefore 100% unrecoverable without either inventing code or redefining",
    "contract declarations. Both silently changing accounting and fabricating executable semantics",
    "would violate the confirmed goal. No such change was made.", "",
    "The submodule dotnet/legacy/README.txt states that the files came from .NET Framework",
    "reference assemblies. Marker, raw headers, per-method RVAs and code lengths are retained in",
    "inventory-final/images/. Directory naming alone is not used as classification proof.", "",
    "The user approved retaining every file while skipping proven reference-only inputs, without",
    "an extra metadata/API gate. Real implementation bodies remain subject to per-image <= 1%.",
    "Exact local-core marker recognition subsequently classified the legacy mscorlib and the",
    ".NET 5 System.Runtime reference input. This is classification, not semantic recovery.",
    "Historical snapshots here preserve the original denominator and counts; later reports",
    "reconcile the approved implementation scope without rewriting these method statuses.", "",
    "## Timing and regression evidence", "",
    "- Rust release incremental build: 4.58 seconds (scan-reporting entry point only).",
    f"- Complete production scan with four processes: {baseline['elapsed_ms'] / 1000:.2f} seconds.",
    f"- Final independent metadata read: {inventory['metadata_read_ms'] / 1000:.2f} seconds; cached reader compile: 0 seconds.",
    "- Python acceptance E2E: nine tests passed, with recorded red/green slices.",
    "- Rust CLI E2E: eight tests passed; full Rust regression, clippy -D warnings and format check passed.",
    "- Rust scan uses the existing decompile_method path, parses the image once and records every MethodDef.",
    "  A scan process exit 0 means complete enumeration, not successful recovery of all methods.", "",
    "## Artifacts", "",
    "- frozen/manifest.json: complete provenance and acquired-file/member hashes.",
    "- inventory-final/inventory.json and images/: independent raw denominators and reference-marker evidence.",
    "- baseline/baseline.json and images/: original production statuses, reasons, errors and per-image gates.",
    "- scope-contradiction.json: every image whose empty stubs alone defeat the <= 1% target.",
    "- e2e-*-red.log / e2e-*-green.log, rust-regression.log, clippy.log: validation records.", "",
    "The goal is not complete. No input was removed from this historical raw metric; the approved",
    "implementation metric is separate and reconciled in the later scope/milestone reports.", "",
]
(ROOT / "BASELINE.md").write_text("\n".join(lines))
print(f"baseline={rate * 100:.2f}% empty_code={zero_rate * 100:.2f}% impossible_images={len(violations)}")
print("diagnostic implementation-only counts:", counts)
