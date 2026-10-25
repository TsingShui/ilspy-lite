#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Collect the frozen twenty classes statically; never execute an input body."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys

from inventory import check_output, identities

HERE = Path(__file__).resolve().parent
RUST = HERE.parents[1]
SUMMARY = r"decompiled=(\d+) declarations=(\d+) partial=(\d+) failed=(\d+)\s*"
COUNT_KEYS = ("decompiled", "declarations", "partial", "failed")


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def capture(command, directory, stem, timeout=60):
    """Preserve actual process bytes, including unsuccessful invocations."""
    command = list(map(str, command))
    record = {"command": command, "exit_code": None, "error": None,
              "stdout": str(directory / (stem + ".stdout.txt")),
              "stderr": str(directory / (stem + ".stderr.txt"))}
    try:
        result = subprocess.run(command, capture_output=True, timeout=timeout)
        out, err = result.stdout, result.stderr
        record["exit_code"] = result.returncode
    except subprocess.TimeoutExpired as error:
        out, err = error.stdout or b"", error.stderr or b""
        record["error"] = f"Process timed out after {timeout} seconds"
    except OSError as error:
        out, err = b"", b""
        record["error"] = str(error)
    Path(record["stdout"]).write_bytes(out)
    Path(record["stderr"]).write_bytes(err)
    write_json(directory / (stem + ".process.json"), record)
    return record, out.decode("utf-8", errors="replace"), err.decode("utf-8", errors="replace")


def counts_from_process(record, stderr):
    match = re.fullmatch(SUMMARY, stderr)
    if record["error"] or not match:
        raise ValueError(f"Missing/unexpected CLI summary: {record['error'] or stderr}")
    counts = dict(zip(COUNT_KEYS, map(int, match.groups())))
    expected = 1 if counts["failed"] else 2 if counts["partial"] else 0
    if record["exit_code"] != expected:
        raise ValueError("CLI status/exit discrepancy")
    if counts["failed"]:
        raise ValueError("CLI reports failed bodies")
    return counts


def validate_identity(entry, snapshot):
    actual = identities(snapshot)
    if actual["mvid"] != entry["mvid"] or actual["root"] != entry["token"]:
        raise ValueError("Frozen MVID/root identity mismatch")
    types = {ty["token"]: ty for ty in snapshot["types"]}
    if entry["token"] not in types or types[entry["token"]]["name"] != entry["name"]:
        raise ValueError("Frozen class missing or renamed")
    for token in types:
        seen = set()
        while token != entry["token"]:
            if token in seen or token not in types:
                raise ValueError("Nested ownership does not reach selected root")
            seen.add(token)
            token = types[token]["parent"]
    return actual


def check_names(snapshot, text):
    # inventory.check_output verifies tokens, signatures and ownership; also keep names exact.
    def escaped(name):
        special = {"\t": r"\t", "\r": r"\r", "\n": r"\n", "\\": r"\\", "'": r"\'", '"': r'\"'}
        return "".join(special.get(c, c if 0x20 <= ord(c) <= 0x7e else f"\\u{{{ord(c):x}}}") for c in name)

    for ty in snapshot["types"]:
        if f"// type token {ty['token']} {escaped(ty['name'])}\n" not in text:
            raise ValueError(f"Type name mismatch: {ty['token']}")
        for kind, plural in (("method", "methods"), ("field", "fields"),
                             ("property", "properties"), ("event", "events")):
            for member in ty[plural]:
                lines = re.findall(r"^\." + kind + " " + member["token"] + r" ([^\n]*)", text, re.M)
                name = escaped(member["name"])
                marker = {"method": f"::{name}(", "field": f"// name={name} signature-bytes=",
                          "property": f" {name}(", "event": f"name={name} type="}[kind]
                if len(lines) != 1 or marker not in lines[0]:
                    raise ValueError(f"{kind} name mismatch: {member['token']}")


def method_blocks(text):
    result = {}
    for block in re.split(r"^// type token .*\n", text, flags=re.M)[1:]:
        matches = list(re.finditer(r"^\.method (0x06[0-9A-F]{6}) .*\n", block, re.M))
        for index, match in enumerate(matches):
            end = matches[index + 1].start() if index + 1 < len(matches) else len(block)
            result[match[1]] = block[match.start():end]
    return result


