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


def readable_alias(name):
    """The public readable spelling: ASCII identifier or UTF-8 hexadecimal alias."""
    return name if re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", name) else "__name_" + name.encode("utf-8").hex().upper()


def readable_type_alias(ty):
    name = ty["name"].rsplit("/", 1)[-1] if "/" in ty["name"] else ty["name"].rsplit(".", 1)[-1]
    return readable_alias(name.split("`", 1)[0])


def signature_counts(blob):
    """Read only the ECMA-335 signature prefix, not parameter types or bindings."""
    data = bytes.fromhex(blob)
    offset = 1

    def compressed():
        nonlocal offset
        first = data[offset]
        offset += 1
        if first < 0x80:
            return first
        width = 1 if first < 0xC0 else 3 if first < 0xE0 else 0
        if not width or offset + width > len(data):
            raise ValueError("Invalid signature count encoding")
        value = first & (0x3F if width == 1 else 0x1F)
        for _ in range(width):
            value = (value << 8) | data[offset]
            offset += 1
        return value

    try:
        generic = compressed() if data[0] & 0x10 else 0
        return compressed(), generic
    except IndexError as error:
        raise ValueError("Truncated signature count prefix") from error


def declaration_arity(text):
    # Ignore commas in literals, generic arguments, array ranks and nested defaults.
    masked = re.sub(r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'', '""', text)
    stack, count = [], 1 if masked.strip() else 0
    for char in masked:
        if char in "<([":
            stack.append(char)
        elif char in ">)]":
            if not stack or stack.pop() != {">": "<", ")": "(", "]": "["}[char]:
                raise ValueError("Unbalanced readable declaration list")
        elif char == "," and not stack:
            count += 1
    if stack:
        raise ValueError("Unclosed readable declaration list")
    return count


def readable_declarations(text):
    """Extract only the renderer's line/indentation declaration tree, not C# semantics."""
    lines = text.splitlines()
    type_pattern = r"^(?:(?:public|private|protected|internal|static|abstract|sealed) )*(?:class|struct|interface|enum) (@?[A-Za-z_][A-Za-z_0-9]*)(?:<([^>]*)>)?"

    def at(index, indent):
        return index < len(lines) and lines[index] == " " * indent + "{"

    def parse_type(index, indent):
        header = re.search(type_pattern, lines[index].strip())
        if not header or not at(index + 1, indent):
            raise ValueError("Readable type header/opening brace missing")
        node = {"name": header[1].lstrip("@"), "generic_arity": declaration_arity(header[2] or ""),
                "fields": [], "methods": [], "properties": [], "events": [], "children": []}
        index += 2
        while index < len(lines):
            raw = lines[index]
            value = raw.strip()
            if not value:
                index += 1
                continue
            depth = len(raw) - len(raw.lstrip(" "))
            if value == "}" and depth == indent:
                return node, index + 1
            if depth != indent + 4:
                raise ValueError(f"Readable owner indentation mismatch at line {index + 1}")
            if value.startswith("//"):
                unreadable = re.fullmatch(r"// Unreadable field (0x04[0-9A-F]{6}); see --details\.", value)
                relation = re.fullmatch(r"// (property|event) (.*?) \{ (.*) \}", value)
                if unreadable:
                    node["fields"].append({"token_marker": unreadable[1]})
                elif relation:
                    declaration, endpoints = relation[2], relation[3]
                    indexer = re.search(r"\bthis\[(.*)\]$", declaration)
                    name = "this" if indexer else declaration.rsplit(" ", 1)[-1].lstrip("@")
                    matches = re.findall(r"(get|set|add|remove|raise|other): (@?[A-Za-z_][A-Za-z_0-9]*);", endpoints)
                    if " ".join(f"{kind}: {alias};" for kind, alias in matches) != endpoints:
                        raise ValueError("Unreadable accessor relationship endpoints")
                    node["properties" if relation[1] == "property" else "events"].append({
                        "name": name, "index_parameters": declaration_arity(indexer[1]) if indexer else 0,
                        "endpoints": sorted((kind, alias.lstrip("@")) for kind, alias in matches)})
                elif "Unreadable" in value:
                    raise ValueError("Unreadable declaration lacks an identifying token marker")
                index += 1
                continue
            if re.search(type_pattern, value):
                child, index = parse_type(index, indent + 4)
                node["children"].append(child)
                continue
            # A field can contain '(' in a quoted constant; mask literals before finding a method header.
            masked = re.sub(r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'', '""', value)
            method = re.search(r"(@?[A-Za-z_][A-Za-z_0-9]*)(?:<([^<>]*)>)?\(([^()]*)\)", masked)
            if method:
                body = not value.endswith(";")
                node["methods"].append({"name": method[1].lstrip("@"),
                    "parameters": declaration_arity(method[3]), "generic_arity": declaration_arity(method[2] or ""),
                    "static": "static" in masked[:method.start()].split(), "body_present": body})
                index += 1
                if body:
                    if not at(index, indent + 4):
                        raise ValueError("Readable method opening brace missing")
                    index += 1
                    while index < len(lines) and lines[index] != " " * (indent + 4) + "}":
                        if lines[index].strip() and len(lines[index]) - len(lines[index].lstrip(" ")) <= indent + 4:
                            raise ValueError("Readable method body/owner boundary mismatch")
                        index += 1
                    if index == len(lines):
                        raise ValueError("Readable method closing brace missing")
                    index += 1
                continue
            field = re.search(r" (@?[A-Za-z_][A-Za-z_0-9]*)$", value.split(" = ", 1)[0].rstrip(";"))
            if not value.endswith(";") or not field:
                raise ValueError(f"Unrecognized readable declaration: {value}")
            node["fields"].append({"name": field[1].lstrip("@")})
            index += 1
        raise ValueError("Readable type closing brace missing")

    roots, index = [], 0
    while index < len(lines):
        value = lines[index].strip()
        if not value or value.startswith("//") or value.startswith("namespace "):
            index += 1
            continue
        if lines[index].startswith(" "):
            raise ValueError("Readable root is incorrectly nested")
        root, index = parse_type(index, 0)
        roots.append(root)
    return roots


