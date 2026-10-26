#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Aggregate static IL fallback statuses and first-failure reasons.

The report never executes the selected assembly. It runs the existing Rust CLI
once per top-level TypeDef, parses the method-status records in the details
view, and groups the first failure reported for each body. The local viewer
uses the same collector through its bounded process runner.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time

MAX_OUTPUT = 16 * 1024 * 1024
STATUS = re.compile(r"^\.method-status (0x[0-9A-F]{8}) (\w+)$", re.M)
REASON = re.compile(r"^// IL FALLBACK: (.+)$", re.M)
FAILED = re.compile(r"^// FAILED(?::| token 0x[0-9A-F]{8}:)?\s*(.*)$", re.M)
TYPE_TOKEN = re.compile(r"^// type token (0x[0-9A-F]{8}) ", re.M)
IL_OFFSET = re.compile(r"\s+at IL_[0-9A-F]+$")
LIST_LINE = re.compile(r"^0x06[0-9A-F]{6}\s+(.*)$")

STATUS_KEYS = {
    "Decompiled": "decompiled",
    "Declaration": "declarations",
    "Partial": "partial",
    "Failed": "failed",
}
SCOPE = ("All method bodies owned by top-level TypeDefs and their nested types. "
         "Rates exclude declarations, which have no body. Each reason is the "
         "first failure reported for that body; later blockers are not counted. "
         "Dependency resolution affects presentation only, not these statuses.")


def normalize_reason(reason):
    """Drop the IL offset so the same failure groups into one row."""
    return IL_OFFSET.sub("", reason.strip()).strip()


def parse_details(text):
    """Read method statuses and first-failure reasons from one --details output."""
    marks = list(STATUS.finditer(text))
    methods = []
    for index, mark in enumerate(marks):
        chunk = text[mark.end():marks[index + 1].start() if index + 1 < len(marks) else len(text)]
        status = mark.group(2)
        reason = ""
        fallback = REASON.search(chunk)
        if fallback:
            reason = normalize_reason(fallback.group(1))
        elif status == "Failed":
            failure = FAILED.search(chunk)
            reason = normalize_reason(failure.group(1)) if failure and failure.group(1).strip() else "unreported failure"
        methods.append({"token": mark.group(1), "status": status, "reason": reason})
    selected = TYPE_TOKEN.search(text)
    return {"token": selected.group(1) if selected else None, "methods": methods}


def type_names_from_list(text):
    """Derive the owning top-level TypeDef names from `list` output."""
    roots = set()
    for line in text.splitlines():
        match = LIST_LINE.match(line)
        if not match or "::" not in match.group(1):
            continue
        owner = match.group(1).split("::", 1)[0].rsplit(None, 1)[-1]
        root = owner.split("/", 1)[0]
        if root and root != "<Module>":
            roots.add(root)
    return sorted(roots)


def run_bounded(command, timeout=60):
    """Run one CLI call with bounded output; the viewer supplies its own runner."""
    started = time.monotonic()
    with tempfile.TemporaryFile() as stdout, tempfile.TemporaryFile() as stderr:
        process = subprocess.Popen([str(value) for value in command], stdout=stdout, stderr=stderr)
        try:
            code = process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
            raise ValueError("Decompiler timed out; choose a smaller assembly or raise --timeout.")
        if stdout.tell() > MAX_OUTPUT or stderr.tell() > MAX_OUTPUT:
            raise ValueError("Decompiler output exceeds the 16 MiB report limit.")
        stdout.seek(0)
        stderr.seek(0)
        return {"text": stdout.read().decode("utf-8", errors="replace"),
                "stderr": stderr.read().decode("utf-8", errors="replace"),
                "exitCode": code, "elapsedMs": round((time.monotonic() - started) * 1000)}


