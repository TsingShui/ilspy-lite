#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute recovered struct-receiver bodies: managed locations, not copies."""
import argparse, importlib.util, json, re, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/struct-receiver-roundtrip"

def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec); spec.loader.exec_module(result); return result

support = module("object_roundtrip", ROOT / "tests/object-roundtrip.py")
adapter = module("reference_roundtrip", ROOT / "tests/reference-roundtrip.py")
array_leaves = module("exact_array_leaves", ROOT / "tests/support/exact_array_leaves.py")

SOURCES = ([ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"]
           + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs")))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--readable", action="store_true")
    parser.add_argument("--mutation-control", action="store_true")
    args = parser.parse_args()
    if not args.cli.is_file(): parser.error("Existing Rust CLI required")
    OUT.mkdir(parents=True, exist_ok=True)
    print(support.run([sys.executable, ROOT / "tests/build-struct-receiver-fixtures.py"]).stdout, end="", flush=True)
    harness = OUT / "StructReceiverRoundtrip.dll"
    sources = [ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs", ROOT / "tests/dotnet/ScopedReferenceBehavior.cs",
               ROOT / "tests/dotnet/EnumBehavior.cs", ROOT / "tests/dotnet/StructCopyBehavior.cs",
               ROOT / "tests/dotnet/StructReceiverBehavior.cs"]
    compile_ms = support.compile_cs(harness, SOURCES, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/struct-receiver-{config}.bin"
        selected, names = [], {}
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            match = re.search(r" StructReceiverCases\.(\w+)::([A-Za-z][A-Za-z0-9_]*)\(", signature)
            if match:
                names[token] = match[2]
                if not match[2].startswith("Unsupported") or match[2] in {"UnsupportedByRef", "UnsupportedLocalEscape", "UnsupportedBox", "UnsupportedArray"}:
                    selected.append((token, match[2]))
        if not selected: raise ValueError("struct receiver selection is empty")
        snippets, identities = [], {}
        for token, name in selected:
            details = support.run([args.cli, "decompile", fixture, "--method", token, "--details", "--no-auto-references"])
            if details.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0":
                raise ValueError("struct receiver method not recovered: " + name)
            (OUT / f"{config}-{name}.details.txt").write_text(details.stdout)
            for alias, signature in re.findall(r"// structural signature (__cil_(?:value|ref)_\w+) bytes=([0-9A-F]+):", details.stdout):
                identities.setdefault(alias, signature)
            statements = details.stdout
            if args.readable:
                readable = support.run([args.cli, "decompile", fixture, "--method", token, "--no-auto-references"])
                if readable.stderr != details.stderr: raise ValueError("view changed admission")
                (OUT / f"{config}-{name}.readable.txt").write_text(readable.stdout)
                statements = adapter.adapt_readable_body(readable.stdout, details.stdout)
            statements = re.sub(r"\b(?:global::)?@?StructReceiverCases\.@?Ops\.(?=@?[A-Za-z_][A-Za-z_0-9]*\()", "Recovered.", statements)
            # Direct call sites must bind to recovered carriers, never to an
            # unavailable original body; an unmapped target is a hard failure.
            def bind(match):
                token = "0x" + match.group(1)
                if token not in names or names[token] not in {n for _, n in selected}:
                    raise ValueError("direct call target has no recovered body: " + match.group(0))
                return "Recovered." + names[token] + "("
            statements = re.sub(r"__cil_direct_([0-9A-F]{8})\(", bind, statements)
            snippets.append(statements)
        directives = OUT / f"aliases-{config}.json"
        support.write_changed(directives, json.dumps([{"alias": k, "bytes": v} for k, v in identities.items()], indent=2))
        aliases = support.run([support.DOTNET / "dotnet", harness, fixture, "--receiver-plan", directives]).stdout
        array_helpers, snippets = array_leaves.plan(support, harness, fixture, OUT,
            [(token, snippet) for (token, _), snippet in zip(selected, snippets)])
        text = aliases + array_helpers + "\nnamespace StructReceiverCases\n{\npublic static class Recovered\n{\n" + "\n".join(snippets) + "\n}\n}\n"
        suffix = config + ("-readable" if args.readable else "-details")
        source = OUT / f"Recovered-{suffix}.cs"
        support.write_changed(source, text)
        recovered = source.with_suffix(".dll")
        compile_ms += support.compile_cs(recovered, [source], references=[fixture])
        result = support.run([support.DOTNET / "dotnet", harness, fixture, "--receiver-check", recovered])
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip()})
        if args.mutation_control:
            # Each mutant loses exactly one field store, so the recovered body
            # still compiles but the caller's storage must differ.
            for method in ("SetX", "Bump", "Move"):
                target = re.search(rf"public static [^\n]+ @{method}\([^\n]+\)\n\{{(.*?)\n\}}", text, re.S)
                if target is None:
                    raise ValueError("receiver mutation target missing: " + method)
                lines = target.group(1).split("\n")
                # Drop the last plain store: a field write reaches the caller's
                # storage either directly or through an acquired location.
                for index in range(len(lines) - 1, -1, -1):
                    line = lines[index].strip()
                    if " = " not in line or not line.endswith(";") or line.startswith("ref "):
                        continue
                    lhs = line.split(" = ", 1)[0].split()[-1]
                    # Keep the mutant compilable: the dropped store's target must
                    # not be read by a later statement.
                    if any(lhs in later for later in lines[index + 1:]):
                        continue
                    lines[index] = "    ; // dropped store"
                    break
                else:
                    raise ValueError("receiver mutation store missing: " + method)
                mutated = text[:target.start(1)] + "\n".join(lines) + text[target.end(1):]
                path = OUT / f"Dropped{method}Store-{suffix}.cs"
                support.write_changed(path, mutated)
                binary = path.with_suffix(".dll")
                compile_ms += support.compile_cs(binary, [path], references=[fixture])
                negative = subprocess.run([str(support.DOTNET / "dotnet"), str(harness), str(fixture),
                                           "--receiver-check", str(binary)], text=True, capture_output=True, timeout=120)
                (OUT / f"Dropped{method}Store-mutation-{suffix}.log").write_text(negative.stdout + negative.stderr)
                if negative.returncode == 0 or "Recovered struct receiver behavior differs: " + method not in negative.stdout + negative.stderr:
                    raise ValueError("receiver mutation not detected: " + method)
                print(config + ": rejected dropped " + method + " store mutant", flush=True)
    view = "readable" if args.readable else "details"
    (OUT / f"behavior-results-{view}.json").write_text(json.dumps({"configurations": rows, "compile_ms": compile_ms,
        "view": view, "mutation_control": args.mutation_control, "third_party_execution": False}, indent=2) + "\n")
    print(f"Owned compilation: {compile_ms} ms; Rust compilation: none")

if __name__ == "__main__":
    main()
