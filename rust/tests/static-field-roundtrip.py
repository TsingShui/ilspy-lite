#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute actual recovered static field operations on owned closed storage."""
import argparse
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys
ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/static-fields/behavior"
spec = importlib.util.spec_from_file_location("object_roundtrip", ROOT / "tests/object-roundtrip.py")
support = importlib.util.module_from_spec(spec)
spec.loader.exec_module(support)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--mutation-control", action="store_true")
    args = parser.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    support.run([sys.executable, ROOT / "tests/build-static-field-fixtures.py"])
    harness = OUT / "StaticFieldRoundtrip.dll"
    support.compile_cs(harness, [ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"]
                       + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs")), executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/static-field-{config}.bin"
        library = ROOT / f"tests/fixtures/static-field-library-{config}.bin"
        bodies, names, arguments_body = {}, {}, None
        directives, gated = [], []
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            found = re.search(r"StaticFieldCases\.(?:Ops|Slots\S*|MixedHost\S*)::([^\s(<]+)", signature)
            if not found or found[1].startswith(".") or found[1] == "ClosedInitializerAnchor": continue
            if re.search(r"<\d+>\(", signature):
                # The carrier is a non-generic static class, so a generic method
                # cannot be carried.
                gated.append(found[1])
                continue
            result = support.run([args.cli, "decompile", fixture, "--method", token, "--no-auto-references", "--reference", library])
            if result.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0":
                raise ValueError("Owned static field method not recovered: " + result.stdout)
            (OUT / f"{config}-{token}.readable.txt").write_text(result.stdout)
            if found[1] == "Argument":
                arguments_body = result.stdout.replace("public static", "private static", 1)
                continue
            bodies[int(token, 16)] = result.stdout
            names[int(token, 16)] = found[1]
            directives.append({"token": int(token, 16),
                "aliases": [{"alias": a, "bytes": b} for a, b in re.findall(r"// structural signature (__cil_\w+) bytes=([0-9A-F]+):", result.stdout)],
                "fields": [{"op": op, "token": int(t, 16)} for op, t in sorted(set(
                    re.findall(r"__cil_(ldsfld|stsfld|ldsflda)_([0-9A-F]{8})", result.stdout)))]})
        if len(bodies) != 24 - len(gated) or arguments_body is None:
            raise ValueError("Static field selection drift: gated=" + repr(gated))
        directive_path = OUT / f"directives-{config}.json"
        support.write_changed(directive_path, json.dumps(directives, indent=2))
        plan = json.loads(support.run([support.DOTNET / "dotnet", harness, fixture, "--static-field-plan", directive_path]).stdout)
        plan_path = OUT / f"plan-{config}.json"
        support.write_changed(plan_path, json.dumps(plan, indent=2))
        pieces = []
        for case in plan:
            body = bodies[case["token"]]
            body = case["header"] + body[body.index("\n{"):]
            if names[case["token"]].startswith("Init"):
                body = re.sub(r"\b(?:global::)?@?StaticFieldCases\.@?Ops\.@?Argument\(", "Argument(", body) + "\n" + arguments_body
            pieces.append("namespace " + case["carrier"] + " {\n" + "\n".join(case["aliases"])
                + "\n public static class Recovered {\n" + "\n".join(case["helpers"]) + "\n" + body + "\n} }")
        def compile_carrier(label, text):
            source = OUT / f"{label}-{config}.cs"
            support.write_changed(source, text)
            binary = source.with_suffix(".dll")
            support.compile_cs(binary, [source], references=[fixture, library])
            return binary
        binary = compile_carrier("Recovered", "\n".join(pieces))
        command = [support.DOTNET / "dotnet", harness, fixture, "--static-field-check", binary, plan_path]
        result = support.run(command)
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip()})
        def first_touch(binary, selected):
            checks, observations = 0, []
            for case in selected:
                name = names[case["token"]]
                phases = [(7, False), (7, True)] + ([(0, False)] if name in ["InitWrite", "InitAddress"] else [])
                for value, fail in phases:
                    outcomes = []
                    for recovered in [False, True]:
                        request = OUT / f"initialization-request-{config}.json"
                        support.write_changed(request, json.dumps({"row": case, "binary": str(binary), "recovered": recovered, "value": value, "fail": fail}))
                        outcomes.append(json.loads(support.run([support.DOTNET / "dotnet", harness, fixture, "--static-field-once", request]).stdout))
                    observations.append({"token": case["token"], "argument": case["argument"], "value": value, "fail": fail, "outcomes": outcomes})
                    if outcomes[0] != outcomes[1]:
                        raise ValueError("Recovered initialization differs: " + name + " " + repr(outcomes))
                    checks += 1
            (OUT / f"{binary.stem}-first-touch-{config}.json").write_text(json.dumps(observations, indent=2) + "\n")
            return checks
        initializers = [case for case in plan if names[case["token"]].startswith("Init")]
        count = first_touch(binary, initializers)
        print(config + ": " + str(count) + " fresh-process first-touch comparisons, two invocations each", flush=True)
        rows.append({"configuration": config, "first_touch_comparisons": count})
        if args.mutation_control:
            for label, name in [("EarlyInitialization", "InitWrite"), ("LateAddress", "InitAddress")]:
                index = next(i for i, case in enumerate(plan) if names[case["token"]] == name)
                changed = pieces[index]
                marker = plan[index]["header"] + "\n{"
                if label == "EarlyInitialization":
                    leaf = re.search(r"__cil_ldsfld_[0-9A-F]{8}\(\)", bodies[plan[index]["token"]])
                    if leaf is None or marker not in changed: raise ValueError("Early initialization mutation missing")
                    changed = changed.replace(marker, marker + "\n int __earlyInitialization = " + leaf[0] + ";", 1)
                else:
                    lines = changed.splitlines()
                    acquisition = next(i for i, line in enumerate(lines) if "= ref __cil_ldsflda_" in line)
                    argument = next(i for i, line in enumerate(lines) if "= Argument(" in line)
                    if "= value;" not in lines[argument - 1]: raise ValueError("Argument binding target missing")
                    block = lines[argument - 1:argument + 1]
                    del lines[argument - 1:argument + 1]
                    lines[acquisition:acquisition] = block
                    changed = "\n".join(lines)
                mutant = compile_carrier("Wrong-" + label, "\n".join(changed if i == index else piece for i, piece in enumerate(pieces)))
                try:
                    first_touch(mutant, [plan[index]])
                except ValueError as error:
                    if "Recovered initialization differs: " + name not in str(error): raise
                    (OUT / f"Wrong-{label}-{config}.log").write_text(str(error) + "\n")
                else:
                    raise ValueError("Initialization semantic mutant survived: " + label)
                print(config + ": rejected " + label, flush=True)
                rows.append({"configuration": config, "mutation": label, "detected": True})
            mixed_index = next(i for i, case in enumerate(plan) if names[case["token"]] == "ReadMixed"
                and case["type_argument"].startswith("System.Int32,") and case["argument"].startswith("System.String,"))
            mixed = pieces[mixed_index]
            old = "new System.Type[]{typeof(global::System.String)}"
            if old not in mixed: raise ValueError("Closed method scope mutation missing")
            mixed = mixed.replace(old, "new System.Type[]{typeof(global::System.Object)}", 1)
            mutant = compile_carrier("Wrong-ClosedStorage", "\n".join(mixed if i == mixed_index else piece for i, piece in enumerate(pieces)))
            negative = subprocess.run([str(x) for x in [*command[:4], mutant, plan_path]], capture_output=True, text=True, timeout=120)
            log = negative.stdout + negative.stderr
            (OUT / f"Wrong-ClosedStorage-{config}.log").write_text(log)
            if negative.returncode == 0 or "Recovered static field differs:" not in log:
                raise ValueError("Closed storage semantic mutant survived: " + log)
            print(config + ": rejected ClosedStorage", flush=True)
            rows.append({"configuration": config, "mutation": "ClosedStorage", "detected": True})
            copy_index = next(i for i, case in enumerate(plan) if names[case["token"]] == "Copy" and case["argument"].startswith("StaticFieldCases.Pair,"))
            copy = pieces[copy_index]
            marker = plan[copy_index]["header"] + "\n{"
            prefix, method = copy.split(marker, 1)
            leaf = re.search(r"__cil_ldsfld_[0-9A-F]{8}\(\)", bodies[plan[copy_index]["token"]])[0]
            method, count = re.subn(r"\breturn\s+\w+;", "return " + leaf + ";", method, count=1)
            if count != 1: raise ValueError("Static copy snapshot mutation missing")
            copy = prefix + marker + method
            mutant = compile_carrier("Wrong-CopySnapshot", "\n".join(copy if i == copy_index else piece for i, piece in enumerate(pieces)))
            negative = subprocess.run([str(x) for x in [*command[:4], mutant, plan_path]], capture_output=True, text=True, timeout=120)
            log = negative.stdout + negative.stderr
            (OUT / f"Wrong-CopySnapshot-{config}.log").write_text(log)
            if negative.returncode == 0 or "Recovered static field differs:" not in log:
                raise ValueError("Static copy snapshot mutant survived: " + log)
            print(config + ": rejected CopySnapshot", flush=True)
            rows.append({"configuration": config, "mutation": "CopySnapshot", "detected": True})
            thread_index = next(i for i, case in enumerate(plan) if names[case["token"]] == "ReadThread")
            thread = pieces[thread_index].replace("il.Emit(System.Reflection.Emit.OpCodes.Ldsfld, target);",
                'target = module.Assembly.GetType("StaticFieldCases.NonThread").GetField("Shared");il.Emit(System.Reflection.Emit.OpCodes.Ldsfld, target);', 1)
            if thread == pieces[thread_index]: raise ValueError("Thread storage mutation missing")
            mutant = compile_carrier("Wrong-ThreadStorage", "\n".join(thread if i == thread_index else piece for i, piece in enumerate(pieces)))
            negative = subprocess.run([str(x) for x in [support.DOTNET / "dotnet", harness, fixture, "--static-field-threads", mutant, plan_path]], capture_output=True, text=True, timeout=120)
            log = negative.stdout + negative.stderr
            (OUT / f"Wrong-ThreadStorage-{config}.log").write_text(log)
            if negative.returncode == 0 or "Recovered thread static differs." not in log:
                raise ValueError("Thread storage semantic mutant survived: " + log)
            print(config + ": rejected ThreadStorage", flush=True)
            rows.append({"configuration": config, "mutation": "ThreadStorage", "detected": True})
            changes = {
                "WrongAddress": ("il.Emit(System.Reflection.Emit.OpCodes.Ldsflda, target);",
                    'if (target.Name == "Value" && target.FieldType == typeof(int)) target = target.DeclaringType.GetField("Counter");'
                    "il.Emit(System.Reflection.Emit.OpCodes.Ldsflda, target);"),
                "DropStore": ("il.Emit(System.Reflection.Emit.OpCodes.Stsfld, target);", "il.Emit(System.Reflection.Emit.OpCodes.Pop);"),
                "WrongStorage": ("il.Emit(System.Reflection.Emit.OpCodes.Ldsfld, target);",
                    'if (target.Name == "Value" && target.FieldType == typeof(int)) target = target.DeclaringType.GetField("Counter");'
                    "il.Emit(System.Reflection.Emit.OpCodes.Ldsfld, target);")}
            for label, (old, new) in changes.items():
                text = "\n".join(pieces)
                if old not in text: raise ValueError("Static field mutant target missing")
                mutant = compile_carrier("Wrong-" + label, text.replace(old, new))
                negative = subprocess.run([str(x) for x in [*command[:4], mutant, plan_path]], capture_output=True, text=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Wrong-{label}-{config}.log").write_text(log)
                if negative.returncode == 0 or "Recovered static field differs:" not in log:
                    raise ValueError("Static field semantic mutant survived: " + label + log)
                print(config + ": rejected " + label, flush=True)
                rows.append({"configuration": config, "mutation": label, "detected": True})
    (OUT / "results.json").write_text(json.dumps({"rows": rows, "third_party_execution": False}, indent=2) + "\n")

if __name__ == "__main__":
    main()