def collect(cli, assembly, type_names=None, workers=8, timeout=60, runner=None):
    """Build one assembly-wide fallback report without executing input code."""
    started = time.monotonic()
    runner = runner or run_bounded
    cli, assembly = str(cli), str(assembly)
    errors = []
    if type_names is None:
        listing = runner([cli, "list", assembly], timeout)
        if listing["exitCode"] != 0:
            raise ValueError(listing["stderr"].strip() or "The assembly could not be listed.")
        type_names = type_names_from_list(listing["text"])
    type_names = sorted(set(type_names))

    def one(name):
        try:
            result = runner([cli, "decompile", assembly, "--type", name,
                             "--details", "--no-auto-references"], timeout)
            parsed = parse_details(result["text"])
            return name, result, parsed, None
        except Exception as error:
            return name, None, None, error

    totals = {"decompiled": 0, "declarations": 0, "partial": 0, "failed": 0}
    reasons = {}
    failures = {}
    rows = []
    with ThreadPoolExecutor(max_workers=max(1, min(workers, len(type_names) or 1))) as pool:
        for name, result, parsed, error in pool.map(one, type_names):
            if error is not None:
                errors.append({"type": name, "error": str(error)})
                continue
            methods = parsed["methods"]
            if not methods and result["exitCode"] != 0:
                errors.append({"type": name,
                               "error": result["stderr"].strip() or "The type could not be read."})
                continue
            row = {"name": name, "token": parsed["token"],
                   "decompiled": 0, "declarations": 0, "partial": 0, "failed": 0,
                   "reasons": {}}
            for method in methods:
                key = STATUS_KEYS.get(method["status"])
                if key is None:
                    errors.append({"type": name,
                                   "error": "Unknown method status: " + method["status"]})
                    continue
                totals[key] += 1
                row[key] += 1
                if key == "partial":
                    reasons[method["reason"]] = reasons.get(method["reason"], 0) + 1
                    row["reasons"][method["reason"]] = row["reasons"].get(method["reason"], 0) + 1
                elif key == "failed":
                    failures[method["reason"]] = failures.get(method["reason"], 0) + 1
            if any(row[key] for key in STATUS_KEYS.values()):
                rows.append(row)

    bodies = totals["decompiled"] + totals["partial"] + totals["failed"]
    partial = totals["partial"]
    failed = totals["failed"]
    rows.sort(key=lambda row: (-row["partial"], -row["decompiled"], row["name"]))
    reason_rows = [{"reason": reason, "count": count, "share": count / partial if partial else 0.0}
                   for reason, count in sorted(reasons.items(), key=lambda item: (-item[1], item[0]))]
    failure_rows = [{"reason": reason, "count": count, "share": count / failed if failed else 0.0}
                    for reason, count in sorted(failures.items(), key=lambda item: (-item[1], item[0]))]
    return {
        "assembly": assembly,
        "scope": SCOPE,
        "summary": {
            **totals,
            "bodies": bodies,
            "rate": partial / bodies if bodies else 0.0,
            "scannedTypes": len(type_names),
        },
        "reasons": reason_rows,
        "failures": failure_rows,
        "types": rows,
        "errors": errors,
        "elapsedMs": round((time.monotonic() - started) * 1000),
    }


def render_text(report, top=20):
    """Print the same aggregation as a compact terminal table."""
    summary = report["summary"]
    lines = [
        "Fallback report: " + report["assembly"],
        "Scope: " + report["scope"],
        f"Types: {summary['scannedTypes']} | bodies: {summary['bodies']} | "
        f"recovered: {summary['decompiled']} | fallback: {summary['partial']} | "
        f"failed: {summary['failed']} | declarations: {summary['declarations']} | "
        f"fallback rate: {summary['rate'] * 100:.2f}%",
    ]
    if report["reasons"]:
        lines += ["", "Fallback reasons:"]
        for row in report["reasons"]:
            lines.append(f"  {row['count']:6d}  {row['share'] * 100:6.2f}%  {row['reason']}")
    if report["failures"]:
        lines += ["", "Failed bodies:"]
        for row in report["failures"]:
            lines.append(f"  {row['count']:6d}  {row['share'] * 100:6.2f}%  {row['reason']}")
    if report["types"]:
        lines += ["", f"Top {min(top, len(report['types']))} fallback types:"]
        for row in report["types"][:top]:
            lines.append(f"  {row['partial']:4d} fallback / {row['decompiled']:4d} recovered  "
                         f"{row['token'] or '?'}  {row['name']}")
    if report["errors"]:
        lines += ["", "Read errors:"]
        for row in report["errors"]:
            lines.append(f"  {row['type']}: {row['error']}")
    lines += ["", f"Elapsed: {report['elapsedMs']} ms"]
    return "\n".join(lines)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("assembly", nargs="+", help="Managed PE assembly to inspect statically")
    parser.add_argument("--cli", type=Path,
                        default=Path(__file__).resolve().parents[2] / "target/release/ilspy-lite",
                        help="Existing ilspy-lite binary (never built automatically)")
    parser.add_argument("--workers", type=int, default=8, help="Concurrent CLI calls")
    parser.add_argument("--timeout", type=int, default=60, help="Seconds per CLI call")
    parser.add_argument("--top", type=int, default=20, help="Fallback types printed per assembly")
    parser.add_argument("--json", type=Path, help="Also write the complete report as JSON")
    args = parser.parse_args(argv)
    if not args.cli.is_file():
        parser.error(f"missing Rust binary: {args.cli}; run cargo build --release --locked first")
    reports = []
    for assembly in args.assembly:
        report = collect(args.cli, assembly, workers=args.workers, timeout=args.timeout)
        reports.append(report)
        print(render_text(report, top=args.top), flush=True)
    if args.json:
        payload = reports[0] if len(reports) == 1 else {"reports": reports}
        args.json.write_text(json.dumps(payload, indent=2) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
