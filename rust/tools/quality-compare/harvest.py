#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Harvest side-by-side type output from ilspy-lite and ILSpy, with crude metrics.

This tool only reads assemblies and runs both decompilers; it never executes
input code. Output is one directory per image holding `<type>.ours.cs`,
`<type>.ilspy.cs` and an index with per-type metrics, so reviewers can compare
quality without re-running either decompiler.
"""
import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
DEFAULT_CLI = ROOT / "rust/target/release/ilspy-lite"
DEFAULT_ILSPY = ROOT / "ICSharpCode.ILSpyCmd/bin/Release/net10.0/ilspycmd.dll"
DOTNET = Path.home() / ".dotnet10/dotnet"

LOCAL = re.compile(r"\bv\d+\b")
HELPER = re.compile(r"__cil_[a-z_]+")
LABEL = re.compile(r"(?m)^\s*(?:IL_[0-9A-F]{4}: )?;\s*$")
IL_COMMENT = re.compile(r"(?m)^\s*// (?:IL_|\.code-bytes|\.maxstack|\.initlocals)")
UNCHECKED = re.compile(r"\bunchecked\(")
DEFAULT = re.compile(r"\bdefault\([^)]*\)")
TRUE_RETURN = re.compile(r"if \(([^\n]+)\)\s*\{\s*return true;\s*\}\s*return false;", re.S)
FALSE_RETURN = re.compile(r"if \(([^\n]+)\)\s*\{\s*return false;\s*\}\s*return true;", re.S)
SELF_INCREMENT = re.compile(r"(\w+) = \1 \+ 1;")
ASSIGN_RETURN = re.compile(r"(?m)^\s*([A-Za-z_][\w<>\[\],. ]*) (v\d+) = ([^\n;]+);\n\s*return \2;\s*$")
CAST = re.compile(r"\((?:int|uint|long|ulong|short|ushort|byte|sbyte|char|bool|float|double|object|string)\)")
EMPTY_STATEMENT = re.compile(r"(?m)^\s*;\s*(?://.*)?$")
THIS = re.compile(r"\bthis\.")
IF_ELSE_RETURN = re.compile(r"(\w+) = true;\s*\n\s*\}\s*\n\s*else\s*\n\s*\{\s*\n\s*\w+ = false;")


def run(command, **kwargs):
    return subprocess.run(command, capture_output=True, text=True, timeout=600, **kwargs)


def list_types(image, ilspy):
    result = run([str(DOTNET), str(ilspy), "-l", "c", str(image)])
    if result.returncode:
        raise SystemExit(f"type listing failed: {result.stderr[-2000:]}")
    types = []
    for line in result.stdout.splitlines():
        line = line.strip()
        for prefix in ("Class ", "Struct ", "Interface ", "Enum ", "Delegate "):
            if line.startswith(prefix):
                types.append((prefix.strip(), line[len(prefix):]))
                break
    return types


def metrics(text):
    lines = text.count("\n") + 1
    return {
        "lines": lines,
        "locals": len(LOCAL.findall(text)),
        "helpers": len(HELPER.findall(text)),
        "stray_labels": len(LABEL.findall(text)),
        "il_comments": len(IL_COMMENT.findall(text)),
        "unchecked": len(UNCHECKED.findall(text)),
        "defaults": len(DEFAULT.findall(text)),
        "bool_returns": len(TRUE_RETURN.findall(text)) + len(FALSE_RETURN.findall(text)),
        "self_increment": len(SELF_INCREMENT.findall(text)),
        "assign_then_return": len(ASSIGN_RETURN.findall(text)),
        "casts": len(CAST.findall(text)),
        "empty_statements": len(EMPTY_STATEMENT.findall(text)),
        "this_prefix": len(THIS.findall(text)),
    }


def decompile_type(image, type_name, cli, ilspy, reference_dirs):
    ours_refs = [arg for directory in reference_dirs for arg in ("--reference-dir", directory)]
    ilspy_refs = [arg for directory in reference_dirs for arg in ("-r", directory)]
    ours = run([str(cli), "decompile", str(image), "--type", type_name, *ours_refs])
    theirs = run([str(DOTNET), str(ilspy), "-t", type_name, *ilspy_refs, str(image)])
    return ours, theirs


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--image", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--cli", type=Path, default=DEFAULT_CLI)
    parser.add_argument("--ilspy", type=Path, default=DEFAULT_ILSPY)
    parser.add_argument("--types", nargs="*", help="explicit type names; default: largest types")
    parser.add_argument("--limit", type=int, default=25)
    parser.add_argument("--min-lines", type=int, default=20)
    parser.add_argument("--reference-dir", action="append", default=[])
    args = parser.parse_args()
    if not args.cli.is_file():
        parser.error("build the production CLI first")
    args.output.mkdir(parents=True, exist_ok=True)
    references = list(args.reference_dir)
    available = list_types(args.image, args.ilspy)
    if args.types:
        wanted = set(args.types)
        chosen = [(kind, name) for kind, name in available if name in wanted]
        missing = wanted - {name for _, name in chosen}
        if missing:
            raise SystemExit(f"types not found: {sorted(missing)}")
    else:
        candidates = []
        for kind, name in available:
            if name.startswith("<>") or "AnonymousType" in name or "EmbeddedAttribute" in name:
                continue
            if any(name.endswith(suffix) for suffix in ("Attribute", "NullableContextAttribute")):
                continue
            ours, _ = decompile_type(args.image, name, args.cli, args.ilspy, references)
            lines = metrics(ours.stdout)["lines"]
            if lines >= args.min_lines:
                candidates.append((lines, kind, name))
        candidates.sort(reverse=True)
        chosen = [(kind, name) for _, kind, name in candidates[: args.limit]]
    rows = []
    for kind, name in chosen:
        ours, theirs = decompile_type(args.image, name, args.cli, args.ilspy, references)
        stem = name.replace("/", "_").replace("+", ".")
        ours_path = args.output / f"{stem}.ours.cs"
        theirs_path = args.output / f"{stem}.ilspy.cs"
        ours_path.write_text(ours.stdout)
        theirs_path.write_text(theirs.stdout)
        mine, reference = metrics(ours.stdout), metrics(theirs.stdout)
        rows.append({
            "kind": kind, "type": name,
            "ours": {"path": str(ours_path), "status": ours.returncode, **mine},
            "ilspy": {"path": str(theirs_path), "status": theirs.returncode, **reference},
            "ratio": round(mine["lines"] / reference["lines"], 3) if reference["lines"] else None,
        })
    args.output.joinpath("index.json").write_text(json.dumps({
        "image": str(args.image), "types": rows}, indent=1) + "\n")
    print(json.dumps({"types": len(rows), "median_ratio": sorted(
        r["ratio"] for r in rows if r["ratio"])[len(rows) // 2] if rows else None}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
