#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute recovered static constructor bodies; one run at type initialization exactly."""
import argparse
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/static-constructor-roundtrip"
SOURCES = ([ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"]
           + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs")))

# Types whose initializer still needs a capability this slice does not have. The
# set is frozen so a silently gated type cannot shrink the executed surface.
UNSUPPORTED = {
    "Caches": "unsupported opcode ldftn at IL_0001",
    "ExternalCache": "external allocation needs dependency facts",
}

# The recovered text may only call shells whose token names this fixture's own
# string literals and its one admitted exception constructor.
SHELLS = re.compile(r"__cil_(ldstr|newobj)_([0-9A-F]{8})\(")


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


support = module("object_roundtrip", ROOT / "tests/object-roundtrip.py")


def without_report(text):
    """Dependency provenance is evidence, not C#."""
    kept = [line for line in text.splitlines()
            if not line.startswith(".dependency") and not line.startswith("// Dependency")]
    return "\n".join(kept) + "\n"


def shells(views):
    """Declare the placeholder shells the recovered text calls, once each."""
    declared = []
    for kind, token in sorted({found for view in views.values() for found in SHELLS.findall(view)}):
        if kind == "ldstr":
            declared.append(f"public static string __cil_ldstr_{token}(string units) => units;")
        else:
            declared.append(
                f"public static global::System.ArgumentOutOfRangeException __cil_newobj_{token}"
                f"(string parameter, string message) => new global::System.ArgumentOutOfRangeException(parameter, message);")
    return declared


def carrier(views, field_names):
    """Compile each recovered type view as its own type, bodies exactly as written."""
    pieces = []
    for name, view in views.items():
        text = without_report(view).replace("namespace StaticConstructorCases;\n", "")
        short = name.split("`", 1)[0]
        text, count = re.subn(rf"\b(class|struct) {re.escape(short)}\b", rf"\1 Recovered{short}", text, count=1)
        if count != 1:
            raise ValueError("recovered type declaration missing: " + name)
        # A `static readonly` field is assignable only by its declaring type, so the
        # qualified store moves to the recovered type's own field of the same name;
        # both sides are then read back and compared field by field.
        text = text.replace(f"StaticConstructorCases.{name}.", f"Recovered{short}.")
        def static_store(match):
            indent, token, value = match.groups()
            owner, field = field_names[token]
            if owner != name or field not in ["Count", "Stamp"] or short != "Registry":
                raise ValueError("Unexpected owned generic initializer storage")
            return f"{indent}RecoveredRegistry<T>.{field} = {value};"
        text = re.sub(r"(?m)^(\s*)__cil_stsfld_([0-9A-F]{8})\((.*)\);$", static_store, text)
        def static_load(match):
            owner, field = field_names[match[1]]
            if owner != name or field not in ["Count", "Stamp"] or short != "Registry":
                raise ValueError("Unexpected owned generic initializer read")
            return f"RecoveredRegistry<T>.{field}"
        text = re.sub(r"__cil_ldsfld_([0-9A-F]{8})\(\)", static_load, text)
        # The initializer's own name is the enclosing type name.
        renamed = f"static Recovered{short}("
        if f"static {short}(" not in text:
            raise ValueError("recovered static constructor missing: " + name)
        text = text.replace(f"static {short}(", renamed)
        pieces.append(text.strip())
    return ("namespace StaticConstructorCases\n{\nusing static StaticConstructorCases.RecoveredShells;\n\n"
            + "public static class RecoveredShells\n{\n" + "\n".join(shells(views)) + "\n}\n\n"
            + "\n\n".join(pieces) + "\n}\n")


def duplicate_body(views):
    """Move Accumulator's initializer statements into a method and run it twice."""
    name = "Accumulator"
    text = views[name]
    header = "static Accumulator()"
    if header not in text:
        raise ValueError("duplicate-body mutation target missing")
    start = text.index(header)
    body_start = text.index("{", start) + 1
    body_end = text.index("\n    }\n", body_start)
    body = text[body_start:body_end]
    moved = (text[:start]
             + "static Accumulator()\n    {\n        __initializer();\n        __initializer();\n    }\n\n"
             + "    static void __initializer()\n    {" + body + "\n    }\n"
             + text[body_end + len("\n    }\n"):])
    return {**views, name: moved}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--mutation-control", action="store_true")
    args = parser.parse_args()
    if not args.cli.is_file():
        parser.error("Existing Rust CLI required")
    OUT.mkdir(parents=True, exist_ok=True)
    print(support.run([sys.executable, ROOT / "tests/build-static-constructor-fixtures.py"]).stdout,
          end="", flush=True)
    harness = OUT / "StaticConstructorRoundtrip.dll"
    compile_ms = support.compile_cs(harness, SOURCES, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/static-constructor-{config}.bin"
        owners, names = {}, {}
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            found = re.search(r"(\S+)::([^\s(]+)\(", signature)
            if found:
                owners[token] = found.group(1)
                names[token] = found.group(2)
        gated, recovered = {}, {}
        for record in map(json.loads, support.run([args.cli, "scan", fixture,
                                                   "--no-auto-references"]).stdout.splitlines()):
            owner = owners.get(record["token"])
            if owner is None or not owner.startswith("StaticConstructorCases."):
                continue
            short = owner.rsplit(".", 1)[1]
            if record["status"] == "Partial":
                gated[short] = record["reason"]
            elif names[record["token"]] == ".cctor":
                recovered[short] = record["token"]
        if {name: reason for name, reason in gated.items() if not reason.startswith(UNSUPPORTED.get(name, "\0"))} or set(gated) != set(UNSUPPORTED):
            raise ValueError("static-constructor negative surface drifted: " + repr(gated))
        if not recovered:
            raise ValueError("static-constructor selection is empty")
        views, field_names = {}, {}
        for name in sorted(recovered):
            original_il = support.run([args.cli, "il", fixture, "--method", recovered[name]]).stdout
            for token, owner, field in re.findall(r"stsfld 0x([0-9A-F]{8}) class StaticConstructorCases\.([^<]+)<[^>]+>::(\w+)", original_il):
                field_names[token] = (owner, field)
            view = support.run([args.cli, "decompile", fixture, "--type",
                                "StaticConstructorCases." + name, "--no-auto-references"])
            if "decompiled=1" not in view.stderr:
                raise ValueError("recovered type view contains a fallback: " + name)
            (OUT / f"{config}-{name}.type.txt").write_text(view.stdout)
            views[name] = view.stdout
        source = OUT / f"Recovered-{config}.cs"
        support.write_changed(source, carrier(views, field_names))
        compiled = source.with_suffix(".dll")
        compile_ms += support.compile_cs(compiled, [source], references=[fixture])
        result = support.run([support.DOTNET / "dotnet", harness, fixture,
                              "--static-ctor-check", compiled, source])
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip(),
                     "recovered": sorted(recovered), "gated": gated})
        if args.mutation_control:
            for label, mutated in (
                # One of the two accumulated stores is gone: the type ends at 5, not 12.
                ("DroppedStore", {**views, "Accumulator": views["Accumulator"].replace(
                    "StaticConstructorCases.Accumulator.Total = v5;\n", "", 1)}),
                # The literal is part of the state, not decoration.
                ("WrongLiteral", {**views, "Simple": views["Simple"].replace('"seed"', '"mutant"', 1)}),
                # Parameter order of the thrown object is observable.
                ("SwappedArguments", {**views, "Failing": views["Failing"].replace(
                    "__cil_newobj_0A000005(v1, v2)", "__cil_newobj_0A000005(v2, v1)", 1)}),
                # The same statements emitted as an ordinary method run twice.
                ("DuplicatedBody", duplicate_body(views)),
            ):
                if mutated == views:
                    raise ValueError("static-constructor mutation target missing: " + label)
                path = OUT / f"{label}-{config}.cs"
                support.write_changed(path, carrier(mutated, field_names))
                binary = path.with_suffix(".dll")
                compile_ms += support.compile_cs(binary, [path], references=[fixture])
                negative = subprocess.run([str(support.DOTNET / "dotnet"), str(harness), str(fixture),
                                           "--static-ctor-check", str(binary), str(path)],
                                          text=True, capture_output=True, timeout=300)
                (OUT / f"{label}-mutation-{config}.log").write_text(negative.stdout + negative.stderr)
                output = negative.stdout + negative.stderr
                if negative.returncode == 0 or "Recovered static constructor" not in output:
                    raise ValueError("static-constructor mutation not detected: " + label)
                rows.append({"configuration": config, "mutation": label,
                             "detected": output.strip().splitlines()[0]})
                print(config + ": rejected " + label + " mutant", flush=True)
    (OUT / "behavior-results.json").write_text(json.dumps(
        {"configurations": rows, "compile_ms": compile_ms,
         "mutation_control": args.mutation_control, "third_party_execution": False}, indent=2) + "\n")
    print(json.dumps({"compile_ms": compile_ms, "rows": rows}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
