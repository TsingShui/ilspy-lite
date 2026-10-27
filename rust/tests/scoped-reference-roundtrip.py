#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Specialize only scoped type descriptors; execute actual owned recovered bodies."""
import argparse
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/generic-value-roundtrip"


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


support = module("object_roundtrip", ROOT / "tests/object-roundtrip.py")
adapter = module("reference_roundtrip", ROOT / "tests/reference-roundtrip.py")


SOURCES = ([ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"]
           + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs")))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--readable", action="store_true")
    parser.add_argument("--mutation-control", action="store_true")
    args = parser.parse_args()
    if not args.cli.is_file():
        parser.error("Existing Rust CLI is required")
    OUT.mkdir(parents=True, exist_ok=True)
    print(support.run([sys.executable, ROOT / "tests/build-scoped-reference-fixtures.py"]).stdout, end="", flush=True)
    harness = OUT / "ScopedReferenceRoundtrip.dll"
    compile_ms = support.compile_cs(harness, SOURCES, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/scoped-reference-{config}.bin"
        selected = []
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            name = signature.split("::", 1)[1].split("(", 1)[0].split("<", 1)[0]
            if name != ".ctor" and (not name.startswith("Unsupported") or name in ("UnsupportedBare", "UnsupportedCall", "UnsupportedBox", "UnsupportedStatic", "UnsupportedAllocation")):
                selected.append((int(token, 16), name))
        if len(selected) != 27:
            raise ValueError("Owned scoped method selection drift")
        snippets, directives = {}, []
        for token, name in selected:
            details = support.run([args.cli, "decompile", fixture, "--method", f"0x{token:08X}",
                                   "--details", "--no-auto-references"])
            if details.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0":
                raise ValueError("Owned scoped method not recovered: " + name)
            (OUT / f"{config}-{name}.details.txt").write_text(details.stdout)
            aliases = [{"alias": m[0], "bytes": m[1], "type_owner": int(m[2], 16), "method_owner": int(m[3], 16)}
                       for m in re.findall(r"// structural signature (\w+) bytes=([0-9A-F]+): [^\n]*?; type-owner=0x([0-9A-Fa-f]{8}) method-owner=0x([0-9A-Fa-f]{8});", details.stdout)]
            directives.append({"token": token, "aliases": aliases, "static_fields": [int(t, 16) for t in sorted(set(re.findall(r"__cil_ldsfld_([0-9A-F]{8})", details.stdout)))], "allocations": [int(t, 16) for t in sorted(set(re.findall(r"__cil_newarr_([0-9A-F]{8})", details.stdout)))]})
            statements = details.stdout
            if args.readable:
                readable = support.run([args.cli, "decompile", fixture, "--method", f"0x{token:08X}", "--no-auto-references"])
                if readable.stderr != details.stderr:
                    raise ValueError("Scoped presentation changed status")
                (OUT / f"{config}-{name}.readable.txt").write_text(readable.stdout)
                statements = adapter.adapt_readable_body(readable.stdout, details.stdout)
            snippets[token] = statements
        path = OUT / f"directives-{config}.json"
        support.write_changed(path, json.dumps(directives, indent=2) + "\n")
        plan = json.loads(support.run([support.DOTNET / "dotnet", harness, fixture, "--scoped-plan", path]).stdout)
        if len(plan) != 270:
            raise ValueError("Scoped closure coverage drift")
        (OUT / f"plan-{config}.json").write_text(json.dumps(plan, indent=2) + "\n")
        for case in plan:
            if case["name"] != "UnsupportedCall": continue
            identities = [peer for peer in plan if peer["name"] == "Identity"
                          and [alias.split(" = ", 1)[1] for alias in peer["aliases"]]
                          == [alias.split(" = ", 1)[1] for alias in case["aliases"]]]
            if len(identities) != 1: raise ValueError("Recovered Identity closure missing or ambiguous")
            case["body"] = re.sub(r"__cil_methodspec_call_[0-9A-F]{8}\(([^()]*)\)",
                                  "global::" + identities[0]["carrier"] + r".Recovered.Identity(\1)",
                                  snippets[case["token"]])
            if "__cil_methodspec_call_" in case["body"]: raise ValueError("Unbound scoped generic call")
        pieces = ["namespace " + case["carrier"] + "\n{\n" + "\n".join(case["aliases"])
                  + "\npublic static class Recovered\n{\n" + "\n".join(case["helpers"]) + "\n" + case.get("body", snippets[case["token"]]) + "\n}\n}\n"
                  for case in plan]
        text = "\n".join(pieces)
        suffix = config + ("-readable" if args.readable else "-details")
        source = OUT / f"Recovered-{suffix}.cs"
        support.write_changed(source, text)
        recovered = source.with_suffix(".dll")
        compile_ms += support.compile_cs(recovered, [source], references=[fixture])
        result = support.run([support.DOTNET / "dotnet", harness, fixture, "--scoped-check", recovered])
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip(), "closed_contexts": len(plan)})
        if args.mutation_control:
            identity = re.search(r"public static __cil_ref_[0-9a-f]+ @Identity\([^\n]+\)\n\{\n.*?\n\}", text, re.S)
            if identity is None:
                raise ValueError("Scoped identity mutation target missing")
            changed, count = re.subn(r"(?m)^(\s*)return [^;]+;", r"\1return null;", identity[0], count=1)
            if count != 1:
                raise ValueError("Scoped identity return missing")
            mutated = text[:identity.start()] + changed + text[identity.end():]
            wrong_binding = text.replace("Pair<global::System.Int32, global::System.String>",
                                        "Pair<global::System.String, global::System.Int32>", 1)
            if wrong_binding == text:
                raise ValueError("Distinct VAR/MVAR mutation binding missing")
            static_index = next(i for i, case in enumerate(plan) if case["name"] == "UnsupportedStatic")
            copied, count = re.subn(r"(private static (.+) __cil_ldsfld_[0-9A-F]{8}\(\) => )([^;\n]+);",
                r"\1\3 == null ? null : new \2(\3);", pieces[static_index], count=1)
            if count != 1: raise ValueError("Static identity mutation target missing")
            static_mutant = "\n".join(copied if i == static_index else piece for i, piece in enumerate(pieces))
            mutants = [
                ("StaticIdentity", static_mutant, "Recovered scoped behavior differs: UnsupportedStatic"),
                ("Body", mutated, "Recovered scoped behavior differs: Identity"),
                ("Binding", wrong_binding, "Scoped runtime parameter/return identity differs: Mixed"),
            ]
            for label, method, type_name, statement in [
                ("Snapshot", "SnapshotValue", "global::System.Int32", "a0 = a1;"),
                ("StructReference", "CopyValue", "global::ScopedReferenceCases.Payload",
                 "if (a0.Text != null) a0.Text = new string(a0.Text.ToCharArray());"),
                ("FloatBits", "CopyValue", "global::System.Double", "a0 = -a0;"),
                ("SingleBits", "CopyValue", "global::System.Single", "a0 = -a0;"),
                ("DecimalBits", "CopyValue", "global::System.Decimal", "if (a0 == 1m) a0 = 1m;"),
            ]:
                index = next((i for i, case in enumerate(plan) if case["name"] == method
                              and any(alias.endswith(" = " + type_name + ";") for alias in case["aliases"])), None)
                if index is None:
                    raise ValueError("Generic value mutation closure missing: " + label)
                pattern = rf"(public static __cil_generic_[0-9a-f]+ @{method}\([^\n]+\)\n\{{\n)"
                piece, count = re.subn(pattern, lambda m: m[1] + "    " + statement + "\n", pieces[index], count=1)
                if count != 1:
                    raise ValueError("Generic value mutation body missing: " + label)
                altered = "\n".join(pieces[:index] + [piece] + pieces[index + 1:])
                mutants.append((label, altered, "Recovered scoped behavior differs: " + method))
            for name, mutant, expected in mutants:
                path = OUT / f"{name}Mutant-{suffix}.cs"
                support.write_changed(path, mutant)
                binary = path.with_suffix(".dll")
                compile_ms += support.compile_cs(binary, [path], references=[fixture])
                negative = subprocess.run([str(support.DOTNET / "dotnet"), str(harness), str(fixture),
                                           "--scoped-check", str(binary)], capture_output=True, text=True, timeout=120)
                (OUT / f"{name}-mutation-{suffix}.log").write_text(negative.stdout + negative.stderr)
                if negative.returncode == 0 or expected not in negative.stdout + negative.stderr:
                    raise ValueError("Scoped mutation was not detected: " + name)
                print(config + ": rejected scoped " + name.lower() + " mutant", flush=True)
    view = "readable" if args.readable else "details"
    (OUT / f"behavior-results-{view}.json").write_text(json.dumps({"configurations": rows, "compile_ms": compile_ms,
        "view": view, "mutation_control": args.mutation_control, "third_party_execution": False}, indent=2) + "\n")
    print(f"Owned compilation: {compile_ms} ms; Rust compilation: none")


if __name__ == "__main__":
    main()
