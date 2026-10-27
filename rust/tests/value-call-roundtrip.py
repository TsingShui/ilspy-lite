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
    global OUT
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--mutation-control", action="store_true")
    parser.add_argument("--reinitialize", action="store_true")
    args = parser.parse_args()
    if args.reinitialize: OUT = ROOT / "report/local-value-construction/reinitialize-behavior"
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
        if args.reinitialize:
            patched = OUT / f"reinitialized-{config}.bin"
            print(support.run([support.DOTNET / "dotnet", harness, fixture, "--value-reinit-fixture", patched]).stdout, end="", flush=True)
            fixture = patched
        names = {}
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            match = re.search(r"(ValueCallCases\.(?:Ops|Probe|Plain|Cell\S*))::([^\s(<]+)", signature)
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
            text = re.sub(r"\b(?:global::)?@?ValueCallCases\.@?Probe\.@?(ArgumentTag|Argument)\(",
                lambda match: "ValueCallCases.Recovered.Probe" + match[1] + "(", text)
            if owner == "ValueCallCases.Probe":
                text = re.sub(r"\b" + name + r"\(", "Probe" + name + "(", text, count=1)
                name = "Probe" + name
            if owner == "ValueCallCases.Plain":
                if name != ".ctor": raise ValueError("Unmapped ordinary value support")
                parameters = re.search(r"\bPlain\(([^()]*)\)", text)
                if parameters is None: raise ValueError("Ordinary constructor parameters missing")
                name = "PlainCtor_" + row["token"][2:]
                text = "public static void " + name + "(ref global::ValueCallCases.Plain receiver, " + parameters[1] + ")" + text[text.index("\n{"):]
                text = "\n".join(line if line.lstrip().startswith("//") else re.sub(r"\bthis\b", "receiver", line) for line in text.splitlines())
                text = re.sub(r"(?m)^\s*(\w+)\.@?Value = (\w+);", r"    __PlainValueStore(ref \1, \2);", text)
            if owner.startswith("ValueCallCases.Cell"):
                if name == ".ctor": name = "Ctor"
                parameters = {"Ctor": "ref global::ValueCallCases.Cell<T> receiver, T value", "Read": "ref global::ValueCallCases.Cell<T> receiver", "Write": "ref global::ValueCallCases.Cell<T> receiver, T value",
                              "Echo": "T value", "ReadThrow": "ref global::ValueCallCases.Cell<T> receiver, int divisor"}[name]
                returns = "void" if name in ["Write", "Ctor"] else "int" if name == "ReadThrow" else "T"
                text = "public static " + returns + " Cell" + name + "<T>(" + parameters + ")" + text[text.index("\n{"):]
                text = "\n".join(line if line.lstrip().startswith("//") else re.sub(r"\bthis\b", "receiver", line) for line in text.splitlines())
                name = "Cell" + name
            calls = sorted(set(re.findall(r"__cil_value_call_([0-9A-F]{8})", text)))
            generic_calls = sorted(set(re.findall(r"__cil_methodspec_call_([0-9A-F]{8})", text)))
            field_addresses = sorted(set(re.findall(r"__cil_ldflda_([0-9A-F]{8})", text)))
            constructors = sorted(set(re.findall(r"__cil_value_newobj_([0-9A-F]{8})", text)))
            initializers = sorted(set(re.findall(r"__cil_value_ctor_([0-9A-F]{8})", text)))
            plan.append({"caller": row["token"][2:], "calls": calls, "generic_calls": generic_calls,
                         "field_addresses": field_addresses, "constructors": constructors, "initializers": initializers})
            executable = "\n".join(line for line in text.splitlines() if not line.strip().startswith("//"))
            executable = re.sub(r"__cil_(?:value_(?:call|newobj|ctor)|methodspec_call|ldflda)_[0-9A-F]{8}", "", executable)
            if "__cil_" in support.without_boolean_helpers(executable): raise ValueError("Unbound value call body: " + name)
            bodies[name] = text
        if not bodies or not all(name in bodies for name in ["CellRead", "CellWrite", "Generic", "ForeignGeneric", "Throw",
                                                               "ForeignExchange", "ForeignExchangeString", "ForeignExchangeCopy",
                                                               "ForeignExchangeAliased", "ForeignExchangeOrder", "ForeignExchangeGeneric", "ForeignExchangeCross"]):
            raise ValueError("Owned value support selection incomplete")
        plan_file = OUT / f"plan-{config}.json"
        support.write_changed(plan_file, json.dumps(plan))
        helpers = support.run([support.DOTNET / "dotnet", harness, fixture, "--value-call-plan", plan_file, library]).stdout
        helpers += """
private delegate void PlainValueStore(ref global::ValueCallCases.Plain receiver, int value);
private static readonly System.Lazy<PlainValueStore> __plainValueStore = new System.Lazy<PlainValueStore>(() => {
    var owner = typeof(global::ValueCallCases.Plain);
    var field = owner.GetField("Value");
    if (field == null || !field.IsInitOnly || field.FieldType != typeof(int)) throw new System.Exception("Owned readonly value field differs.");
    var method = new System.Reflection.Emit.DynamicMethod("owned-readonly-value-store", typeof(void), new[] { owner.MakeByRefType(), typeof(int) }, owner.Module, true);
    var il = method.GetILGenerator(); il.Emit(System.Reflection.Emit.OpCodes.Ldarg_0); il.Emit(System.Reflection.Emit.OpCodes.Ldarg_1);
    il.Emit(System.Reflection.Emit.OpCodes.Stfld, field); il.Emit(System.Reflection.Emit.OpCodes.Ret);
    return (PlainValueStore)method.CreateDelegate(typeof(PlainValueStore));
});
private static void __PlainValueStore(ref global::ValueCallCases.Plain receiver, int value) => __plainValueStore.Value(ref receiver, value);
"""
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
            for label in ["GenericReceiverCopy", "GenericArgumentCopy", "GenericFieldAlias", "GenericOrder", "GenericException", "GenericResult"]:
                name = "ForeignExchangeAliased" if label == "GenericFieldAlias" else "ForeignExchangeOrder" if label == "GenericOrder" else "ForeignExchange"
                changed = dict(bodies)
                if label == "GenericReceiverCopy":
                    changed[name], count = re.subn(r"= ref value;", "= ref copiedReceiver;", changed[name], count=1)
                    changed[name] = changed[name].replace("\n{", "\n{\n ValueCallLibrary.Cell<int> copiedReceiver = value;", 1)
                elif label == "GenericArgumentCopy":
                    changed[name], count = re.subn(r"= ref input;", "= ref copiedArgument;", changed[name], count=1)
                    changed[name] = changed[name].replace("\n{", "\n{\n int copiedArgument = input;", 1)
                elif label == "GenericFieldAlias":
                    changed[name], count = re.subn(r"ref int (\w+) = ref (__cil_ldflda_[0-9A-F]{8}\(ref \w+\));",
                        r"int copiedField = \2; ref int \1 = ref copiedField;", changed[name], count=1)
                elif label == "GenericOrder":
                    pattern = r"int (\w+) = (ValueCallLibrary\.Probe\.Argument\((\w+)\));\n(\s*)int (\w+) = (__cil_methodspec_call_[0-9A-F]{8}\(ref \w+, ref \w+, )\1(\));"
                    changed[name], count = re.subn(pattern, r"int \5 = \6\3\7;\n\4\2;", changed[name], count=1)
                elif label == "GenericException":
                    changed[name], count = re.subn(r"int (\w+) = divisor;", r"int \1 = 1;", changed[name], count=1)
                else:
                    changed[name], count = re.subn(r"\breturn \w+;", "return 0;", changed[name], count=1)
                if count != 1: raise ValueError("Generic value-call mutation missing: " + label)
                mutant = compile_carrier("Wrong-" + label, changed)
                negative = subprocess.run([str(x) for x in [*command[:4], mutant, library]], text=True, capture_output=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Wrong-{label}-{config}.log").write_text(log)
                if negative.returncode == 0 or "Recovered value call differs: " + name not in log:
                    raise ValueError("Generic value-call mutant survived: " + label + "\n" + log)
                print(config + ": rejected " + label + " mutant", flush=True)
                results.append({"configuration": config, "mutation": label, "detected": True})
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
            plain_ctor = next(name for name, body in bodies.items() if name.startswith("PlainCtor_") and re.search(r", object tag\)", body.split("\n{", 1)[0]))
            for label in ["PlainValue", "PlainTag", "PlainDestination", "PlainArgument", "PlainArgumentOrder", "PlainRefCopies"] + (["PlainZeroing", "PlainLateValue"] if args.reinitialize else []):
                changed = dict(bodies)
                if label == "PlainValue":
                    changed[plain_ctor], count = re.subn(r"__PlainValueStore\(ref (\w+),\s*\w+\)", r"__PlainValueStore(ref \1, 0)", changed[plain_ctor], count=1)
                elif label == "PlainTag":
                    changed[plain_ctor], count = re.subn(r"(?m)^\s*\w+\.@?Tag = \w+;\n", "", changed[plain_ctor], count=1)
                elif label == "PlainDestination":
                    changed["NewPlainInto"], count = re.subn(r"\n\{", "\n{\n target = default;", changed["NewPlainInto"], count=1)
                elif label == "PlainArgument":
                    changed["ProbeArgumentTag"], count = re.subn(r"\breturn (\w+);", r"global::ValueCallCases.Probe.Trace = 0; return \1;", changed["ProbeArgumentTag"], count=1)
                elif label == "PlainArgumentOrder":
                    changed["NewPlainOrder"], count = re.subn(r"ValueCallCases\.Recovered\.ProbeArgumentTag\([^()]+\)", "earlyTag", changed["NewPlainOrder"], count=1)
                    changed["NewPlainOrder"] = changed["NewPlainOrder"].replace("\n{", "\n{\n object earlyTag = ValueCallCases.Recovered.ProbeArgumentTag(tag);", 1)
                elif label == "PlainRefCopies":
                    changed["NewPlainRef"] = changed["NewPlainRef"].replace("ref value", "ref copiedValue").replace("ref tag", "ref copiedTag")
                    changed["NewPlainRef"], count = re.subn(r"\n\{", "\n{\n int copiedValue = value; object copiedTag = tag;", changed["NewPlainRef"], count=1)
                elif label == "PlainZeroing":
                    changed[plain_ctor], count = re.subn(r"\n\{", "\n{\n receiver.Tag = null; receiver.Count = 0;", changed[plain_ctor], count=1)
                else:
                    stored = re.search(r"(?m)^\s*__PlainValueStore\([^;]+;\n", changed[plain_ctor])
                    if stored is None: raise ValueError("Readonly constructor store missing")
                    text = changed[plain_ctor][:stored.start()] + changed[plain_ctor][stored.end():]
                    after = re.search(r"(?m)^.*\.@?Count = \w+;\n", text)
                    if after is None: raise ValueError("Constructor division destination missing")
                    changed[plain_ctor] = text[:after.end()] + stored[0] + text[after.end():]
                    count = 1
                if count != 1: raise ValueError("Ordinary constructor mutant missing: " + label)
                mutant = compile_carrier("Wrong-" + label, changed)
                negative = subprocess.run([str(x) for x in [*command[:4], mutant, library]], text=True, capture_output=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Wrong-{label}-{config}.log").write_text(log)
                if negative.returncode == 0 or "Recovered value call differs:" not in log:
                    raise ValueError("Ordinary constructor mutant survived: " + label + log)
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
