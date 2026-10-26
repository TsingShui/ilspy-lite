#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Fixed-corpus static acceptance tooling. Never execute selected input code."""
import argparse
from concurrent.futures import ProcessPoolExecutor
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import subprocess
import sys
import time
import zipfile

PINNED_COMMIT = "9668d49b9f71b40045e5417b55d20e330be95921"
PE_EXTENSIONS = {".dll", ".exe", ".netmodule", ".winmd"}
ARCHIVE_EXTENSIONS = {".zip", ".nupkg", ".7z", ".rar", ".tar", ".gz", ".bz2", ".xz"}
MAX_FILE = 512 * 1024 * 1024
MAX_EXPANDED = 2 * 1024 * 1024 * 1024
MAX_MEMBERS = 100_000
MAX_DEPTH = 8


def git(checkout, *args):
    return subprocess.check_output(["git", "-C", str(checkout), *args])


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2) + "\n")
    temporary.replace(path)


def collect(args):
    started = time.monotonic()
    checkout, output = args.checkout.resolve(), args.output.resolve()
    commit = git(checkout, "rev-parse", "HEAD").decode().strip()
    if commit != args.commit:
        raise ValueError(f"checkout commit differs: {commit} != {args.commit}")
    if git(checkout, "status", "--porcelain", "--untracked-files=no").strip():
        raise ValueError("tracked corpus files differ from the pinned checkout")
    output.mkdir(parents=True, exist_ok=True)
    previous = {}
    previous_supplements = {}
    if args.reuse_acquisition:
        acquisition = json.loads(args.reuse_acquisition.read_text())
        if acquisition["submodule_commit"] != commit:
            raise ValueError("acquisition cache belongs to another commit")
        previous = {r["origin"].removeprefix("ILSpy-tests/"): r for r in acquisition["files"]}
        previous_supplements = {Path(r["origin"]).name: r["sha256"] for r in acquisition.get("openra", [])}
    files, archives, errors = [], [], []
    work = {"expanded": 0, "members": 0}

    def visit(data, origin, path, blob=None, depth=0, reused_hash=None):
        if len(data) > MAX_FILE:
            raise ValueError("file exceeds 512 MiB limit")
        digest = reused_hash or hashlib.sha256(data).hexdigest()
        record = {"origin": origin, "path": str(path), "bytes": len(data),
                  "sha256": digest, "git_blob": blob,
                  "hash_reused": reused_hash is not None,
                  "pe_candidate": data[:2] == b"MZ" or PurePosixPath(origin).suffix.lower() in PE_EXTENSIONS}
        files.append(record)
        is_archive = data[:4] in (b"PK\x03\x04", b"PK\x05\x06", b"PK\x07\x08")
        if not is_archive:
            if PurePosixPath(origin).suffix.lower() in ARCHIVE_EXTENSIONS:
                raise ValueError("unrecognized or unsupported archive; enumeration is incomplete")
            return
        if depth >= MAX_DEPTH:
            raise ValueError("archive nesting exceeds limit")
        with zipfile.ZipFile(io.BytesIO(data)) as archive:
            entries = archive.infolist()
            work["members"] += len(entries)
            if work["members"] > MAX_MEMBERS:
                raise ValueError("archive member count exceeds limit")
            destination = output / "extracted" / digest
            names = set()
            # Validate the whole directory before writing any of its members.
            for entry in entries:
                name = PurePosixPath(entry.filename)
                mode = entry.external_attr >> 16
                if (not entry.filename or name.is_absolute() or ".." in name.parts
                        or "\\" in entry.filename or ":" in entry.filename
                        or "\x00" in entry.filename or str(name) in names):
                    raise ValueError(f"unsafe or duplicate archive member: {entry.filename!r}")
                if stat.S_ISLNK(mode) or (mode and stat.S_IFMT(mode) not in (0, stat.S_IFREG, stat.S_IFDIR)):
                    raise ValueError(f"non-regular archive member: {entry.filename!r}")
                if entry.flag_bits & 1:
                    raise ValueError(f"encrypted archive member: {entry.filename!r}")
                if entry.file_size > MAX_FILE:
                    raise ValueError("archive member exceeds 512 MiB limit")
                work["expanded"] += entry.file_size
                if work["expanded"] > MAX_EXPANDED:
                    raise ValueError("archive expansion exceeds 2 GiB limit")
                names.add(str(name))
            archives.append({"origin": origin, "sha256": digest, "members": len(entries),
                             "expanded_bytes": sum(e.file_size for e in entries)})
            for entry in entries:
                if entry.is_dir():
                    continue
                member_origin = origin + "!" + entry.filename
                member_path = destination.joinpath(*PurePosixPath(entry.filename).parts)
                try:
                    # ZipFile.read validates the CRC; no archive metadata is used as a write path unchecked.
                    member = archive.read(entry)
                    if len(member) != entry.file_size:
                        raise ValueError("archive member size differs from directory")
                    parent = output
                    for part in member_path.relative_to(output).parts[:-1]:
                        parent = parent / part
                        if parent.is_symlink():
                            raise ValueError("extraction directory is a symlink")
                        parent.mkdir(exist_ok=True)
                    if member_path.is_symlink():
                        raise ValueError("extraction destination is a symlink")
                    if member_path.exists():
                        if not member_path.is_file() or member_path.stat().st_size != len(member) or member_path.read_bytes() != member:
                            raise ValueError("existing extraction content differs from archive member")
                    else:
                        with member_path.open("xb") as stream:
                            stream.write(member)
                    visit(member, member_origin, member_path, depth=depth + 1)
                except (ValueError, OSError, zipfile.BadZipFile, NotImplementedError) as error:
                    errors.append({"origin": member_origin, "error": str(error)})

    for raw in git(checkout, "ls-tree", "-rz", "HEAD").split(b"\x00"):
        if not raw:
            continue
        metadata, name_bytes = raw.split(b"\t", 1)
        mode, kind, blob = metadata.decode().split()
        name = name_bytes.decode()
        path = checkout / name
        try:
            if kind != "blob" or mode not in ("100644", "100755") or path.is_symlink():
                raise ValueError("corpus entry is not a regular tracked file")
            if path.stat().st_size > MAX_FILE:
                raise ValueError("file exceeds 512 MiB limit")
            data = path.read_bytes()
            cached = previous.get(name)
            digest = cached["sha256"] if cached and cached["git_blob"] == blob and cached["bytes"] == len(data) else None
            visit(data, name, path, blob=blob, reused_hash=digest)
        except (ValueError, OSError, zipfile.BadZipFile, NotImplementedError) as error:
            errors.append({"origin": name, "error": str(error)})
    for path in args.supplement:
        path = path.resolve()
        try:
            if path.is_symlink() or not path.is_file() or path.stat().st_size > MAX_FILE:
                raise ValueError("supplement must be a bounded regular file")
            visit(path.read_bytes(), "supplement:" + path.name, path,
                  reused_hash=previous_supplements.get(path.name))
        except (ValueError, OSError, zipfile.BadZipFile, NotImplementedError) as error:
            errors.append({"origin": str(path), "error": str(error)})
    origins = [r["origin"] for r in files]
    if len(origins) != len(set(origins)):
        errors.append({"origin": "manifest", "error": "duplicate input provenance"})
    manifest = {"schema": 1, "commit": commit, "checkout": str(checkout),
                "complete": not errors, "input_executed": False,
                "files": files, "archives": archives, "errors": errors,
                "elapsed_ms": round((time.monotonic() - started) * 1000)}
    save(output / "manifest.json", manifest)
    print(f"files={len(files)} PE_candidates={sum(r['pe_candidate'] for r in files)} "
          f"archives={len(archives)} errors={len(errors)} elapsed_ms={manifest['elapsed_ms']}")
    return 0 if manifest["complete"] else 1


