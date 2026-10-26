#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute recovered exception identities and exact construction."""
import argparse
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys
ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/exception-types/behavior"
spec = importlib.util.spec_from_file_location("object_roundtrip", ROOT / "tests/object-roundtrip.py")
support = importlib.util.module_from_spec(spec)
spec.loader.exec_module(support)

def main():
    global OUT
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--mutation-control", action="store_true")
    parser.add_argument("--legacy", action="store_true")
    args = parser.parse_args()
    if args.legacy: OUT = OUT / "legacy"
    OUT.mkdir(parents=True, exist_ok=True)
    support.run([sys.executable, ROOT / ("tests/build-exception-fixtures.py" if args.legacy else "tests/build-exception-type-fixtures.py")])
    harness = OUT / "ExceptionTypeRoundtrip.dll"
    support.compile_cs(harness, [ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"]
                       + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs")), executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    results = []
    for config in ("release", "debug"):
        prefix = "exception" if args.legacy else "exception-type"
        fixture = ROOT / f"tests/fixtures/{prefix}-{config}.bin"
        bodies = {}
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            found = re.search(r"(?:ExceptionTypeCases\.Ops|ExceptionCases.Cells)::([^\s(]+)", signature)
            expected = ["UnsupportedType", "UnsupportedOverload"] if args.legacy else ["ThrowBase", "ThrowSystem", "Rethrow", "Construct", "Range", "ThrowLocal", "MatchLocal", "ConstructLocal", "Io", "Format", "CatchIo", "Order", "TextArg", "InnerArg"]
            if not found or found[1] not in expected: continue
            result = support.run([args.cli, "decompile", fixture, "--method", token, "--no-auto-references"])
            if result.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0": raise ValueError("Exception method not recovered: " + result.stdout)
            (OUT / f"{config}-{found[1]}.readable.txt").write_text(result.stdout)
            body = result.stdout
            if found[1] in ["TextArg", "InnerArg"]: body = body.replace("public static", "private static", 1)
            for helper in ["TextArg", "InnerArg"]:
                body = re.sub(r"\b(?:global::)?@?ExceptionTypeCases\.@?Ops\.@?" + helper + r"\(", helper + "(", body)
            bodies[found[1]] = body
        if len(bodies) != (2 if args.legacy else 14): raise ValueError("Exception selection drift")
        tokens = sorted({int(token, 16) for text in bodies.values() for token in re.findall(r"__cil_newobj_([0-9A-F]{8})", text)})
        plan = OUT / f"constructors-{config}.json"
        support.write_changed(plan, json.dumps(tokens))
        helpers = support.run([support.DOTNET / "dotnet", harness, fixture, "--exception-type-plan", plan]).stdout
        def compile_carrier(label, methods):
            source = OUT / f"{label}-{config}.cs"
            support.write_changed(source, "namespace ExceptionTypeCases { public static class Recovered { public static int Trace;\n"
                                  + helpers + "\n".join(methods.values()) + "\n} }")
            binary = source.with_suffix(".dll")
            support.compile_cs(binary, [source], references=[fixture])
            return binary
        binary = compile_carrier("Recovered", bodies)
        command = [support.DOTNET / "dotnet", harness, fixture, "--exception-type-legacy" if args.legacy else "--exception-type-check", binary]
        result = support.run(command)
        print(config + ": " + result.stdout.strip(), flush=True)
        results.append({"configuration": config, "behavior": result.stdout.strip()})
        if args.mutation_control and not args.legacy:
            for label, name in [("ThrowIdentity", "ThrowBase"), ("InnerIdentity", "Construct"), ("RangeOrder", "Range"), ("ArgumentOrder", "Order"), ("MessageIdentity", "Construct")]:
                changed = dict(bodies)
                if name == "ThrowBase": changed[name], count = re.subn(r"throw value;", 'throw new System.Exception(value == null ? null : value.Message);', changed[name], count=1)
                elif name == "Construct" and label == "InnerIdentity": changed[name], count = re.subn(r", inner\)", ", null)", changed[name], count=1)
                elif name == "Construct": changed[name], count = re.subn(r"\(message, inner\)", "(message == null ? null : new string(message.ToCharArray()), inner)", changed[name], count=1)
                elif name == "Order":
                    lines = changed[name].splitlines()
                    targets = [i for i, line in enumerate(lines) if "= TextArg(" in line]
                    if len(targets) != 2: raise ValueError("Constructor argument trace target missing")
                    lines[targets[0]], lines[targets[1]] = lines[targets[1]], lines[targets[0]]
                    changed[name] = "\n".join(lines)
                    count = 1
                else:
                    changed[name], count = re.subn(r"(= __cil_newobj_[0-9A-F]{8}\()parameter, actual, message", r"\1message, actual, parameter", changed[name], count=1)
                if count != 1: raise ValueError("Exception mutation target missing: " + label)
                mutant = compile_carrier("Wrong-" + label, changed)
                negative = subprocess.run([str(x) for x in [*command[:4], mutant]], text=True, capture_output=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Wrong-{label}-{config}.log").write_text(log)
                if negative.returncode == 0 or "Recovered exception behavior differs:" not in log: raise ValueError("Exception semantic mutant survived: " + label + log)
                print(config + ": rejected " + label, flush=True)
                results.append({"configuration": config, "mutation": label, "detected": True})
    (OUT / "results.json").write_text(json.dumps({"rows": results, "third_party_execution": False}, indent=2) + "\n")

if __name__ == "__main__":
    main()