def check_readable_inventory(snapshot, text):
    """Require a per-owner declaration multiset; method overload types stay details-only."""
    types = snapshot["types"]
    roots = readable_declarations(text)
    output = {"scope": "static declaration presence/tree only; no semantic C# parsing", "types": []}

    def compare(node, ty, parent):
        token = ty["token"]
        record = {"token": token, "parent": parent, "name": node["name"]}
        expected_name = readable_type_alias(ty)
        marker = node["name"] != expected_name and node["name"] == "__type_" + token[2:]
        if not marker and (node["name"] != expected_name or node["generic_arity"] != len(ty["generic_parameters"])):
            raise ValueError(f"Readable type name/generic arity mismatch: {token}")

        def consume(kind, expected, key):
            actual = list(node[kind])
            for member in expected:
                marker = next((value for value in actual if value.get("token_marker") == member["token"]), None)
                match = marker or next((value for value in actual if key(value) == key(member)), None)
                if match is None:
                    raise ValueError(f"Readable {kind} missing/mismatched in owner {token}: {member['token']}")
                actual.remove(match)
            if actual:
                raise ValueError(f"Extra/duplicate readable {kind} in owner {token}")
            record[kind] = [member["token"] for member in expected]

        fields = []
        for field in ty["fields"]:
            name = field["name"]
            alias = "__il_field_" + field["token"][2:] if not re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", name) or name.startswith("__il_field_") else name
            fields.append({"token": field["token"], "name": alias})
        consume("fields", fields, lambda value: value.get("name"))
        methods = []
        for method in ty["methods"]:
            parameters, generic = signature_counts(method["signature"])
            if generic != len(method["generic_parameters"]):
                raise ValueError(f"Independent method generic arity inconsistent: {method['token']}")
            methods.append({"token": method["token"], "name": expected_name if method["name"] in (".ctor", ".cctor") else readable_alias(method["name"]),
                "parameters": parameters, "generic_arity": generic, "static": bool(method["flags"] & 0x10),
                "body_present": method["has_rva"]})
        for actual in node["methods"]:
            placeholder = re.fullmatch(r"__method_(06[0-9A-F]{6})", actual["name"])
            if placeholder and not any(method["name"] == actual["name"] for method in methods):
                actual["token_marker"] = "0x" + placeholder[1]
                expected = next((m for m in methods if m["token"] == actual["token_marker"]), None)
                if expected and actual["body_present"] != expected["body_present"]:
                    raise ValueError("Unreadable method marker lost its body/declaration boundary")
        consume("methods", methods, lambda value: tuple(value.get(k) for k in ("name", "parameters", "generic_arity", "static", "body_present")))
        record["observed_fields"] = node["fields"]
        record["observed_methods"] = node["methods"]
        record["unreadable_type_token_marker"] = marker
        method_names = {method["token"]: readable_alias(method["name"]) for method in ty["methods"]}
        for kind in ("properties", "events"):
            expected = []
            for member in ty[kind]:
                targets = (("get", "getter"), ("set", "setter")) if kind == "properties" else (("add", "adder"), ("remove", "remover"), ("raise", "raiser"))
                endpoints = [(label, method_names[member[key]]) for label, key in targets if member[key] != "0x00000000"]
                endpoints += [("other", method_names[target]) for target in member["others"]]
                count = signature_counts(member["signature"])[0] if kind == "properties" else 0
                expected.append({"token": member["token"], "name": "this" if count else readable_alias(member["name"]),
                                 "index_parameters": count, "endpoints": sorted(endpoints)})
            consume(kind, expected, lambda value: (value["name"], value["index_parameters"], value["endpoints"]))
        record["observed_relationships"] = {kind: node[kind] for kind in ("properties", "events")}
        output["types"].append(record)
        children = [child for child in types if child["parent"] == token]
        if len(children) != len(node["children"]):
            raise ValueError(f"Readable owned nested type count mismatch: {token}")
        remaining = list(children)
        for child in node["children"]:
            candidates = [entry for entry in remaining if child["name"] in (
                readable_type_alias(entry), "__type_" + entry["token"][2:])]
            if len(candidates) != 1:
                raise ValueError(f"Readable child name/parent mismatch: {token}")
            remaining.remove(candidates[0])
            compare(child, candidates[0], token)

    if len(roots) != 1:
        raise ValueError("Readable selected root count mismatch")
    root = next(ty for ty in types if ty["token"] == snapshot["root"])
    compare(roots[0], root, root["parent"])
    if len(output["types"]) != len(types):
        raise ValueError("Readable declaration tree lost selected types")
    return output


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
                    if snapshot:
                        config["readable_inventory"] = check_readable_inventory(snapshot, stdout)
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