def metadata_reader():
    """Compile the owned static reader only when its source/toolchain changes."""
    here = Path(__file__).resolve().parent
    report = here.parents[1] / "report/fallback-corpus/reader"
    dotnet = Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10"))
    def latest(path):
        versions = [p for p in path.iterdir() if p.name.startswith("10.") and p.name.count(".") == 2]
        if not versions:
            raise ValueError(f"existing .NET 10 toolchain missing: {path}")
        return max(versions, key=lambda p: tuple(map(int, p.name.split("."))))
    sdk = latest(dotnet / "sdk")
    runtime = latest(dotnet / "shared/Microsoft.NETCore.App")
    refs = latest(dotnet / "packs/Microsoft.NETCore.App.Ref") / "ref/net10.0"
    source = here / "MetadataInventory.cs"
    stamp = {"source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
             "sdk": str(sdk), "runtime": str(runtime), "refs": str(refs)}
    report.mkdir(parents=True, exist_ok=True)
    binary = report / "MetadataInventory.dll"
    previous = report / "build-stamp.json"
    if binary.is_file() and previous.is_file() and json.loads(previous.read_text()) == stamp:
        return [str(dotnet / "dotnet"), str(binary)], 0
    response = report / "compile.rsp"
    response.write_text("\n".join([
        "-nologo", "-target:exe", "-langversion:14", "-warnaserror+", "-deterministic+",
        f'-out:"{binary}"', *[f'-r:"{p}"' for p in sorted(refs.glob("*.dll"))], f'"{source}"']) + "\n")
    started = time.monotonic()
    result = subprocess.run([str(dotnet / "dotnet"), str(sdk / "Roslyn/bincore/csc.dll"),
                             "@" + str(response)], capture_output=True, text=True, timeout=120)
    (report / "build.log").write_text(result.stdout + result.stderr)
    if result.returncode:
        raise ValueError("static metadata reader compilation failed: " + result.stdout + result.stderr)
    save(binary.with_suffix(".runtimeconfig.json"), {"runtimeOptions": {"tfm": "net10.0",
         "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}})
    save(previous, stamp)
    return [str(dotnet / "dotnet"), str(binary)], round((time.monotonic() - started) * 1000)


def inventory(args):
    started = time.monotonic()
    manifest = json.loads(args.manifest.read_text())
    if not manifest["complete"]:
        raise ValueError("acquisition manifest is incomplete")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    candidates = [r for r in manifest["files"] if r["pe_candidate"]]
    groups = {}
    for record in candidates:
        # Equal content retains every origin while avoiding repeated metadata work.
        key = record.get("sha256", str(Path(record["path"]).resolve()))
        groups.setdefault(key, []).append(record)
    requests = output / "reader-inputs.json"
    ordered = list(groups.values())
    save(requests, [r[0]["path"] for r in ordered])
    command, compile_ms = metadata_reader()
    read_started = time.monotonic()
    with (output / "reader-output.ndjson").open("w") as stdout, (output / "reader-stderr.log").open("w") as stderr:
        result = subprocess.run([*command, str(requests)], stdout=stdout, stderr=stderr, timeout=300)
    if result.returncode:
        raise ValueError("static metadata reader failed; see reader-stderr.log")
    read_ms = round((time.monotonic() - read_started) * 1000)
    images, errors = [], []
    lines = (output / "reader-output.ndjson").read_text().splitlines()
    if len(lines) != len(ordered):
        raise ValueError("static metadata reader omitted an image")
    for index, (line, origins) in enumerate(zip(lines, ordered)):
        snapshot = json.loads(line)
        if snapshot["path"] != origins[0]["path"]:
            raise ValueError("static metadata reader reordered an image")
        methods = snapshot["methods"]
        counts = {kind: sum(m["body_kind"] == kind for m in methods)
                  for kind in ("cil", "declaration", "native", "missing")}
        snapshot_path = output / "images" / f"{index:04d}.json"
        save(snapshot_path, snapshot)
        image = {"id": index, "sha256": origins[0].get("sha256"), "path": snapshot["path"],
                 "origins": [r["origin"] for r in origins], "kind": snapshot["kind"],
                 "assembly": snapshot.get("assembly"), "mvid": snapshot.get("mvid"),
                 "complete": snapshot["complete"], "method_rows": snapshot.get("method_rows", 0),
                 "reference_marker": snapshot.get("reference_marker", False),
                 "zero_code_methods": sum(m["body_kind"] == "cil" and m["code_size"] == 0 for m in methods),
                 "counts": counts, "body_read_errors": sum(bool(m["body_error"]) for m in methods),
                 "inventory_path": str(snapshot_path)}
        images.append(image)
        for error in snapshot["errors"]:
            errors.append({"image": index, "origins": image["origins"], **error})
    summary = {"candidate_origins": len(candidates), "unique_images": len(images),
               "managed_images": sum(i["kind"] == "managed" for i in images),
               "native_images": sum(i["kind"] == "native" for i in images),
               "unreadable_images": sum(i["kind"] == "unreadable" for i in images),
               "cil_methods": sum(i["counts"]["cil"] for i in images)}
    result = {"schema": 1, "manifest": str(args.manifest.resolve()), "complete": not errors,
              "input_executed": False, "images": images, "errors": errors, "summary": summary,
              "compile_ms": compile_ms, "metadata_read_ms": read_ms,
              "elapsed_ms": round((time.monotonic() - started) * 1000)}
    save(output / "inventory.json", result)
    print(json.dumps({"summary": summary, "errors": len(errors), "compile_ms": compile_ms,
                      "metadata_read_ms": read_ms, "elapsed_ms": result["elapsed_ms"]}))
    return 0 if result["complete"] else 1


def gate_image(image):
    bodies = image["bodies"]
    counts = image["counts"]
    recovered = counts.get("Decompiled", 0)
    unrecovered = sum(value for key, value in counts.items() if key != "Decompiled")
    accounted = recovered + unrecovered
    if accounted < bodies:
        unrecovered += bodies - accounted
    complete = image["scan_complete"] and accounted == bodies
    return {"id": image["id"], "bodies": bodies, "unrecovered": unrecovered,
            "complete": complete, "pass": complete and 100 * unrecovered <= bodies}


def gate(args):
    report = json.loads(args.baseline.read_text())
    decisions = [gate_image(i) for i in report["images"]]
    complete = report["complete"] and all(i["complete"] for i in decisions)
    passed = complete and all(i["pass"] for i in decisions)
    print(json.dumps({"complete": complete, "pass": passed, "images": decisions}))
    return 0 if passed else 2 if complete else 1


def build_identity_index(cli, paths, timeout):
    """Read each image's own identity and the references it declares, from the images alone."""
    owned, declared = {}, {}
    paths = [str(p) for p in paths]
    if not paths:
        return owned, declared
    process = subprocess.run([str(cli), "identities", *paths],
                             capture_output=True, text=True, timeout=timeout)
    if process.returncode:
        raise ValueError(f"identities failed: {process.stderr[-2000:]}")
    for line in process.stdout.splitlines():
        record = json.loads(line)
        if "error" in record:
            continue
        declared[record["path"]] = record["references"]
        owned.setdefault(record["identity"], []).append(record["path"])
    return owned, declared


def parse_identity(text):
    """Split a canonical identity into the fields a selection policy compares."""
    parts = [part.strip() for part in text.split(",")]
    fields = {}
    for part in parts[1:]:
        key, _, value = part.partition("=")
        fields[key.strip()] = value.strip()
    version = tuple(int(n) for n in fields.get("Version", "0.0.0.0").split("."))
    return {"name": parts[0], "version": version,
            "token": fields.get("PublicKeyToken", "null")}


def directory_family_depth(own_path, candidate):
    """Count the leading directory components an image shares with a candidate file."""
    depth = 0
    for own, other in zip(PurePosixPath(own_path).parent.parts,
                          PurePosixPath(candidate).parent.parts):
        if own != other:
            break
        depth += 1
    return depth


def is_legacy_path(path):
    """Report whether a directory component is named legacy; that family holds the framework set."""
    return "legacy" in PurePosixPath(path).parent.parts


def order_candidates(own_path, candidates):
    """Order identity-equal files: nearest directory family first, then non-legacy, then path."""
    return sorted(candidates, key=lambda path: (-directory_family_depth(own_path, path),
                                                is_legacy_path(path), path))


def tie_break_reason(own_path, ordered):
    """Name the rule that separated the first ordered candidate from every other one."""
    chosen, rest = ordered[0], ordered[1:]
    depth = directory_family_depth(own_path, chosen)
    if all(depth > directory_family_depth(own_path, path) for path in rest):
        return "directory-family"
    tied = [path for path in rest if directory_family_depth(own_path, path) == depth]
    if not is_legacy_path(chosen) and all(is_legacy_path(path) for path in tied):
        return "non-legacy"
    return "path-order"


def select_references(owned, identities, own_path, policy, ambiguity):
    """Resolve each reference identity to one corpus file and record the choice.

    With ``ambiguity == "tie-break"`` an identity owned by several identity-equal files
    selects exactly one of them; otherwise it stays ambiguous and nothing is passed.
    """
    decisions = []
    for identity in sorted(set(identities)):
        candidates = sorted(path for path in owned.get(identity, []) if path != own_path)
        decision = {"identity": identity, "outcome": "unresolved", "chosen": None,
                    "basis": None, "candidates": candidates, "same_major_candidates": []}
        if len(candidates) == 1:
            decision.update(outcome="selected", chosen=candidates[0], basis="unique")
        elif candidates:
            if ambiguity == "tie-break":
                ordered = order_candidates(own_path, candidates)
                decision.update(outcome="selected", chosen=ordered[0],
                                basis=tie_break_reason(own_path, ordered))
            else:
                decision["outcome"] = "ambiguous"
        elif policy == "same-major":
            wanted = parse_identity(identity)
            matches = sorted({path for other, paths in owned.items()
                              if (got := parse_identity(other))["name"].lower() == wanted["name"].lower()
                              and got["token"] == wanted["token"]
                              and got["version"][0] == wanted["version"][0]
                              for path in paths if path != own_path})
            decision["same_major_candidates"] = matches
            if len(matches) == 1:
                decision.update(outcome="substituted", chosen=matches[0], basis="same-major")
            elif matches:
                decision["outcome"] = "ambiguous"
        decisions.append(decision)
    return decisions


def selection_counts(decisions):
    """Count decisions per outcome plus the identities a tie-break had to settle."""
    counts = {"selected": 0, "substituted": 0, "unresolved": 0, "ambiguous": 0}
    for decision in decisions:
        counts[decision["outcome"]] += 1
    counts["tie_broken"] = sum(decision["basis"] in ("directory-family", "non-legacy", "path-order")
                               for decision in decisions)
    return counts


def plan_references(owned, declared_index, own_path, policy, ambiguity, hop):
    """Select declared references, then with ``hop`` the references those files declare."""
    own_declared = declared_index.get(own_path, [])
    declared_selection = select_references(owned, own_declared, own_path, policy, ambiguity)
    hop1_selection = []
    if hop:
        chosen = {decision["chosen"] for decision in declared_selection if decision["chosen"]}
        inherited = sorted({identity for path in chosen for identity in declared_index.get(path, [])}
                           - set(own_declared))
        hop1_selection = select_references(owned, inherited, own_path, policy, ambiguity)
        hop1_selection = select_references(owned, inherited, own_path, policy, ambiguity)
    passed = []
    for decision in declared_selection + hop1_selection:
        if decision["chosen"] and decision["chosen"] not in passed:
            passed.append(decision["chosen"])
    return {"declared": declared_selection, "hop1": hop1_selection, "passed": passed}


_REFERENCE_INDEX = ({}, {})


def _init_worker(owned_identities, declared_references):
    global _REFERENCE_INDEX
    _REFERENCE_INDEX = (owned_identities, declared_references)


def scan_one(image, cli, cli_stamp, output, references_mode, reference_policy,
             reference_ambiguity, reference_hop, resume, timeout, scan_threads):
    """Scan one image, account for every known body, and return its record."""
    image_output = output / "images" / f"{image['id']:04d}"
    image_output.mkdir(parents=True, exist_ok=True)
    owned_identities, declared_references = _REFERENCE_INDEX
    selection = {"declared": [], "hop1": [], "passed": []}
    if references_mode == "declared":
        selection = plan_references(
            owned_identities, declared_references, image["path"],
            reference_policy, reference_ambiguity, reference_hop)
    selected = selection["passed"]
    stamp = {"cli": cli_stamp, "scan_threads": scan_threads, "image_sha256": image.get("sha256"),
             "inventory_mtime_ns": Path(image["inventory_path"]).stat().st_mtime_ns,
             "references": {"mode": references_mode,
                            "policy": reference_policy,
                            "ambiguity": reference_ambiguity,
                            "hop": reference_hop,
                            "selected": [d["chosen"] for d in selection["declared"] if d["chosen"]],
                            "hop1": [d["chosen"] for d in selection["hop1"] if d["chosen"]],
                            "selection": {"declared": selection["declared"],
                                          "hop1": selection["hop1"]}}}
    checkpoint = image_output / "summary.json"
    if resume and checkpoint.is_file():
        previous = json.loads(checkpoint.read_text())
        if previous.get("stamp") == stamp:
            return previous
    methods = json.loads(Path(image["inventory_path"]).read_text())["methods"]
    bodies = [m for m in methods if m["body_kind"] == "cil"]
    errors = []
    records = {}
    scan_started = time.monotonic()
    try:
        with (image_output / "stdout.ndjson").open("w") as stdout, (image_output / "stderr.log").open("w") as stderr:
            command = [str(cli), "scan", image["path"]]
            if scan_threads > 1:
                command += ["--threads", str(scan_threads)]
            if references_mode == "declared":
                command.append("--no-auto-references")
                for reference in selected:
                    command += ["--reference", reference]
            process = subprocess.run(command, stdout=stdout, stderr=stderr,
                                     timeout=timeout)
        if process.returncode:
            errors.append({"error": "scan process failed", "exit_code": process.returncode,
                           "diagnostic": (image_output / "stderr.log").read_text()[-4000:]})
        with (image_output / "stdout.ndjson").open() as stdout:
            for line in stdout:
                record = json.loads(line)
                if record["token"] in records:
                    errors.append({"token": record["token"], "error": "duplicate scan token"})
                records[record["token"]] = record
    except (OSError, subprocess.SubprocessError, ValueError) as error:
        errors.append({"error": str(error)})
    all_tokens = {m["token"] for m in methods}
    for token in sorted(set(records) - all_tokens):
        errors.append({"token": token, "error": "scan returned an unknown method"})
    for token in sorted(all_tokens - set(records)):
        errors.append({"token": token, "error": "method omitted by production scan"})
    for record in records.values():
        if record["status"] not in ("Decompiled", "Partial", "Failed", "Declaration"):
            errors.append({"token": record["token"], "error": "scan returned an unknown method status"})
    results, counts, reasons = [], {}, {}
    for method in bodies:
        record = records.get(method["token"])
        if record is None:
            status, reason = "Missing", "method omitted by production scan"
        elif record["status"] not in ("Decompiled", "Partial", "Failed"):
            status, reason = "Failed", "CIL body was reported as a declaration or unknown status"
            errors.append({"token": method["token"], "error": reason})
        else:
            status, reason = record["status"], record.get("reason")
            if status != "Decompiled" and not reason:
                reason = "unreported failure"
        counts[status] = counts.get(status, 0) + 1
        if status != "Decompiled":
            normalized = re.sub(r"\s+at IL_[0-9A-F]+$", "", reason)
            reasons[normalized] = reasons.get(normalized, 0) + 1
        results.append({"token": method["token"], "owner": method["owner"], "name": method["name"],
                        "status": status, "reason": reason})
    # Bodyless images have no rate, but parsing failures remain visible and block completion.
    results_path = image_output / "results.json"
    save(results_path, results)
    resolution = None
    for line in (image_output / "stderr.log").read_text().splitlines():
        match = re.match(r"references=(\d+) resolved=(\d+) failed=(\d+)$", line)
        if match:
            resolution = {"declared": int(match.group(1)),
                          "resolved": int(match.group(2)),
                          "failed": int(match.group(3))}
    result = {"id": image["id"], "assembly": image["assembly"], "origins": image["origins"],
              "bodies": len(bodies), "counts": counts, "reasons": reasons,
              "references": reference_record(references_mode, reference_policy, reference_ambiguity,
                                             reference_hop, selection,
                                             declared_references.get(image["path"], []), resolution),
              "scan_complete": not errors, "errors": errors,
              "results_path": str(results_path), "stamp": stamp,
              "elapsed_ms": round((time.monotonic() - scan_started) * 1000)}
    decision = gate_image(result)
    result["unrecovered"] = decision["unrecovered"]
    result["gate_pass"] = decision["pass"]
    result["rate"] = decision["unrecovered"] / len(bodies) if bodies else None
    save(checkpoint, result)
    print(f"[{image['id']:04d}] {image['assembly']} bodies={len(bodies)} "
          f"unrecovered={decision['unrecovered']} complete={not errors} ms={result['elapsed_ms']}", flush=True)
    return result


def reference_record(references_mode, reference_policy, reference_ambiguity, reference_hop,
                     selection, declared, resolution):
    """Summarize one image's declared and hop-1 selection for its result record."""
    declared_counts = selection_counts(selection["declared"])
    hop1_counts = selection_counts(selection["hop1"])
    return {"mode": references_mode, "policy": reference_policy,
            "ambiguity": reference_ambiguity, "hop": reference_hop,
            "declared": len(declared),
            "declared_identities": len(selection["declared"]),
            "selected": declared_counts["selected"],
            "unresolved": declared_counts["unresolved"],
            "ambiguous": declared_counts["ambiguous"],
            "substituted": declared_counts["substituted"],
            "tie_broken": declared_counts["tie_broken"],
            "hop1_declared": len(selection["hop1"]),
            "hop1_selected": hop1_counts["selected"],
            "hop1_unresolved": hop1_counts["unresolved"],
            "hop1_ambiguous": hop1_counts["ambiguous"],
            "hop1_substituted": hop1_counts["substituted"],
            "hop1_tie_broken": hop1_counts["tie_broken"],
            "passed": len(selection["passed"]),
            "resolution": resolution}


def baseline(args):
    started = time.monotonic()
    source = json.loads(args.inventory.read_text())
    if not source["complete"]:
        raise ValueError("independent inventory is incomplete; cannot establish a baseline")
    cli, output = args.cli.resolve(), args.output.resolve()
    if not cli.is_file():
        raise ValueError("existing production CLI is required")
    output.mkdir(parents=True, exist_ok=True)
    cli_stamp = {"path": str(cli), "mtime_ns": cli.stat().st_mtime_ns, "bytes": cli.stat().st_size}
    managed = [i for i in source["images"] if i["kind"] == "managed"]
    owned_identities, declared_references = ({}, {})
    if args.references == "declared":
        owned_identities, declared_references = build_identity_index(
            cli, [image["path"] for image in managed], args.timeout)
    payloads = [(image, cli, cli_stamp, output, args.references,
                 args.reference_version_policy, args.reference_ambiguity, args.reference_hop,
                 args.resume, args.timeout, args.scan_threads)
                for image in managed]
    # Process-based, not thread-based: the JSON accounting below is CPU-bound and
    # the GIL otherwise serializes it across workers.
    with ProcessPoolExecutor(max_workers=args.workers,
                             initializer=_init_worker,
                             initargs=(owned_identities, declared_references)) as pool:
        images = list(pool.map(scan_one, *zip(*payloads)))
    counts, reasons = {}, {}
    for image in images:
        for key, value in image["counts"].items():
            counts[key] = counts.get(key, 0) + value
        for key, value in image["reasons"].items():
            reasons[key] = reasons.get(key, 0) + value
    summary = {"managed_images": len(images), "body_images": sum(i["bodies"] > 0 for i in images),
               "bodies": sum(i["bodies"] for i in images), "counts": counts,
               "incomplete_images": sum(not i["scan_complete"] for i in images),
               "passing_body_images": sum(i["bodies"] > 0 and i["gate_pass"] for i in images)}
    report = {"schema": 1, "inventory": str(args.inventory.resolve()), "cli": cli_stamp,
              "complete": all(i["scan_complete"] for i in images), "images": images,
              "references": {"mode": args.references,
                             "policy": args.reference_version_policy,
                             "ambiguity": args.reference_ambiguity,
                             "hop": args.reference_hop},
              "summary": summary, "reasons": dict(sorted(reasons.items(), key=lambda item: -item[1])),
              "elapsed_ms": round((time.monotonic() - started) * 1000)}
    save(output / "baseline.json", report)
    print(json.dumps({"summary": summary, "elapsed_ms": report["elapsed_ms"]}))
    return 0 if report["complete"] and all(i["gate_pass"] for i in images) else 2 if report["complete"] else 1


def plan(args):
    """Compute the reference selection for every managed image without scanning any body."""
    started = time.monotonic()
    source = json.loads(args.inventory.read_text())
    if not source["complete"]:
        raise ValueError("independent inventory is incomplete; cannot plan references")
    cli = args.cli.resolve()
    if not cli.is_file():
        raise ValueError("existing production CLI is required")
    managed = [i for i in source["images"] if i["kind"] == "managed"]
    owned_identities, declared_references = build_identity_index(
        cli, [image["path"] for image in managed], args.timeout)
    images, detail = [], []
    for image in managed:
        selection = plan_references(owned_identities, declared_references, image["path"],
                                    args.reference_version_policy,
                                    args.reference_ambiguity, args.reference_hop)
        record = {"id": image["id"], "assembly": image["assembly"], "path": image["path"],
                  "origins": image["origins"], "bodies": image["counts"]["cil"],
                  "identity_error": image["path"] not in declared_references}
        record.update(reference_record("declared", args.reference_version_policy,
                                       args.reference_ambiguity, args.reference_hop, selection,
                                       declared_references.get(image["path"], []), None))
        images.append(record)
        detail.append({**record, "selection": selection})

    def totals(rows):
        keys = ("declared", "declared_identities", "selected", "unresolved", "ambiguous",
                "substituted", "tie_broken",
                "hop1_declared", "hop1_selected", "hop1_unresolved", "hop1_ambiguous",
                "hop1_substituted", "hop1_tie_broken", "passed")
        summed = {key: sum(row[key] for row in rows) for key in keys}
        summed["images"] = len(rows)
        summed["images_with_selection"] = sum(row["passed"] > 0 for row in rows)
        summed["images_with_hop1"] = sum(row["hop1_selected"] > 0 for row in rows)
        return summed

    summary = {"managed_images": len(images),
               "body_images": sum(row["bodies"] > 0 for row in images),
               "policy": args.reference_version_policy,
               "ambiguity": args.reference_ambiguity, "hop": args.reference_hop,
               "identity_error_images": sum(row["identity_error"] for row in images),
               "all_images": totals(images),
               "body_image_totals": totals([row for row in images if row["bodies"] > 0]),
               "elapsed_ms": round((time.monotonic() - started) * 1000)}
    if args.output:
        save(args.output.resolve() / "plan.json",
             {"schema": 1, "inventory": str(args.inventory.resolve()),
              "cli": {"path": str(cli), "mtime_ns": cli.stat().st_mtime_ns,
                      "bytes": cli.stat().st_size},
              "summary": summary, "images": detail})
    print(json.dumps({"summary": summary, "images": images}))
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    acquisition = commands.add_parser("collect", help="Enumerate a pinned checkout and bounded archive members")
    acquisition.add_argument("--checkout", type=Path, required=True)
    acquisition.add_argument("--commit", default=PINNED_COMMIT)
    acquisition.add_argument("--output", type=Path, required=True)
    acquisition.add_argument("--supplement", type=Path, action="append", default=[])
    acquisition.add_argument("--reuse-acquisition", type=Path)
    metadata = commands.add_parser("inventory", help="Independently enumerate all method rows using static PEReader")
    metadata.add_argument("--manifest", type=Path, required=True)
    metadata.add_argument("--output", type=Path, required=True)
    scan = commands.add_parser("baseline", help="Run the production CLI and account for every independently known body")
    scan.add_argument("--inventory", type=Path, required=True)
    scan.add_argument("--cli", type=Path, required=True)
    scan.add_argument("--output", type=Path, required=True)
    scan.add_argument("--workers", type=int, default=max(1, (os.cpu_count() or 4) // 2),
                      help="Images scanned concurrently; pair with --scan-threads so threads x workers fits the machine")
    scan.add_argument("--scan-threads", type=int, default=2,
                      help="Threads inside each scan process; each worker keeps its own fact caches")
    scan.add_argument("--timeout", type=int, default=300)
    scan.add_argument("--resume", action="store_true")
    scan.add_argument("--references", choices=("none", "declared"), default="none",
                      help="Pass exactly the references each image declares and record the selection")
    scan.add_argument("--reference-version-policy", choices=("exact", "same-major"),
                      default="exact")
    scan.add_argument("--reference-ambiguity", choices=("tie-break", "skip"), default="tie-break",
                      help="Pass one deterministic file when an identity is owned by several "
                           "identity-equal files, or pass none for those identities")
    scan.add_argument("--reference-hop", type=int, choices=(0, 1), default=0,
                      help="Also pass the references the selected files declare, one hop, non-recursive")
    dry = commands.add_parser("plan", help="Print the per-image reference selection without scanning")
    dry.add_argument("--inventory", type=Path, required=True)
    dry.add_argument("--cli", type=Path, required=True)
    dry.add_argument("--output", type=Path,
                     help="Optional directory receiving plan.json with every chosen path and candidate list")
    dry.add_argument("--timeout", type=int, default=300)
    dry.add_argument("--reference-version-policy", choices=("exact", "same-major"),
                     default="exact")
    dry.add_argument("--reference-ambiguity", choices=("tie-break", "skip"), default="tie-break")
    dry.add_argument("--reference-hop", type=int, choices=(0, 1), default=0)
    check = commands.add_parser("gate", help="Evaluate per-image integer <= 1% gates")
    check.add_argument("--baseline", type=Path, required=True)
    args = parser.parse_args()
    try:
        return {"collect": collect, "inventory": inventory, "baseline": baseline,
                "gate": gate, "plan": plan}[args.command](args)
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