def normalized(text):
    return "\n".join(line.strip() for line in text.splitlines())


def check_original_il(method, record, text, stderr):
    token = method["token"]
    if record["error"] or record["exit_code"] != 0 or stderr.strip():
        raise ValueError(f"Original IL collection failed: {token}")
    if not text.startswith(f"// token {token} "):
        raise ValueError(f"Original IL identity missing: {token}")
    if method["has_rva"]:
        for marker in (".method-flags ", ".maxstack ", ".initlocals ", ".code-bytes "):
            if marker not in text:
                raise ValueError(f"Lost original IL ({marker.strip()}): {token}")
        if not re.search(r"^IL_[0-9A-F]+: ", text, re.M):
            raise ValueError(f"Original IL instructions missing: {token}")


def check_fallback(method, status, block, readable, il):
    token = method["token"]
    if status == "Partial":
        evidence = "\n".join("// " + line for line in il.splitlines())
        if normalized(evidence) not in normalized(block):
            raise ValueError(f"Incomplete detailed fallback IL: {token}")
        # The readable contract omits only the token/signature and flag headers.
        lines = [line for line in il.splitlines()
                 if not line.startswith("// token ") and not line.startswith(".method-flags ")]
        evidence = "\n".join("// " + line for line in lines)
        if not evidence or normalized(evidence) not in normalized(readable):
            raise ValueError(f"Incomplete readable fallback IL: {token}")
    elif status == "Declaration" and method["has_rva"]:
        raise ValueError(f"IL body hidden as declaration: {token}")


def applicability(method, il):
    """Candidate triage only; runtime fixture suitability remains a separate decision."""
    ops = sorted(set(re.findall(r"^IL_[0-9A-F]+: (\S+)", il, re.M)))
    if method["name"] in (".ctor", ".cctor"):
        reason = "Constructor body candidate only; receiver/static initialization and exact call adapters need a fixture"
    elif any(op in ("call", "callvirt", "newobj", "ldftn", "ldvirtftn") for op in ops):
        reason = "Call-bearing candidate; exact external targets, dispatch and fixture dependencies must be reviewed"
    elif any("fld" in op for op in ops):
        reason = "Field/state candidate; construct receiver/dependencies and compare return, mutations and exceptions"
    else:
        reason = "Potential isolated candidate; review signature, setup and boundary/exception cases before execution"
    return {"potential_behavior_test_applicability": "candidate_not_validated",
            "reason": reason, "opcodes": ops, "noRuntimeProof": True}


