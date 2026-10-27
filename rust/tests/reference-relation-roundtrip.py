#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute recovered reference relationships without adding runtime conversions."""
import argparse
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/reference-relations/behavior"


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


support = module("object_roundtrip", ROOT / "tests/object-roundtrip.py")
generic = module("generic_declaring", ROOT / "tests/generic-declaring-roundtrip.py")
leaves = module("exact_reference_leaves", ROOT / "tests/support/exact_reference_leaves.py")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--mutation-control", action="store_true")
    parser.add_argument("--selected-version", action="store_true", help="Use a version-substituted owned library")
    args = parser.parse_args()
    global OUT
    if args.selected_version:
        OUT = OUT / "selected-version"
    OUT.mkdir(parents=True, exist_ok=True)
    support.run([sys.executable, ROOT / "tests/build-reference-relation-fixtures.py"])
    harness = OUT / "ReferenceRelationRoundtrip.dll"
    support.compile_cs(harness, [ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"]
                       + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs")), executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    results = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/reference-relation-{config}.bin"
        library = ROOT / f"tests/fixtures/relation-library-{config}.bin"
        if args.selected_version:
            source = OUT / f"SelectedLibrary-{config}.cs"
            original = (ROOT / "tests/dotnet/ReferenceRelationCases.cs").read_text()
            support.write_changed(source, original.replace("#if LIBRARY",
                '#define LIBRARY\n[assembly: System.Reflection.AssemblyVersion("4.0.0.0")]\n#if LIBRARY', 1))
            library = OUT / f"relation-library-{config}.bin"
            support.compile_cs(library, [source])
        names = {}
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            match = re.search(r"(RelationCases\.(?:Ops|Cell\S*))::([^\s(<]+)", signature)
            # Semantic groups with their own drivers stay out of this selection:
            # value construction (external-allocation driver) and array allocation
            # and element access (general-array driver).
            excluded = {
                "SeededValue", "ForeignArray", "ForeignValueArray",
                # Unmanaged pointer bodies are executed by the unsafe pointer driver.
                "ReadPointer", "ReadWrite", "SamePointer", "NullPointer", "OffsetValue",
                "Difference",
                # Negative case: a foreign constructed field write stays refused.
                "WriteForeignValue",
                # Value-type MethodSpec receivers are dispatched through a `ref`
                # receiver slot that this driver's leaf helpers do not model; the
                # struct-receiver driver covers value receivers, and the token form
                # is checked by the Rust test.
                "CallValueGeneric", "CallLocalValueGeneric",
            }
            if match and not match[2].startswith(".") and match[2] not in excluded:
                names[token] = (match[1], match[2])
        methods = []
        for row in map(json.loads, support.run([args.cli, "scan", fixture, "--no-auto-references", "--reference", library]).stdout.splitlines()):
            if row["token"] not in names: continue
            owner, name = names[row["token"]]
            if row["status"] != "Decompiled": raise ValueError("Owned relation not recovered: " + owner + "." + name)
            text = support.run([args.cli, "decompile", fixture, "--method", row["token"], "--no-auto-references", "--reference", library]).stdout
            (OUT / f"{config}-{row['token']}.readable.txt").write_text(text)
            text = generic.inline_structural_types(text)
            parameters = ["A", "B"] if name in ["Swap", "SwappedInterface"] else ["T"]
            for position, parameter in enumerate(parameters):
                text = text.replace("!!" + str(position), parameter).replace("!" + str(position), parameter)
            text = re.sub(r"\b(?:global::)?@?RelationCases\.@?Ops\.", "RelationCases.Recovered.", text)
            if name == "ReadCast":
                text = "public class RecoveredCell<T> {\n" + text + "\n}\n"
            executable = "\n".join(line for line in text.splitlines() if not line.strip().startswith("//"))
            bound = re.sub(r"__cil_(?:callvirt|reference_call|memberref_call|methodspec_call)_[0-9A-F]{8}", "__leaf", executable)
            bound = re.sub(r"__cil_type_[0-9A-F]{8}", "__alias", bound)
            bound = re.sub(r"__cil_(?:arg|load|store|return|call_arg|call_result)_r4|__cil_F\b", "__float_leaf", bound)
            bound = re.sub(r"__cil_(?:conv_r8|conv_r4_float|div_F8)", "__double_leaf", bound)
            if "__cil_" in support.without_boolean_helpers(bound): raise ValueError("Unbound relation body: " + name)
            methods.append(text)
        if not methods: raise ValueError("Empty relation selection")
        if not any("class RecoveredCell<T>" in method for method in methods): raise ValueError("Owned self-cast selection missing")
        aliases, helpers = leaves.plan(support, harness, fixture, OUT, methods, "RelationCases.Ops", library)

        float_alias = "using __cil_F = System.Single;\n" if any("__cil_F" in body for body in methods) else ""
        float_helpers = ""
        for leaf in ["arg", "load", "store", "call_arg", "call_result", "return"]:
            if any(f"__cil_{leaf}_r4(" in body for body in methods):
                float_helpers += f"static float __cil_{leaf}_r4(float value) => value;\n"
        double_helpers = ""
        for body in methods:
            if "__cil_conv_r8(" in body:
                double_helpers += "static double __cil_conv_r8(long value) => value;\n"
            if "__cil_conv_r4_float(" in body:
                double_helpers += "static float __cil_conv_r4_float(double value) => (float)value;\n"
            if "__cil_div_F8(" in body:
                double_helpers += "static double __cil_div_F8(double a, double b) => a / b;\n"

        def compile_carrier(label, bodies):
            source = OUT / f"{label}-{config}.cs"
            support.write_changed(source, float_alias + aliases
                                  + "namespace RelationCases { public static class Recovered {\n"
                                  + float_helpers + double_helpers + helpers + "\n".join(bodies) + "\n} }\n")
            binary = source.with_suffix(".dll")
            support.compile_cs(binary, [source], references=[fixture, library])
            return binary
        binary = compile_carrier("Recovered", methods)
        command = [support.DOTNET / "dotnet", harness, fixture, "--reference-relation-check", binary, library]
        result = support.run(command)
        print(config + ": " + result.stdout.strip(), flush=True)
        results.append({"configuration": config, "behavior": result.stdout.strip()})
        if args.mutation_control:
            for name, replacement in [("DeclaredOverload", "return input == null ? 29 : 29;"),
                                       ("Swap", "return null;"), ("Foreign", "return null;"),
                                       ("ReadCast", "return default;"),
                                       ("SkipInitBranch", "return 0;"),
                                       ("SkipInitLoop", "return 0;"),
                                       ("SkipInitAddress", "return 0;"),
                                       ("ExternalGeneric", "return 0;"),
                                       ("FloatLe", "return false;"),
                                       ("DoubleSum", "return 5.0;")]:
                selected = [i for i, text in enumerate(methods) if re.search(r"\b" + name + r"(?:<[^>]+>)?\(", text)]
                if len(selected) != 1: raise ValueError("Mutation target differs: " + name)
                changed = list(methods)
                if name == "DeclaredOverload":
                    changed[selected[0]], count = re.subn(r"\(\(RelationCases\.Base<string>\)\(input\)\)", "input", changed[selected[0]], count=1)
                elif name == "ExternalGeneric":
                    changed[selected[0]], count = re.subn(
                        r"(__cil_callvirt_[0-9A-F]{8}\([^,]+, )value\)", r"\g<1>0)", changed[selected[0]], count=1)
                elif name == "SkipInitAddress":
                    changed[selected[0]], count = re.subn(r"RelationCases\.LocalInitNegative\.Increment\(ref [^)]*\);", "", changed[selected[0]], count=1)
                else:
                    changed[selected[0]], count = re.subn(r"return [^;]+;", replacement, changed[selected[0]], count=1)
                if count != 1: raise ValueError("Mutation return missing")
                mutant = compile_carrier("Wrong-" + name, changed)
                negative = subprocess.run([str(x) for x in [*command[:4], mutant, library]], text=True, capture_output=True, timeout=120)
                log = negative.stdout + negative.stderr
                (OUT / f"Wrong-{name}-{config}.log").write_text(log)
                if negative.returncode == 0 or "Recovered reference relation differs:" not in log:
                    raise ValueError("Relation semantic mutant survived: " + name + log)
                print(config + ": rejected " + name + " mutant", flush=True)
                results.append({"configuration": config, "mutation": name, "detected": True})
    (OUT / "results.json").write_text(json.dumps({"rows": results, "third_party_execution": False}, indent=2) + "\n")


if __name__ == "__main__":
    main()
