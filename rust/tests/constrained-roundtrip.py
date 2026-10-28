#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute recovered constrained operations and managed-location CFG joins."""
import argparse
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/value-dispatch/behavior"


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


support = module("object_roundtrip", ROOT / "tests/object-roundtrip.py")


def aliases(body):
    return [{"alias": name, "bytes": signature} for name, signature in
            re.findall(r"// structural signature (__cil_\w+) bytes=([0-9A-F]+):", body)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--mutation-control", action="store_true")
    args = parser.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    support.run([sys.executable, ROOT / "tests/build-constrained-fixtures.py"])
    harness = OUT / "ConstrainedRoundtrip.dll"
    support.compile_cs(harness, [ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"]
                       + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs")), executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/constrained-{config}.bin"
        methods, support_bodies, support_evidence, evidence = {}, {}, {}, {}
        gated = []
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            match = re.search(r"(ConstrainedCases\.(?:Ops|Ledger|Host\S*))::([^\s(<]+)", signature)
            if not match or match[2].startswith(".") or match[2] == "ArrayAnchor": continue
            body = support.run([args.cli, "decompile", fixture, "--method", token, "--no-auto-references"])
            if body.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0":
                raise ValueError("Owned constrained body not recovered: " + match[2] + body.stdout)
            if re.search(r"<\d+>\(", signature) or re.search(r"![0-9]", body.stdout):
                # The carrier is not generic, so a scope variable cannot be spelled.
                gated.append(match[2] + ": generic scope variable")
                continue
            (OUT / f"{config}-{token}.readable.txt").write_text(body.stdout)
            details = support.run([args.cli, "decompile", fixture, "--method", token, "--details"])
            if details.stderr != body.stderr:
                raise ValueError("view changed admission for " + token)
            (OUT / f"{config}-{token}.details.txt").write_text(details.stdout)
            evidence[int(token, 16)] = details.stdout
            if not match[1].endswith("Ledger"): methods[int(token, 16)] = body.stdout
            else:
                support_bodies[match[2]] = body.stdout
                support_evidence[match[2]] = details.stdout
        # The carrier is non-generic, so a method whose body still names a scope
        # variable is gated instead of silently dropped.
        expected = 11 - len([g for g in gated if not g.startswith(("Argument", "Replace"))])
        if len(methods) != expected or set(support_bodies) != {"Argument", "Replace"}:
            raise ValueError("Owned constrained selection drift")
        directives = []
        for token, body in methods.items():
            combined = body + support_bodies["Argument"]
            if "__cil_methodspec_call_" in body: combined += support_evidence["Replace"]
            directives.append({"token": token, "aliases": list({entry["alias"]: entry for entry in aliases(combined)}.values()),
                               "calls": [{"constraint": int(c, 16), "method": int(m, 16)} for c, m in
                                         sorted(set(re.findall(r"__cil_constrained_([0-9A-F]{8})_([0-9A-F]{8})", body)))],
                               "methodspecs": [int(t, 16) for t in sorted(set(re.findall(r"__cil_methodspec_call_([0-9A-F]{8})", body)))],
                               "directs": [int(t, 16) for t in sorted(set(re.findall(r"__cil_direct_([0-9A-F]{8})", body)))]})
        path = OUT / f"directives-{config}.json"
        support.write_changed(path, json.dumps(directives, indent=2))
        plan = json.loads(support.run([support.DOTNET / "dotnet", harness, fixture, "--constrained-plan", path]).stdout)
        plan_path = OUT / f"plan-{config}.json"
        support.write_changed(plan_path, json.dumps(plan, indent=2))
        pieces = []
        for case in plan:
            body = methods[case["token"]]
            body = case["header"] + body[body.index("\n{"):]
            body += "\n" + support_bodies["Argument"]
            if "__cil_methodspec_call_" in body:
                # The original Replace<T> method receives the same closed T that
                # its MethodSpec explicitly binds in this selected caller.
                replacement = support_bodies["Replace"]
                replacement = ("public static int Replace(ref " + next(
                    entry.split(" = ", 1)[1].rstrip(";") for entry in case["aliases"]
                    if "__cil_generic_" in entry) + " receiver, "
                    + next(entry.split(" = ", 1)[1].rstrip(";") for entry in case["aliases"]
                           if "__cil_generic_" in entry) + " replacement, int delta)"
                    + replacement[replacement.index("\n{"):])
                body += "\n" + replacement
            body = re.sub(r"\b(?:global::)?@?ConstrainedCases\.@?Ledger\.@?Argument\(", "Argument(", body)
            pieces.append("namespace " + case["carrier"] + " {\n" + "\n".join(case["aliases"])
                          + "\n public static class Recovered {\n" + "\n".join(case["helpers"]) + "\n" + body + "\n} }")
        source = OUT / f"Recovered-{config}.cs"
        support.write_changed(source, "\n".join(pieces))
        binary = source.with_suffix(".dll")
        support.compile_cs(binary, [source], references=[fixture])
        command = [support.DOTNET / "dotnet", harness, fixture, "--constrained-check", binary, plan_path]
        result = support.run(command)
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip()})
        if args.mutation_control:
            mutants = []
            for index, case in enumerate(plan):
                piece = pieces[index]
                if " Step(" in case["header"] and "ref global::ConstrainedCases.Counter receiver" in case["header"]:
                    pattern = r"(private static global::System.Int32 __cil_constrained_[0-9A-F_]+\(ref global::ConstrainedCases.Counter p0, global::System.Int32 p1\)) => Operation0.Value\(ref p0, p1\);"
                    changed, count = re.subn(pattern, r"\1 { global::ConstrainedCases.Counter copy = p0; return Operation0.Value(ref copy, p1); }", piece, count=1)
                    if count != 1: raise ValueError("Copied receiver mutation target missing")
                    mutants.append(("CopiedReceiver", index, changed))
                if " ArrayCarry(" in case["header"]:
                    needle = case["header"] + "\n{"
                    count = piece.count(needle)
                    changed = piece.replace(needle, needle + "\n values = (uint[])values.Clone();", 1)
                    if count != 1: raise ValueError("Array location copy mutation missing")
                    mutants.append(("ArrayCopy", index, changed))
                if " Conditional(" in case["header"]:
                    changed, count = re.subn(r"= ref right;", "= ref left;", piece, count=1)
                    if count != 1: raise ValueError("Wrong joined location mutation missing")
                    mutants.append(("WrongLocation", index, changed))
                if " Slot(" in case["header"] and "CounterClass" in case["header"]:
                    matches = list(re.finditer(r"= ref l0;", piece))
                    if len(matches) < 2: raise ValueError("Reference snapshot mutation missing")
                    target = matches[-1]
                    changed = piece[:target.start()] + "= ref receiver;" + piece[target.end():]
                    mutants.append(("SlotSnapshot", index, changed))
                if " Order(" in case["header"] and "CounterClass" in case["header"]:
                    changed, count = re.subn(r"(public static int Argument\(int delta\)\n\{)", r"\1\n global::ConstrainedCases.Ledger.Trace = 0;", piece, count=1)
                    if count != 1: raise ValueError("Argument state mutation missing")
                    mutants.append(("ArgumentTrace", index, changed))
            if len(mutants) != 5: raise ValueError("Constrained mutation selection drift")
            for label, index, changed in mutants:
                text = "\n".join(changed if i == index else piece for i, piece in enumerate(pieces))
                path = OUT / f"Wrong-{label}-{config}.cs"
                support.write_changed(path, text)
                mutant = path.with_suffix(".dll")
                support.compile_cs(mutant, [path], references=[fixture])
                negative = subprocess.run([str(x) for x in [*command[:4], mutant, plan_path]], text=True, capture_output=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Wrong-{label}-{config}.log").write_text(log)
                if negative.returncode == 0 or "Recovered constrained behavior differs:" not in log:
                    raise ValueError("Constrained semantic mutant survived: " + label + log)
                print(config + ": rejected " + label + " mutant", flush=True)
                rows.append({"configuration": config, "mutation": label, "detected": True})
    (OUT / "results.json").write_text(json.dumps({"rows": rows, "third_party_execution": False}, indent=2) + "\n")


if __name__ == "__main__":
    main()
