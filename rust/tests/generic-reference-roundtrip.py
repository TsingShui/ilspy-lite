#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute actual recovered closed-generic reference copies on owned fixtures only."""
import argparse
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


support = module("object_roundtrip", ROOT / "tests/object-roundtrip.py")
adapter = module("reference_roundtrip", ROOT / "tests/reference-roundtrip.py")
leaves = module("exact_reference_leaves", ROOT / "tests/support/exact_reference_leaves.py")


SOURCES = ([ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"]
           + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs")))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--readable", action="store_true")
    parser.add_argument("--mutation-control", action="store_true")
    parser.add_argument("--arrays", action="store_true", help="Run the owned closed-vector profile")
    args = parser.parse_args()
    prefix = "array" if args.arrays else "generic"
    family = "ArrayReferenceCases" if args.arrays else "GenericReferenceCases"
    expected = 21 if args.arrays else 16
    out = ROOT / f"report/{prefix}-reference-roundtrip"
    if not args.cli.is_file():
        parser.error("Existing Rust CLI is required")
    out.mkdir(parents=True, exist_ok=True)
    print(support.run([sys.executable, ROOT / f"tests/build-{prefix}-reference-fixtures.py"]).stdout, end="", flush=True)
    harness = out / "GenericReferenceRoundtrip.dll"
    compile_ms = support.compile_cs(harness, SOURCES, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    results = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/{prefix}-reference-{config}.bin"
        selected = []
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            match = re.search(rf" ({family}\.(?:Cell|Ops))::([A-Za-z][A-Za-z0-9_]*)\(", signature)
            if match and (not match[2].startswith("Unsupported") or (not args.arrays and match[2] == "UnsupportedDispatch")):
                selected.append((token, match[2]))
        if len(selected) != expected or len({name for _, name in selected}) != expected:
            raise ValueError("Owned generic method selection drift")
        snippets, aliases = [], {}
        for token, name in selected:
            details = support.run([args.cli, "decompile", fixture, "--method", token, "--details", "--no-auto-references"])
            if details.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0":
                raise ValueError("Method not recovered: " + name)
            (out / f"{config}-{name}.details.txt").write_text(details.stdout)
            for alias, signature in re.findall(r"// structural signature (\w+) bytes=([0-9A-F]+):", details.stdout):
                if alias in aliases and aliases[alias] != signature:
                    raise ValueError("Ambiguous structural alias")
                aliases[alias] = signature
            statements = details.stdout
            if args.readable:
                readable = support.run([args.cli, "decompile", fixture, "--method", token, "--no-auto-references"])
                if readable.stderr != details.stderr:
                    raise ValueError("Presentation changed method status")
                (out / f"{config}-{name}.readable.txt").write_text(readable.stdout)
                statements = adapter.adapt_readable_body(readable.stdout, details.stdout)
            rebound = re.sub(rf"\b(?:global::)?@?{family}\.@?Ops\.", "Recovered.", statements)
            if name == "Forward" and re.search(r"Recovered\.@?LocalIdentity\(", rebound) is None:
                raise ValueError("Forward did not bind to recovered LocalIdentity")
            snippets.append(rebound)
        directives = out / f"aliases-{config}.json"
        support.write_changed(directives, json.dumps([{"alias": alias, "bytes": value} for alias, value in sorted(aliases.items())], indent=2))
        # Resolve only signature tokens in the owned module. No selected body is
        # synthesized or invoked while supplying these closed type aliases.
        alias_source = support.run([support.DOTNET / "dotnet", harness, fixture, "--aliases", directives]).stdout
        exact_aliases, helpers = leaves.plan(support, harness, fixture, out, snippets, family + ".Ops")
        text = alias_source + exact_aliases + "\npublic static class Recovered\n{\n" + helpers + "\n".join(snippets) + "\n}\n"
        suffix = config + ("-readable" if args.readable else "-details")
        source = out / f"Recovered-{suffix}.cs"
        support.write_changed(source, text)
        recovered = out / f"Recovered-{suffix}.dll"
        compile_ms += support.compile_cs(recovered, [source], references=[fixture])
        started = time.monotonic()
        result = support.run([support.DOTNET / "dotnet", harness, fixture, recovered])
        print(config + ": " + result.stdout.strip(), flush=True)
        results.append({"configuration": config, "behavior": result.stdout.strip(),
                        "elapsed_ms": round((time.monotonic() - started) * 1000), "aliases": len(aliases)})
        if args.mutation_control:
            identity = re.search(r"public static __cil_ref_[0-9a-f]+ @LocalIdentity\([^\n]+\)\n\{\n.*?\n\}", text, re.S)
            if identity is None:
                raise ValueError("Recovered identity mutation shape missing")
            changed, count = re.subn(r"(?m)^(\s*)return [^;]+;", r"\1return null;", identity[0], count=1)
            if count != 1:
                raise ValueError("Recovered identity return missing")
            mutant_source = out / f"Mutant-{suffix}.cs"
            support.write_changed(mutant_source, text[:identity.start()] + changed + text[identity.end():])
            mutant = out / f"Mutant-{suffix}.dll"
            compile_ms += support.compile_cs(mutant, [mutant_source], references=[fixture])
            negative = subprocess.run([str(support.DOTNET / "dotnet"), str(harness), str(fixture), str(mutant), "--forward-only"],
                                      text=True, capture_output=True, timeout=120)
            (out / f"mutation-{suffix}.log").write_text(negative.stdout + negative.stderr)
            if negative.returncode == 0 or "Recovered behavior differs: Forward" not in negative.stdout + negative.stderr:
                raise ValueError("Mutation failed to detect borrowed helper")
            print(config + ": mutation rejected; Forward calls recovered LocalIdentity", flush=True)
            extra = ("if (a0 != null && a0.Length != 0) { a0[a0.Length - 1] = null; }" if args.arrays
                     else "if (a0 != null) { a0.Reverse(); a0.Reverse(); }")
            collection, count = re.subn(
                r"(public static __cil_ref_[0-9a-f]+ @ExternalIdentity\([^\n]+\)\n\{\n)",
                lambda match: match[1] + "    " + extra + "\n", text, count=1)
            if count != 1:
                raise ValueError("Collection mutation target missing")
            collection_source = out / f"CollectionMutant-{suffix}.cs"
            support.write_changed(collection_source, collection)
            collection_library = out / f"CollectionMutant-{suffix}.dll"
            compile_ms += support.compile_cs(collection_library, [collection_source], references=[fixture])
            negative = subprocess.run([str(support.DOTNET / "dotnet"), str(harness), str(fixture), str(collection_library)],
                                      text=True, capture_output=True, timeout=120)
            (out / f"collection-mutation-{suffix}.log").write_text(negative.stdout + negative.stderr)
            if negative.returncode == 0 or "Recovered behavior differs: ExternalIdentity" not in negative.stdout + negative.stderr:
                raise ValueError("Collection observer failed to detect extra mutations")
            print(config + (": extra array element mutation rejected" if args.arrays else
                            ": extra collection mutations rejected despite identical final contents"), flush=True)
            if args.arrays:
                for method, statement in [
                    ("FloatIdentity", "if (a0 != null && a0.Length != 0) { a0[0] = -a0[0]; }"),
                    ("ValueArgument", "if (a0 != null && a0.Length != 0) { a0[0].Tag = new string(((string)a0[0].Tag).ToCharArray()); }"),
                ]:
                    pattern = rf"(public static __cil_ref_[0-9a-f]+ @{method}\([^\n]+\)\n\{{\n)"
                    altered, count = re.subn(pattern, lambda match: match[1] + "    " + statement + "\n", text, count=1)
                    if count != 1:
                        raise ValueError("Array storage mutation target missing")
                    path = out / f"{method}Mutant-{suffix}.cs"
                    support.write_changed(path, altered)
                    library = path.with_suffix(".dll")
                    compile_ms += support.compile_cs(library, [path], references=[fixture])
                    negative = subprocess.run([str(support.DOTNET / "dotnet"), str(harness), str(fixture), str(library)],
                                              text=True, capture_output=True, timeout=120)
                    (out / f"{method}-mutation-{suffix}.log").write_text(negative.stdout + negative.stderr)
                    if negative.returncode == 0 or f"Recovered behavior differs: {method}" not in negative.stdout + negative.stderr:
                        raise ValueError("Storage mutation observer failed")
                    print(config + ": storage mutation rejected: " + method, flush=True)
    view = "readable" if args.readable else "details"
    (out / f"behavior-results-{view}.json").write_text(json.dumps({"view": view, "configurations": results,
        "compile_ms": compile_ms, "mutation_control": args.mutation_control, "third_party_execution": False}, indent=2) + "\n")
    print(f"Owned compilation: {compile_ms} ms; Rust compilation: none")


if __name__ == "__main__":
    main()
