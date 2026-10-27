#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute default recovered typed vector bodies on owned inputs."""
import argparse
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/typed-arrays/behavior"
spec = importlib.util.spec_from_file_location("object_roundtrip", ROOT / "tests/object-roundtrip.py")
support = importlib.util.module_from_spec(spec)
spec.loader.exec_module(support)

def main():
    global OUT
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--mutation-control", action="store_true")
    parser.add_argument("--legacy-enums", action="store_true")
    args = parser.parse_args()
    if args.legacy_enums: OUT = ROOT / "report/typed-arrays/legacy-enums"
    family = "enum" if args.legacy_enums else "typed-array"
    OUT.mkdir(parents=True, exist_ok=True)
    support.run([sys.executable, ROOT / f"tests/build-{family}-fixtures.py"])
    harness = OUT / "TypedArrayRoundtrip.dll"
    support.compile_cs(harness, [ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"]
                       + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs")), executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/{family}-{config}.bin"
        bodies, directives = {}, []
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            pattern = r"EnumCases\.[SU](?:8|16|32|64)Ops::(UnsupportedArray)\(" if args.legacy_enums else r"TypedArrayCases\.(?:Ops|Owner[^:]*)::([A-Za-z][A-Za-z0-9_]*)"
            match = re.search(pattern, signature)
            if not match: continue
            result = support.run([args.cli, "decompile", fixture, "--method", token, "--no-auto-references"])
            if result.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0":
                raise ValueError("Owned vector method not recovered: " + result.stdout)
            (OUT / f"{config}-{token}.readable.txt").write_text(result.stdout)
            bodies[int(token, 16)] = result.stdout
            if match[1] == "Touch": continue
            directives.append({"token": int(token, 16),
                "aliases": [{"alias": a, "bytes": b} for a, b in re.findall(r"// structural signature (__cil_\w+) bytes=([0-9A-F]+):", result.stdout)],
                "allocations": [int(t, 16) for t in sorted(set(re.findall(r"__cil_newarr_([0-9A-F]{8})", result.stdout)))],
                "elements": [{"op": op, "token": int(t, 16)} for op, t in sorted(set(re.findall(r"__cil_(ldelem|ldelema|stelem)_([0-9A-F]{8})", result.stdout)))]})
        if len(bodies) != (8 if args.legacy_enums else 42): raise ValueError("Typed vector selection drift")
        path = OUT / f"directives-{config}.json"
        support.write_changed(path, json.dumps(directives, indent=2))
        plan = json.loads(support.run([support.DOTNET / "dotnet", harness, fixture, "--typed-array-plan", path]).stdout)
        plan_path = OUT / f"plan-{config}.json"
        support.write_changed(plan_path, json.dumps(plan, indent=2))
        pieces = []
        support_body = "" if args.legacy_enums else next((body for body in bodies.values() if re.search(r"^(?:public|private) static object Touch\(", body)), None)
        if support_body is None: raise ValueError("Recovered owned argument helper missing")
        for case in plan:
            body = bodies[case["token"]]
            body = case["header"] + body[body.index("\n{"):]
            if "Order(" in case["header"]:
                body = re.sub(r"(?:global::)?@?TypedArrayCases\.@?Ops\.@?Touch\(", "Touch(", body) + "\n" + support_body
            pieces.append("namespace " + case["carrier"] + " {\n" + "\n".join(case["aliases"])
                + "\n public static class Recovered {\n" + "\n".join(case["helpers"]) + "\n" + body + "\n} }")
        def compile_carrier(label, text):
            source = OUT / f"{label}-{config}.cs"
            if "__cil_F" in text: text = "using __cil_F = System.Single;\nusing static CilFloat;\n" + text
            support.write_changed(source, text)
            binary = source.with_suffix(".dll")
            support.compile_cs(binary, [source] + ([ROOT / "tests/dotnet/FloatSemantics.cs"] if "__cil_F" in text else []), references=[fixture])
            return binary
        text = "\n".join(pieces)
        binary = compile_carrier("Recovered", text)
        command = [support.DOTNET / "dotnet", harness, fixture, "--typed-array-check", binary, plan_path]
        result = support.run(command)
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip()})
        if args.mutation_control and args.legacy_enums:
            changed = pieces[0].replace("il.Emit(System.Reflection.Emit.OpCodes.Ldarg_0);",
                "il.Emit(System.Reflection.Emit.OpCodes.Ldarg_0);il.Emit(System.Reflection.Emit.OpCodes.Ldc_I4_1);il.Emit(System.Reflection.Emit.OpCodes.Add);", 1)
            if changed == pieces[0]: raise ValueError("Enum allocation mutation missing")
            mutant = compile_carrier("Wrong-EnumLength", "\n".join([changed, *pieces[1:]]))
            negative = subprocess.run([str(x) for x in [*command[:4], mutant, plan_path]], capture_output=True, text=True, timeout=120)
            log = negative.stdout + negative.stderr
            (OUT / f"Wrong-EnumLength-{config}.log").write_text(log)
            if negative.returncode == 0 or "Recovered vector behavior differs:" not in log:
                raise ValueError("Enum allocation mutant survived: " + log)
            print(config + ": rejected EnumLength", flush=True)
            rows.append({"configuration": config, "mutation": "EnumLength", "detected": True})
        if args.mutation_control and not args.legacy_enums:
            index = next(i for i, case in enumerate(plan) if " ReadObject(" in case["header"])
            marker = plan[index]["header"] + "\n{"
            prefix, method = pieces[index].split(marker, 1)
            method, count = re.subn(r"\breturn\s+\w+;", "return new object();", method, count=1)
            if count != 1: raise ValueError("Reference identity mutation missing")
            changed = prefix + marker + method
            mutant = compile_carrier("Wrong-ReferenceIdentity", "\n".join(changed if i == index else piece for i, piece in enumerate(pieces)))
            negative = subprocess.run([str(x) for x in [*command[:4], mutant, plan_path]], capture_output=True, text=True, timeout=120)
            log = negative.stdout + negative.stderr
            (OUT / f"Wrong-ReferenceIdentity-{config}.log").write_text(log)
            if negative.returncode == 0 or "Recovered vector behavior differs:" not in log:
                raise ValueError("Reference vector identity mutant survived: " + log)
            print(config + ": rejected ReferenceIdentity", flush=True)
            rows.append({"configuration": config, "mutation": "ReferenceIdentity", "detected": True})
            store_index = next(i for i, case in enumerate(plan) if " StoreObject(" in case["header"])
            marker = plan[store_index]["header"] + "\n{"
            prefix, method = pieces[store_index].split(marker, 1)
            method, count = re.subn(r"(?m)^\s*\w+\[\w+\] = [^;]+;", "", method, count=1)
            if count != 1: raise ValueError("Reference store mutation missing")
            changed = prefix + marker + method
            mutant = compile_carrier("Wrong-DroppedStore", "\n".join(changed if i == store_index else piece for i, piece in enumerate(pieces)))
            negative = subprocess.run([str(x) for x in [*command[:4], mutant, plan_path]], capture_output=True, text=True, timeout=120)
            log = negative.stdout + negative.stderr
            (OUT / f"Wrong-DroppedStore-{config}.log").write_text(log)
            if negative.returncode == 0 or "Recovered vector behavior differs:" not in log:
                raise ValueError("Reference vector store mutant survived: " + log)
            print(config + ": rejected DroppedStore", flush=True)
            rows.append({"configuration": config, "mutation": "DroppedStore", "detected": True})
            for label, method_name in [("CopiedAddress", "AddressPair"), ("ValueSnapshot", "ExchangePair")]:
                index = next(i for i, case in enumerate(plan) if " " + method_name + "(" in case["header"])
                changed = pieces[index]
                if label == "CopiedAddress":
                    changed, count = re.subn(r"(\.Value\()values,index\);", r"\1(global::TypedArrayCases.Pair[])values.Clone(),index);", changed, count=1)
                else:
                    marker = plan[index]["header"] + "\n{"
                    prefix, method = changed.split(marker, 1)
                    loaded = re.search(r"__cil_ldelem_[0-9A-F]{8}\([^)]+\)", bodies[plan[index]["token"]])
                    if loaded is None: raise ValueError("Value snapshot operation missing")
                    method, count = re.subn(r"\breturn\s+\w+;", "return " + loaded[0] + ";", method, count=1)
                    changed = prefix + marker + method
                if count != 1: raise ValueError("Typed element mutation target missing: " + label)
                mutant = compile_carrier("Wrong-" + label, "\n".join(changed if i == index else piece for i, piece in enumerate(pieces)))
                negative = subprocess.run([str(x) for x in [*command[:4], mutant, plan_path]], capture_output=True, text=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Wrong-{label}-{config}.log").write_text(log)
                if negative.returncode == 0 or "Recovered vector behavior differs:" not in log:
                    raise ValueError("Typed element semantic mutant survived: " + label + log)
                print(config + ": rejected " + label, flush=True)
                rows.append({"configuration": config, "mutation": label, "detected": True})
            for label, method_name, pattern, replacement in [
                ("BooleanStorage", "StoreBool", r"__cil_bool_store\((\w+)\)", r"__cil_bool_store(\1 == 0 ? 0 : 1)"),
                ("FloatStorage", "StoreFloat", r"__cil_store_r4\((\w+)\)", r"__cil_store_r4(\1 == 0 ? 0 : \1)")]:
                index = next(i for i, case in enumerate(plan) if " " + method_name + "(" in case["header"])
                changed, count = re.subn(pattern, replacement, pieces[index], count=1)
                if count != 1: raise ValueError("Scalar storage mutation missing: " + label)
                mutant = compile_carrier("Wrong-" + label, "\n".join(changed if i == index else piece for i, piece in enumerate(pieces)))
                negative = subprocess.run([str(x) for x in [*command[:4], mutant, plan_path]], capture_output=True, text=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Wrong-{label}-{config}.log").write_text(log)
                if negative.returncode == 0 or "Recovered vector behavior differs:" not in log:
                    raise ValueError("Scalar vector storage mutant survived: " + label + log)
                print(config + ": rejected " + label, flush=True)
                rows.append({"configuration": config, "mutation": label, "detected": True})
            for label, method_name in [("LateAcquisition", "AddressOrder"), ("ArgumentEffect", "StoreOrder")]:
                index = next(i for i, case in enumerate(plan) if " " + method_name + "(" in case["header"])
                marker = plan[index]["header"] + "\n{"
                prefix, method = pieces[index].split(marker, 1)
                if label == "LateAcquisition":
                    acquisition = re.search(r"(?m)^\s*ref object (\w+) = ref __cil_ldelema_[0-9A-F]{8}\([^;]+;\n(?:\s*\w+ = ref \1;\n)?", method)
                    if acquisition is None: raise ValueError("Acquisition mutation operation missing")
                    line = acquisition[0]
                    method = method[:acquisition.start()] + method[acquisition.end():]
                    trace = re.search(r"(?m)^.*TypedArrayCases\.@?Ops\.@?Trace = \w+;\n", method)
                    if trace is None: raise ValueError("Acquisition trace store missing")
                    method = method[:trace.end()] + line + method[trace.end():]
                else:
                    method, count = re.subn(r"Touch\((\w+),\s*__cil_bool_store\(\w+\)\)", r"\1", method, count=1)
                    if count != 1: raise ValueError("Argument effect mutation call missing")
                changed = prefix + marker + method
                mutant = compile_carrier("Wrong-" + label, "\n".join(changed if i == index else piece for i, piece in enumerate(pieces)))
                negative = subprocess.run([str(x) for x in [*command[:4], mutant, plan_path]], capture_output=True, text=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Wrong-{label}-{config}.log").write_text(log)
                if negative.returncode == 0 or "Recovered vector behavior differs:" not in log:
                    raise ValueError("Ordered vector mutant survived: " + label + log)
                print(config + ": rejected " + label, flush=True)
                rows.append({"configuration": config, "mutation": label, "detected": True})
            allocation_index = next(i for i, case in enumerate(plan) if " NewByte(" in case["header"])
            marker = plan[allocation_index]["header"] + "\n{"
            prefix, method = pieces[allocation_index].split(marker, 1)
            match = re.search(r"\breturn\s+(\w+);", method)
            if match is None: raise ValueError("Allocation return mutation missing")
            for label in ["AllocationLength", "ZeroInitialization", "AllocationIdentity"]:
                if label == "AllocationLength":
                    changed = pieces[allocation_index].replace("il.Emit(System.Reflection.Emit.OpCodes.Ldarg_0);",
                        "il.Emit(System.Reflection.Emit.OpCodes.Ldarg_0);il.Emit(System.Reflection.Emit.OpCodes.Ldc_I4_1);il.Emit(System.Reflection.Emit.OpCodes.Add);", 1)
                elif label == "ZeroInitialization":
                    changed = prefix + marker + method.replace(match[0], "if (" + match[1] + ".Length != 0) " + match[1] + "[0] = 77; " + match[0], 1)
                else:
                    changed = prefix + "private static readonly byte[] __cached = new byte[0];\n" + marker + method.replace(match[0], "return length == 0 ? __cached : " + match[1] + ";", 1)
                mutant = compile_carrier("Wrong-" + label, "\n".join(changed if i == allocation_index else piece for i, piece in enumerate(pieces)))
                negative = subprocess.run([str(x) for x in [*command[:4], mutant, plan_path]], capture_output=True, text=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Wrong-{label}-{config}.log").write_text(log)
                if negative.returncode == 0 or "Recovered vector behavior differs:" not in log:
                    raise ValueError("Allocation semantic mutant survived: " + label + log)
                print(config + ": rejected " + label, flush=True)
                rows.append({"configuration": config, "mutation": label, "detected": True})
            for label, replacement in [("Length", "unchecked((nint)(values.LongLength + 1))"),
                                        ("Null", "values == null ? 0 : unchecked((nint)(values.LongLength))")]:
                changed, count = re.subn(r"unchecked\(\(nint\)\([^;\n]+?\.LongLength\)\)", replacement, text, count=1)
                if count != 1: raise ValueError("Vector length mutation target missing")
                mutant = compile_carrier("Wrong-" + label, changed)
                negative = subprocess.run([str(x) for x in [*command[:4], mutant, plan_path]], capture_output=True, text=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Wrong-{label}-{config}.log").write_text(log)
                if negative.returncode == 0 or "Recovered vector behavior differs:" not in log:
                    raise ValueError("Typed vector semantic mutant survived: " + label + log)
                print(config + ": rejected " + label, flush=True)
                rows.append({"configuration": config, "mutation": label, "detected": True})
    (OUT / "results.json").write_text(json.dumps({"rows": rows, "third_party_execution": False}, indent=2) + "\n")

if __name__ == "__main__":
    main()
