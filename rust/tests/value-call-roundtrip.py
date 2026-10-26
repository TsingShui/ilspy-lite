#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute recovered constructed-value calls with real typed receiver locations."""
import argparse
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/value-calls/behavior"


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


support = module("object_roundtrip", ROOT / "tests/object-roundtrip.py")
generic = module("generic_declaring", ROOT / "tests/generic-declaring-roundtrip.py")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--mutation-control", action="store_true")
    args = parser.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    support.run([sys.executable, ROOT / "tests/build-value-call-fixtures.py"])
    harness = OUT / "ValueCallRoundtrip.dll"
    support.compile_cs(harness, [ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"]
                       + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs")), executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    results = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/value-call-{config}.bin"
        library = ROOT / f"tests/fixtures/value-call-library-{config}.bin"
        names = {}
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            match = re.search(r"(ValueCallCases\.(?:Ops|Probe|Cell\S*))::([^\s(<]+)", signature)
            if match and (not match[2].startswith(".") or match[2] == ".ctor"): names[token] = (match[1], match[2])
        bodies, plan = {}, []
        for row in map(json.loads, support.run([args.cli, "scan", fixture, "--no-auto-references", "--reference", library]).stdout.splitlines()):
            if row["token"] not in names: continue
            owner, name = names[row["token"]]
            if row["status"] != "Decompiled": raise ValueError("Owned value call not recovered: " + owner + "." + name)
            text = support.run([args.cli, "decompile", fixture, "--method", row["token"], "--no-auto-references", "--reference", library]).stdout
            (OUT / f"{config}-{row['token']}.readable.txt").write_text(text)
            text = generic.inline_structural_types(text).replace("!!0", "T").replace("!0", "T")
            text = re.sub(r"\b(?:global::)?@?ValueCallCases\.@?Ops\.", "ValueCallCases.Recovered.", text)
            text = re.sub(r"\b(?:global::)?@?ValueCallCases\.@?Probe\.@?Argument\(", "ValueCallCases.Recovered.ProbeArgument(", text)
            if owner == "ValueCallCases.Probe":
                text = re.sub(r"\bArgument\(", "ProbeArgument(", text, count=1)
                name = "ProbeArgument"
            if owner.startswith("ValueCallCases.Cell"):
                if name == ".ctor": name = "Ctor"
                parameters = {"Ctor": "ref global::ValueCallCases.Cell<T> receiver, T value", "Read": "ref global::ValueCallCases.Cell<T> receiver", "Write": "ref global::ValueCallCases.Cell<T> receiver, T value",
                              "Echo": "T value", "ReadThrow": "ref global::ValueCallCases.Cell<T> receiver, int divisor"}[name]
                returns = "void" if name in ["Write", "Ctor"] else "int" if name == "ReadThrow" else "T"
                text = "public static " + returns + " Cell" + name + "<T>(" + parameters + ")" + text[text.index("\n{"):]
                text = "\n".join(line if line.lstrip().startswith("//") else re.sub(r"\bthis\b", "receiver", line) for line in text.splitlines())
                name = "Cell" + name
            calls = sorted(set(re.findall(r"__cil_value_call_([0-9A-F]{8})", text)))
            constructors = sorted(set(re.findall(r"__cil_value_newobj_([0-9A-F]{8})", text)))
            plan.append({"caller": row["token"][2:], "calls": calls, "constructors": constructors})
            executable = "\n".join(line for line in text.splitlines() if not line.strip().startswith("//"))
            executable = re.sub(r"__cil_value_(?:call|newobj)_[0-9A-F]{8}", "", executable)
            if "__cil_" in support.without_boolean_helpers(executable): raise ValueError("Unbound value call body: " + name)
            bodies[name] = text
        if not bodies or not all(name in bodies for name in ["CellRead", "CellWrite", "Generic", "ForeignGeneric", "Throw"]):
            raise ValueError("Owned value support selection incomplete")
        plan_file = OUT / f"plan-{config}.json"
        support.write_changed(plan_file, json.dumps(plan))
        helpers = support.run([support.DOTNET / "dotnet", harness, fixture, "--value-call-plan", plan_file, library]).stdout
        def compile_carrier(label, methods):
            source = OUT / f"{label}-{config}.cs"
            support.write_changed(source, "namespace ValueCallCases { public static class Recovered {\n" + helpers
                                  + "\n".join(methods.values()) + "\n} }\n")
            binary = source.with_suffix(".dll")
            support.compile_cs(binary, [source], references=[fixture, library])
            return binary
        binary = compile_carrier("Recovered", bodies)
        command = [support.DOTNET / "dotnet", harness, fixture, "--value-call-check", binary, library]
        result = support.run(command)
        print(config + ": " + result.stdout.strip(), flush=True)
        results.append({"configuration": config, "behavior": result.stdout.strip()})
        if args.mutation_control:
            for name, replacement in [("CellRead", "receiver.Count = 0;"), ("CellWrite", "receiver.Value = default;"),
                                       ("CellReadThrow", "receiver.Count = 0;")]:
                changed = dict(bodies)
                if name == "CellWrite":
                    changed[name], count = re.subn(r"return;", replacement + " return;", changed[name], count=1)
                else:
                    changed[name], count = re.subn(r"\n\{", "\n{\n" + replacement, changed[name], count=1)
                if count != 1: raise ValueError("Value-call mutation target missing")
                mutant = compile_carrier("Wrong-" + name, changed)
                negative = subprocess.run([str(x) for x in [*command[:4], mutant, library]], text=True, capture_output=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Wrong-{name}-{config}.log").write_text(log)
                if negative.returncode == 0 or "Recovered value call differs:" not in log:
                    raise ValueError("Value-call semantic mutant survived: " + name + log)
                print(config + ": rejected " + name + " mutant", flush=True)
                results.append({"configuration": config, "mutation": name, "detected": True})
            for label, name, result in [("DelayedBox", "CopyBox", "(object)value"), ("UnboxValue", "Unbox", "default")]:
                changed = dict(bodies)
                changed[name], count = re.subn(r"\breturn\s+[^;]+;", "return " + result + ";", changed[name], count=1)
                if count != 1: raise ValueError("Boxing mutation missing")
                mutant = compile_carrier("Wrong-" + label, changed)
                negative = subprocess.run([str(x) for x in [*command[:4], mutant, library]], text=True, capture_output=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Wrong-{label}-{config}.log").write_text(log)
                if negative.returncode == 0 or "Recovered value call differs:" not in log:
                    raise ValueError("Boxing semantic mutant survived: " + label + log)
                print(config + ": rejected " + label + " mutant", flush=True)
                results.append({"configuration": config, "mutation": label, "detected": True})
            for label, statement in [("CtorCount", "receiver.Count = 0;"), ("CtorValue", "receiver.Value = default;")]:
                changed = dict(bodies)
                changed["CellCtor"], count = re.subn(r"return;", statement + " return;", changed["CellCtor"], count=1)
                if count != 1: raise ValueError("Constructor mutation missing")
                mutant = compile_carrier("Wrong-" + label, changed)
                negative = subprocess.run([str(x) for x in [*command[:4], mutant, library]], text=True, capture_output=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Wrong-{label}-{config}.log").write_text(log)
                if negative.returncode == 0 or "Recovered value call differs:" not in log:
                    raise ValueError("Constructor semantic mutant survived: " + label + log)
                print(config + ": rejected " + label + " mutant", flush=True)
                results.append({"configuration": config, "mutation": label, "detected": True})
            for label, name, statement in [("EarlyCtorThrow", "CellCtor", "int early = 100 / global::ValueCallCases.Probe.Divisor;"),
                                            ("ArgumentTrace", "ProbeArgument", "global::ValueCallCases.Probe.Trace = 0;")]:
                changed = dict(bodies)
                changed[name], count = re.subn(r"\n\{", "\n{\n" + statement, changed[name], count=1)
                if count != 1: raise ValueError("Constructor effect mutation missing")
                mutant = compile_carrier("Wrong-" + label, changed)
                negative = subprocess.run([str(x) for x in [*command[:4], mutant, library]], text=True, capture_output=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Wrong-{label}-{config}.log").write_text(log)
                if negative.returncode == 0 or "Recovered value call differs:" not in log:
                    raise ValueError("Constructor effect mutant survived: " + label + log)
                print(config + ": rejected " + label + " mutant", flush=True)
                results.append({"configuration": config, "mutation": label, "detected": True})
            changed = dict(bodies)
            changed["CellRead"] = changed["CellRead"].replace("ref receiver", "ref copied")
            changed["CellRead"], count = re.subn(r"\n\{", "\n{\n global::ValueCallCases.Cell<T> copied = receiver;", changed["CellRead"], count=1)
            if count != 1: raise ValueError("Copied receiver mutation missing")
            mutant = compile_carrier("Wrong-CopiedReceiver", changed)
            negative = subprocess.run([str(x) for x in [*command[:4], mutant, library]], text=True, capture_output=True, timeout=120)
            log = negative.stdout + negative.stderr
            (OUT / f"Wrong-CopiedReceiver-{config}.log").write_text(log)
            if negative.returncode == 0 or "Recovered value call differs:" not in log:
                raise ValueError("Copied receiver mutant survived: " + log)
            print(config + ": rejected CopiedReceiver mutant", flush=True)
            results.append({"configuration": config, "mutation": "CopiedReceiver", "detected": True})
    (OUT / "results.json").write_text(json.dumps({"rows": results, "third_party_execution": False}, indent=2) + "\n")


if __name__ == "__main__":
    main()