def compile_oracle(args):
    if args.oracle:
        return args.oracle.resolve()
    root = args.dotnet.resolve().parent

    def latest(path):
        return max((p for p in path.iterdir() if p.name.startswith("10.")),
                   key=lambda p: tuple(map(int, p.name.split("."))))

    sdk = latest(root / "sdk")
    ref = latest(root / "packs/Microsoft.NETCore.App.Ref") / "ref/net10.0"
    runtime = latest(root / "shared/Microsoft.NETCore.App")
    oracle = args.output / "ClassInventory.dll"
    response = oracle.with_suffix(".rsp")
    response.write_text("\n".join(["-nologo", "-target:exe", "-langversion:14", "-warnaserror+",
        f'-out:"{oracle}"', *[f'-r:"{p}"' for p in sorted(ref.glob("*.dll"))],
        f'"{RUST / "tests/dotnet/ClassInventory.cs"}"']))
    write_json(oracle.with_suffix(".runtimeconfig.json"), {"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}})
    record, _, _ = capture([args.dotnet, sdk / "Roslyn/bincore/csc.dll", f"@{response}"],
                           args.output, "oracle-build", timeout=120)
    if record["error"] or record["exit_code"] != 0:
        raise ValueError("Independent .NET 10 oracle compilation failed; see oracle-build process evidence")
    return oracle


def inspect_class(args, oracle, manifest, entry, index):
    directory = args.output / f"{index:02d}-{Path(entry['assembly']).stem}-{entry['token']}"
    directory.mkdir()
    row = {"selection": entry, "noRuntimeProof": True, "errors": [], "methods": [],
           "configurations": {}, "supported_methods": []}
    assembly = args.assemblies / entry["assembly"]
    record, text, stderr = capture([args.dotnet, oracle, assembly, entry["name"]], directory, "inventory")
    row["inventory_process"] = record
    snapshot = None
    try:
        if record["error"] or record["exit_code"] != 0 or stderr.strip():
            raise ValueError("Independent inventory failed or class missing")
        snapshot = json.loads(text)
        row["inventory"] = validate_identity(entry, snapshot)
    except (ValueError, KeyError, TypeError) as error:
        row["errors"].append(f"inventory: {error}")
    originals = {}
    if snapshot:
        for ty in snapshot["types"]:
            for method in ty["methods"]:
                token = method["token"]
                process, il, err = capture([args.cli, "il", assembly, "--method", token], directory, token + ".il")
                item = {"owner": ty["token"], "owner_name": ty["name"], "token": token,
                        "name": method["name"], "signature_bytes": method["signature"],
                        "has_rva": method["has_rva"], "original_il": process,
                        "signature": il.splitlines()[0] if il.splitlines() else None,
                        "statuses": {}, "errors": [], "noRuntimeProof": True}
                try:
                    check_original_il(method, process, il, err)
                except ValueError as error:
                    item["errors"].append(str(error))
                originals[token] = (method, il, item)
                row["methods"].append(item)
    companions = [args.assemblies / source["assembly"] for source in manifest["population_sources"]
                  if source["assembly"] != entry["assembly"]]
    explicit = ["--no-auto-references"]
    for reference in companions + args.reference:
        explicit += ["--reference", str(reference)]
    for reference_dir in args.reference_dir:
        explicit += ["--reference-dir", str(reference_dir)]
    for name, options in (("automatic", []), ("explicit", explicit)):
        config = {"errors": [], "counts": None}
        row["configurations"][name] = config
        outputs = {}
        for style, extra in (("readable", []), ("details", ["--details"])):
            process, stdout, stderr = capture([args.cli, "decompile", assembly, "--type", entry["name"],
                                               *options, *extra], directory, name + "." + style)
            config[style] = process
            outputs[style] = stdout
            try:
                counts = counts_from_process(process, stderr)
                if style == "readable":
                    config["counts"] = counts
                elif counts != config["counts"]:
                    raise ValueError("Readable/details status discrepancy")
                if style == "details" and snapshot:
                    check_output(snapshot, stdout, stderr, process["exit_code"])
                    check_names(snapshot, stdout)
            except ValueError as error:
                config["errors"].append(f"{style}: {error}")
        counts = config["counts"]
        readable = outputs["readable"]
        if counts and len(re.findall(r"^\s*// IL fallback:", readable, re.M)) != counts["partial"]:
            config["errors"].append("Readable fallback/status count discrepancy")
        if re.search(r"^\s*(?:\.method-status |\.type-flags |// (?:Failed|FAILED))", readable, re.M):
            config["errors"].append("Readable style contains diagnostic metadata or failed bodies")
        if not re.search(r"\b(class|interface|struct|enum)\s+", readable):
            config["errors"].append("Readable class declaration missing")
        blocks = method_blocks(outputs["details"])
        for token, (method, il, item) in originals.items():
            block = blocks.get(token, "")
            matches = re.findall(r"^\.method-status " + token + r" (\w+)$", block, re.M)
            status = matches[0] if len(matches) == 1 else None
            item["statuses"][name] = status
            try:
                if status not in ("Decompiled", "Declaration", "Partial", "Failed"):
                    raise ValueError(f"Missing/invalid method status: {token}")
                check_fallback(method, status, block, readable, il)
            except ValueError as error:
                config["errors"].append(str(error))
            item.setdefault("diagnostics", {})[name] = re.findall(r"^// (?:IL FALLBACK:|FAILED)[^\n]*", block, re.M)
    for token, (method, il, item) in originals.items():
        supported = [name for name, status in item["statuses"].items() if status == "Decompiled"]
        if supported:
            row["supported_methods"].append({"token": token, "owner": item["owner"],
                "owner_name": item["owner_name"], "name": item["name"], "signature": item["signature"],
                "signature_bytes": item["signature_bytes"], "configurations": supported,
                "collection_errors": item["errors"], **applicability(method, il)})
    errors = row["errors"] + [error for item in row["methods"] for error in item["errors"]]
    errors += [f"{name}: {error}" for name, config in row["configurations"].items() for error in config["errors"]]
    row["static_status"] = "failed" if errors else "passed"
    write_json(directory / "class.json", row)
    return {"selection": entry, "path": str((directory / "class.json").relative_to(args.output)),
            "static_status": row["static_status"], "errors": errors,
            "counts": {name: config["counts"] for name, config in row["configurations"].items()},
            "supported_method_count": len(row["supported_methods"])}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", required=True, type=Path)
    parser.add_argument("--assemblies", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path, help="new independent ignored report directory")
    parser.add_argument("--reference", action="append", type=Path, default=[], help="extra explicit-mode dependency (repeatable)")
    parser.add_argument("--reference-dir", action="append", type=Path, default=[], help="extra explicit-mode directory (repeatable)")
    parser.add_argument("--dotnet", type=Path, default=Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10")) / "dotnet")
    parser.add_argument("--oracle", type=Path, help="reuse a compiled independent ClassInventory.dll instead of compiling it")
    args = parser.parse_args()
    for name in ("cli", "assemblies", "output", "dotnet"):
        setattr(args, name, getattr(args, name).absolute())
    args.reference = [p.absolute() for p in args.reference]
    args.reference_dir = [p.absolute() for p in args.reference_dir]
    # Refuse report reuse rather than overwriting another run's evidence.
    args.output.mkdir(parents=True, exist_ok=False)
    manifest = json.loads((HERE / "random-classes.json").read_text())
    provenance = json.loads((HERE / "manifest.json").read_text())
    report = {"schema": 1, "scope": "STATIC CLI/inventory/IL only; not a final semantic gate",
              "noRuntimeProof": True, "frozen_manifest": manifest, "provenance": provenance,
              "cli": str(args.cli), "assemblies": str(args.assemblies), "classes": [], "errors": []}
    revision = subprocess.run(["git", "-C", str(RUST), "rev-parse", "HEAD"], capture_output=True, text=True)
    report["collector_checkout_head"] = revision.stdout.strip() if revision.returncode == 0 else None
    report["cli_revision_note"] = "Caller must build --cli from the recorded checkout; no binary hash or semantic certification"
    try:
        if manifest["count"] != 20 or len(manifest["classes"]) != 20:
            raise ValueError("Expected all twenty frozen selections; no filtering/resampling")
        identities_seen = {(e["assembly"], e["mvid"], e["token"]) for e in manifest["classes"]}
        if len(identities_seen) != 20:
            raise ValueError("Duplicate frozen selection identity")
        oracle = compile_oracle(args)
        for index, entry in enumerate(manifest["classes"], 1):
            try:
                row = inspect_class(args, oracle, manifest, entry, index)
            except (OSError, ValueError, KeyError, TypeError) as error:
                row = {"selection": entry, "static_status": "failed", "errors": [str(error)], "path": None}
            report["classes"].append(row)
            print(f"{index:02d} {entry['token']} {entry['name']}: {row['static_status']}")
    except (OSError, ValueError, KeyError, TypeError) as error:
        report["errors"].append(str(error))
    # Setup failures must not erase the frozen selections from the report.
    for entry in manifest["classes"][len(report["classes"]):]:
        report["classes"].append({"selection": entry, "path": None, "static_status": "failed",
                                  "errors": ["Not collected because collector setup failed"], "noRuntimeProof": True})
    report["static_pass_count"] = sum(row["static_status"] == "passed" for row in report["classes"])
    report["static_failed_count"] = len(report["classes"]) - report["static_pass_count"]
    write_json(args.output / "report.json", report)
    print(f"Static checks: {report['static_pass_count']}/20; no runtime proof. Evidence: {args.output / 'report.json'}")
    return 0 if report["static_pass_count"] == 20 and not report["errors"] else 1


if __name__ == "__main__":
    sys.exit(main())
