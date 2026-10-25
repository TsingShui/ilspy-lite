#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Check the frozen class/member inventory using an independent .NET 10 reader.

This is static inspection, not a lowering or behavioral gate. Inputs and their
recorded provenance are reused; generated metadata stays in the ignored report.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess

RUST = Path(__file__).resolve().parents[2]


def identities(snapshot):
    result = {"root": snapshot["root"], "mvid": snapshot["mvid"]}
    result["types"] = [t["token"] for t in snapshot["types"]]
    for kind in ("fields", "methods", "properties", "events"):
        result[kind] = [member["token"] for t in snapshot["types"] for member in t[kind]]
    for kind in ("types", "fields", "methods", "properties", "events"):
        if len(result[kind]) != len(set(result[kind])):
            raise ValueError(f"Duplicate {kind} identity in metadata snapshot")
    return result


def check_output(snapshot, text, stderr, code):
    """Compare the public class output with independently read member metadata."""
    sections = re.split(r"^// type token (0x02[0-9A-F]{6}) .*\n", text, flags=re.M)
    actual_types = sections[1::2]
    expected_types = [t["token"] for t in snapshot["types"]]
    if len(actual_types) != len(set(actual_types)) or set(actual_types) != set(expected_types):
        raise ValueError("Missing, extra or duplicate selected type")
    blocks = dict(zip(actual_types, sections[2::2]))
    statuses = {}

    def require(condition, reason):
        if not condition:
            raise ValueError(reason)

    def raw(blob):
        return "signature-bytes=[" + ", ".join(blob[i:i + 2] for i in range(0, len(blob), 2)) + "]"

    def members(block, keyword, expected):
        matches = re.findall(r"^\." + keyword + r" (0x[0-9A-F]{8})(?: |$)([^\n]*)", block, re.M)
        require(len(matches) == len({t for t, _ in matches}), f"Duplicate {keyword} declaration")
        require({t for t, _ in matches} == {m["token"] for m in expected}, f"Incomplete {keyword} inventory")
        return dict(matches)

    def constants(block, parent, constant):
        if constant is None:
            return
        encoded = "bytes=[" + ", ".join(constant["value"][i:i + 2]
                                        for i in range(0, len(constant["value"]), 2)) + "]"
        require(any(line.startswith(".constant ") and f"parent={parent} " in line and encoded in line
                    for line in block.splitlines()), f"Missing constant: {parent}")

    def generics(block, owner, expected):
        matches = re.findall(r"^\.generic (0x2A[0-9A-F]{6}) owner=" + owner +
                             r" number=(\d+) flags=0x([0-9A-F]{4}) ", block, re.M)
        require(sorted(matches) == sorted((g["token"], str(g["index"]), f"{g['flags']:04X}")
                                         for g in expected), f"Generic context mismatch: {owner}")
        for g in expected:
            actual = re.findall(r"^\.constraint .* owner=" + g["token"] +
                                r" type=(0x[0-9A-F]{8})(?: |$)", block, re.M)
            require(actual == g["constraints"], f"Generic constraints mismatch: {g['token']}")

    for ty in snapshot["types"]:
        block = blocks[ty["token"]]
        require(f".type-flags 0x{ty['flags']:08X} " in block, "Type flags mismatch")
        parent = ty["parent"]
        require((f".enclosing-type {parent} " in block) if parent != "0x00000000"
                else ".enclosing-type " not in block, "Enclosing type mismatch")
        base = ty["base_type"]
        require(".extends none\n" in block if base == "0x00000000" else f".extends {base} " in block,
                "Base type mismatch")
        require(re.findall(r"^\.implements (0x[0-9A-F]{8}) ", block, re.M) == ty["interfaces"],
                "Interface inventory mismatch")
        layouts = re.findall(r"^\.class-layout (0x0F[0-9A-F]{6}) parent=(0x02[0-9A-F]{6}) "
                             r"packing=(\d+) size=(\d+) raw-row-bytes=\[([^]]*)\]$", block, re.M)
        expected_layouts = [(layout["token"], layout["parent"], str(layout["packing"]), str(layout["size"]),
                             ", ".join(layout["raw_row"][i:i + 2] for i in range(0, len(layout["raw_row"]), 2)))
                            for layout in ty["class_layout"]]
        require(layouts == expected_layouts, f"Class layout record mismatch: {ty['token']}")
        generics(block, ty["token"], ty["generic_parameters"])
        fields = members(block, "field", ty["fields"])
        for field in ty["fields"]:
            line = fields[field["token"]]
            require(f"flags=0x{field['flags']:04X} " in line and raw(field["signature"]) in line,
                    f"Field declaration mismatch: {field['token']}")
            constants(block, field["token"], field["constant"])
        methods = members(block, "method", ty["methods"])
        points = list(re.finditer(r"^\.method-status (0x06[0-9A-F]{6}) (\w+)\n", block, re.M))
        require(len(points) == len(methods) and len({p[1] for p in points}) == len(points),
                "Method status inventory mismatch")
        for method in ty["methods"]:
            token = method["token"]
            line = methods[token]
            require(f"flags=0x{method['flags']:04X} impl-flags=0x{method['impl_flags']:04X} " in line
                    and raw(method["signature"]) in line, f"Method declaration mismatch: {token}")
            status = next((p[2] for p in points if p[1] == token), None)
            require(status in ("Decompiled", "Declaration", "Partial", "Failed"), f"Invalid status: {token}")
            statuses[token] = status
            method_start = next(p.end() for p in points if p[1] == token)
            body = re.split(r"^\.method 0x", block[method_start:], maxsplit=1, flags=re.M)[0]
            require(status != "Declaration" or not method["has_rva"], f"Body hidden as declaration: {token}")
            if status == "Decompiled":
                require("IL FALLBACK" not in body and "// FAILED" not in body and "public static " in body,
                        f"Fallback counted as decompiled: {token}")
            elif status == "Partial":
                require("// IL FALLBACK:" in body and ".code-bytes " in body, f"Missing fallback: {token}")
            params = re.findall(r"^\.param (0x08[0-9A-F]{6}) method=" + token +
                                r" sequence=(\d+) flags=0x([0-9A-F]{4}) ", block, re.M)
            require(params == [(p["token"], str(p["sequence"]), f"{p['flags']:04X}")
                               for p in method["parameters"]], f"Parameter metadata mismatch: {token}")
            for param in method["parameters"]:
                constants(block, param["token"], param["constant"])
            generics(block, token, method["generic_parameters"])
        for keyword, entries in (("property", ty["properties"]), ("event", ty["events"])):
            declarations = members(block, keyword, entries)
            for entry in entries:
                token = entry["token"]
                line = declarations[token]
                require(f"flags=0x{entry['flags']:04X} " in line, f"Flags mismatch: {token}")
                if keyword == "property":
                    require(raw(entry["signature"]) in line, f"Property signature mismatch: {token}")
                    targets = [("getter", entry["getter"]), ("setter", entry["setter"])]
                    constants(block, token, entry["constant"])
                else:
                    require("type=" + entry["event_type"] + " " in line, f"Event type mismatch: {token}")
                    targets = [("adder", entry["adder"]), ("remover", entry["remover"]), ("fire", entry["raiser"])]
                relations = re.findall(r"^\.accessor .* association=" + token +
                                       r" semantics=0x[0-9A-F]{4} (\w+) method=(0x[0-9A-F]{8}) ", block, re.M)
                expected = [(kind, target) for kind, target in targets if target != "0x00000000"]
                expected += [("other", target) for target in entry["others"]]
                require(sorted(relations) == sorted(expected), f"Accessor relationships mismatch: {token}")
    counts = {"decompiled": 0, "declarations": 0, "partial": 0, "failed": 0}
    keys = {"Decompiled": "decompiled", "Declaration": "declarations", "Partial": "partial", "Failed": "failed"}
    for status in statuses.values():
        counts[keys[status]] += 1
    require("// FAILED metadata" not in text, "Class metadata failed")
    require(stderr.strip() == " ".join(f"{k}={v}" for k, v in counts.items()), "CLI summary mismatch")
    require(code == (1 if counts["failed"] else 2 if counts["partial"] else 0), "CLI exit mismatch")
    require(counts["failed"] == 0, "Class has failed bodies")
    return counts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=RUST / "report/openra")
    parser.add_argument("--output", type=Path, default=RUST / "report/openra/class-inventory")
    parser.add_argument("--check-cli", action="store_true", help="compare public --type output, not just input metadata")
    parser.add_argument("--cli", type=Path, default=RUST / "target/release/ilspy-lite")
    args = parser.parse_args()
    dotnet = Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10"))

    def latest(path):
        return max((p for p in path.iterdir() if p.name.startswith("10.")),
                   key=lambda p: tuple(map(int, p.name.split("."))))

    sdk = latest(dotnet / "sdk")
    ref = latest(dotnet / "packs/Microsoft.NETCore.App.Ref") / "ref/net10.0"
    runtime = latest(dotnet / "shared/Microsoft.NETCore.App")
    args.output.mkdir(parents=True, exist_ok=True)
    oracle = args.output / "ClassInventory.dll"
    response = oracle.with_suffix(".rsp")
    response.write_text("\n".join([
        "-nologo", "-target:exe", "-langversion:14", "-warnaserror+",
        f'-out:"{oracle}"', *[f'-r:"{p}"' for p in sorted(ref.glob("*.dll"))],
        f'"{RUST / "tests/dotnet/ClassInventory.cs"}"',
    ]))
    oracle.with_suffix(".runtimeconfig.json").write_text(json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    subprocess.run([str(dotnet / "dotnet"), str(sdk / "Roslyn/bincore/csc.dll"), f"@{response}"],
                   check=True, timeout=120)
    manifest = json.loads((RUST / "acceptance/openra/classes.json").read_text())
    report = []
    for entry in manifest["classes"]:
        result = subprocess.run([str(dotnet / "dotnet"), str(oracle),
                                 str(args.root / "assemblies" / entry["assembly"]), entry["name"]],
                                check=True, text=True, capture_output=True, timeout=30)
        snapshot = json.loads(result.stdout)
        actual = identities(snapshot)
        (args.output / f"{entry['id']}.json").write_text(result.stdout)
        if entry.get("frozen_inventory") != actual:
            raise ValueError(f"Frozen member inventory mismatch: {entry['name']}")
        if not set(entry["required_methods"]).issubset(actual["methods"]):
            raise ValueError(f"Required methods missing: {entry['name']}")
        if args.check_cli:
            result = subprocess.run([str(args.cli), "decompile", str(args.root / "assemblies" / entry["assembly"]),
                                     "--type", entry["name"]], text=True, capture_output=True, timeout=60)
            (args.output / f"{entry['id']}.class.txt").write_text(result.stdout)
            counts = check_output(snapshot, result.stdout, result.stderr, result.returncode)
            report.append({"id": entry["id"], "counts": counts, "exit": result.returncode})
            print(f"{entry['name']}: complete class output matches; {counts}")
        else:
            print(f"{entry['name']}: frozen class/member inventory matches; no method executed")
    if args.check_cli:
        (args.output / "class-report.json").write_text(json.dumps({"classes": report,
            "behavior": "separate gate; not performed by inventory.py"}, indent=2) + "\n")


if __name__ == "__main__":
    main()
