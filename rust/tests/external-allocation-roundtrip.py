#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute recovered external-value storage; widths, loads and stores must match."""
import argparse, importlib.util, json, re, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/external-allocation-roundtrip"
SOURCES = ([ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"]
           + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs")))


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec); spec.loader.exec_module(result); return result

support = module("object_roundtrip", ROOT / "tests/object-roundtrip.py")
generic = module("generic_value_roundtrip", ROOT / "tests/generic-value-roundtrip.py")


def without_dependency_report(text):
    """The diagnostic view appends dependency provenance; it is evidence, not C#."""
    kept = [line for line in text.splitlines()
            if not line.startswith(".dependency") and not line.startswith("// Dependency")]
    return "\n".join(kept) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--mutation-control", action="store_true")
    args = parser.parse_args()
    if not args.cli.is_file(): parser.error("Existing Rust CLI required")
    OUT.mkdir(parents=True, exist_ok=True)
    print(support.run([sys.executable, ROOT / "tests/build-external-allocation-fixtures.py"]).stdout, end="", flush=True)
    harness = OUT / "ExternalAllocationRoundtrip.dll"
    compile_ms = support.compile_cs(harness, SOURCES, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/external-allocation-{config}.bin"
        library = ROOT / f"tests/fixtures/external-allocation-library-{config}.bin"
        names, owners, signatures = {}, {}, {}
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            found = re.search(r"(\S+)::([^\s(]+)\(", signature)
            if found:
                owners[token] = found.group(1); names[token] = found.group(2)
                signatures[token] = signature
        scan = support.run([args.cli, "scan", fixture, "--no-auto-references",
                            "--reference", library])
        selected = [(record["token"], names[record["token"]])
                    for record in map(json.loads, scan.stdout.splitlines())
                    if record["status"] == "Decompiled" and owners.get(record["token"]) == "ExternalAllocationCases.Ops"]
        if not selected: raise ValueError("external-allocation selection is empty")
        snippets, skipped, static_tokens = [], [], set()
        for token, name in selected:
            details = support.run([args.cli, "decompile", fixture, "--method", token, "--details",
                                   "--no-auto-references", "--reference", library])
            if details.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0":
                raise ValueError("external-allocation method not recovered: " + name)
            # The claim under test is the default readable output, so that is what
            # is compiled; the diagnostic view only has to agree on admission.
            readable = support.run([args.cli, "decompile", fixture, "--method", token,
                                    "--no-auto-references", "--reference", library])
            if readable.stderr != details.stderr:
                raise ValueError("view changed admission for " + name)
            body = generic.inline_structural_types(without_dependency_report(readable.stdout))
            # Each `newobj` comment names the owner in metadata notation; the
            # intrinsic plus its type alias become `new Owner(args)` on that line.
            owners = dict(re.findall(
                r"// newobj token 0x([0-9A-F]{8}): (?:class |valuetype )?(.+?)::\.ctor",
                details.stdout,
            ))

            def allocate_named(match):
                indent, declared, variable, owner, arguments, token = match.groups()
                # Prefer the declared C# spelling; the readable view prints it from
                # the same resolution, so it is already the exact instantiation.
                name = declared if "__cil_" not in declared else re.sub(
                    r"^class |^valuetype |\[[^\]]+\]", "", owners.get(token, owner)).replace("/", ".")
                return f"{indent}{name} {variable} = new {name}({arguments});"

            def allocate(match):
                indent, declared, variable, site, arguments = match.groups()
                owner = owners.get(site)
                if owner is None:
                    raise ValueError("allocation owner is not named at " + site)
                type_name = re.sub(r"\[[^\]]+\]", "", owner).replace("/", ".")
                # The inlined declaration is already a valid C# type (including a
                # constructed instantiation); the metadata name is the fallback.
                if "__cil_" not in declared:
                    type_name = declared
                return f"{indent}{type_name} {variable} = new {type_name}({arguments});"

            # The readable view already spells the constructor and keeps the exact
            # token as a trailing comment, so the comment is dropped and the
            # constructor call itself is the executable form.
            body = re.sub(
                r"(?m)^(\s*)(.+?) (\w+) = new (.+?)\(([^()]*)\) /\* __cil_newobj_([0-9A-F]{8}) \*/;$",
                allocate_named,
                body,
            )
            body = re.sub(
                r"(?m)^(\s*)(.+?) (\w+) = __cil_newobj_([0-9A-F]{8})\(([^()]*)\);$",
                allocate,
                body,
            )
            # The declared type still carries an opaque structural alias; the
            # allocation itself is now spelled with the real constructor name.
            if any(
                marker in body
                for marker in (
                    "__cil_newobj_",
                    "__cil_memberref_call_",
                    "__cil_callvirt_",
                    "__cil_direct_",
                )
            ):
                raise ValueError("unexecutable allocation alias in " + name)
            (OUT / f"{config}-{name}.details.txt").write_text(details.stdout)
            (OUT / f"{config}-{name}.readable.txt").write_text(readable.stdout)
            code = "\n".join(line for line in body.splitlines()
                              if not line.lstrip().startswith("//"))
            static_tokens.update(int(t, 16) for t in re.findall(r"__cil_ldsfld_([0-9A-F]{8})", code))
            code = re.sub(r"__cil_ldsfld_[0-9A-F]{8}", "", code)
            if "__cil_" in support.without_boolean_helpers(code):
                skipped.append(name)
                continue
            snippets.append(body)
        # Every selected body here is compilable; an unusable alias would mean the
        # readable view regressed, so it must fail rather than shrink the selection.
        if skipped:
            raise ValueError("unexpected unexecutable selection: " + repr(skipped))
        if not snippets: raise ValueError("external-allocation carrier is empty")
        field_plan = OUT / f"static-fields-{config}.json"
        support.write_changed(field_plan, json.dumps(sorted(static_tokens)))
        static_helpers = support.run([support.DOTNET / "dotnet", harness, fixture, "--external-allocation-fields", field_plan, library]).stdout
        carrier = ("namespace ExternalAllocationCases\n{\npublic static class Recovered\n{\n"
                   + static_helpers + "\n".join(snippets) + "\n}\n}\n")
        source = OUT / f"Recovered-{config}.cs"
        support.write_changed(source, carrier)
        recovered = source.with_suffix(".dll")
        compile_ms += support.compile_cs(recovered, [source], references=[fixture, library])
        result = support.run([support.DOTNET / "dotnet", harness, fixture,
                              "--external-allocation-check", recovered, library])
        print(config + ": " + result.stdout.strip() + f" [skipped {skipped}]", flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip(),
                     "skipped": skipped})
        if args.mutation_control:
            wrong_static, count = re.subn(r"(private static int __cil_ldsfld_[0-9A-F]{8}\(\) => )[^;]+;", r"\g<1>0;", carrier, count=1)
            if count != 1: raise ValueError("Static field mutation missing")
            path = OUT / f"WrongStatic-{config}.cs"
            support.write_changed(path, wrong_static)
            binary = path.with_suffix(".dll")
            compile_ms += support.compile_cs(binary, [path], references=[fixture, library])
            mutated = subprocess.run([str(support.DOTNET / "dotnet"), str(harness), str(fixture), "--external-allocation-check", str(binary), str(library)], text=True, capture_output=True, timeout=120)
            log = mutated.stdout + mutated.stderr
            (OUT / f"WrongStatic-{config}.log").write_text(log)
            if mutated.returncode == 0 or "Recovered external allocation differs: UnsupportedStatic" not in log:
                raise ValueError("Static storage mutation survived: " + log)
            rows.append({"configuration": config, "mutation": "wrong-static-storage", "detected": True})
            print(config + ": rejected static storage mutant", flush=True)
            # Returning null where a fresh instance is constructed must change the
            # observed object, so the identity/field comparison is the gate.
            import re as _re
            pattern = r"(public static [^\n]*MakeNode\([^)]*\)\s*\{)(.*?)(\n\})"
            bad, count = _re.subn(pattern, r"\1\n        return null;\3", carrier, count=1, flags=_re.S)
            if count != 1: raise ValueError("external-allocation mutation target missing")
            path = OUT / f"NullAllocation-{config}.cs"
            support.write_changed(path, bad)
            binary = path.with_suffix(".dll")
            compile_ms += support.compile_cs(binary, [path], references=[fixture, library])
            mutated = subprocess.run([str(support.DOTNET / "dotnet"), str(harness), str(fixture),
                                      "--external-allocation-check", str(binary), str(library)],
                                     text=True, capture_output=True, timeout=300)
            (OUT / f"NullAllocation-mutation-{config}.log").write_text(mutated.stdout + mutated.stderr)
            if mutated.returncode == 0:
                raise ValueError("mutation control survived: null allocation accepted")
            rows.append({"configuration": config, "mutation": "null-allocation",
                         "detected": mutated.stderr.strip().splitlines()[0] if mutated.stderr.strip() else "nonzero exit"})
            print(config + " mutation: null allocation rejected", flush=True)
    (OUT / "behavior-results.json").write_text(json.dumps(
        {"configurations": rows, "compile_ms": compile_ms,
         "mutation_control": args.mutation_control, "third_party_execution": False}, indent=2) + "\n")
    print(json.dumps({"compile_ms": compile_ms, "rows": rows}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
