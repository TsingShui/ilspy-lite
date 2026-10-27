#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Compare actual recovered object bodies with owned .NET fixture behavior.

Uses an existing Rust CLI. Owned fixtures and test carriers compile only when
changed. No third-party corpus assembly is loaded or executed.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/object-roundtrip"
DOTNET = Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10"))


def latest(path):
    return max((p for p in path.iterdir() if p.name.startswith("10.")),
               key=lambda p: tuple(map(int, p.name.split("."))))


def run(command, expected=0):
    result = subprocess.run([str(v) for v in command], capture_output=True, text=True, timeout=120)
    if result.returncode != expected:
        raise ValueError(f"Command exited {result.returncode}: {command}\n{result.stdout}\n{result.stderr}")
    return result


def without_boolean_helpers(text):
    """Remove only known primitive names from an unbound-symbol diagnostic copy."""
    return re.sub(r"\b__cil_bool_(?:load|store)\b", "", text)


def compile_cs(output, sources, executable=False, references=(), unsafe=False):
    sources = list(sources)
    if any("__cil_bool_load" in p.read_text() or "__cil_bool_store" in p.read_text() for p in sources):
        helper = ROOT / "tests/dotnet/BooleanStorage.cs"
        if helper not in sources: sources.append(helper)
    sdk = latest(DOTNET / "sdk")
    refs = latest(DOTNET / "packs/Microsoft.NETCore.App.Ref") / "ref/net10.0"
    stamp = {"sources": {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in sources},
             "sdk": str(sdk), "refs": str(refs), "references": {str(p): p.stat().st_mtime_ns for p in references}}
    previous = output.with_suffix(".build.json")
    if output.is_file() and previous.is_file() and json.loads(previous.read_text()) == stamp:
        return 0
    response = output.with_suffix(".rsp")
    response.write_text("\n".join([
        "-nologo", "-target:exe" if executable else "-target:library", "-langversion:14",
        "-warnaserror+", "-nowarn:0164,0219", "-deterministic+", *(["-unsafe+"] if unsafe else []), f'-out:"{output}"',
        *[f'-r:"{p}"' for p in sorted(refs.glob("*.dll"))],
        *[f'-r:"{p}"' for p in references], *[f'"{p}"' for p in sources]]) + "\n")
    started = time.monotonic()
    run([DOTNET / "dotnet", sdk / "Roslyn/bincore/csc.dll", "@" + str(response)])
    previous.write_text(json.dumps(stamp, indent=2) + "\n")
    return round((time.monotonic() - started) * 1000)


def write_changed(path, text):
    if not path.is_file() or path.read_text() != text:
        path.write_text(text)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--mutation-control", action="store_true")
    parser.add_argument("--readable", action="store_true", help="Execute actual readable statements with identity-only rebinding")
    args = parser.parse_args()
    if not args.cli.is_file():
        parser.error("existing Rust CLI missing; build or cargo test first")
    OUT.mkdir(parents=True, exist_ok=True)
    print(run([sys.executable, ROOT / "tests/build-object-fixtures.py"]).stdout, end="", flush=True)
    harness = OUT / "ObjectRoundtrip.dll"
    compile_ms = compile_cs(harness, [ROOT / "tests/dotnet/ObjectRoundtrip.cs"], executable=True)
    runtime = latest(DOTNET / "shared/Microsoft.NETCore.App")
    harness.with_suffix(".runtimeconfig.json").write_text(json.dumps({"runtimeOptions": {"tfm": "net10.0",
        "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    adapter = None
    if args.readable:
        spec = importlib.util.spec_from_file_location("reference_roundtrip", ROOT / "tests/reference-roundtrip.py")
        adapter = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(adapter)
    rows = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/object-{config}.bin"
        listing = run([args.cli, "list", fixture]).stdout.splitlines()
        selected = []
        for line in listing:
            token, signature = line.split(" ", 1)
            match = re.search(r" (ObjectCases\.(?:Cell|Ops))::([A-Za-z][A-Za-z0-9_]*)\(", signature)
            if match and not match[2].startswith("Unsupported"):
                selected.append((token, match[2]))
        if len(selected) != 19 or len({name for _, name in selected}) != 19:
            raise ValueError("Owned object fixture method selection drift")
        snippets = []
        for token, name in selected:
            details = run([args.cli, "decompile", fixture, "--method", token,
                           "--details", "--no-auto-references"])
            if details.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0":
                raise ValueError("Method was not recovered: " + name)
            (OUT / f"{config}-{name}.details.txt").write_text(details.stdout)
            # Every selected helper is rebound to its actual recovered body.
            # The carrier borrows only the owned fixture's type/field definitions.
            statements = details.stdout
            if adapter is not None:
                readable = run([args.cli, "decompile", fixture, "--method", token, "--no-auto-references"])
                if readable.stderr != details.stderr:
                    raise ValueError("Readable view changed method status")
                statements = adapter.adapt_readable_body(readable.stdout, details.stdout)
                (OUT / f"{config}-{name}.readable.txt").write_text(readable.stdout)
            rebound = re.sub(r"\b(?:global::)?@?ObjectCases\.@?Ops\.", "Recovered.", statements)
            if name == "Forward" and re.search(r"Recovered\.@?Identity\(", rebound) is None:
                raise ValueError("Selected helper was not rebound to the recovered body")
            snippets.append(rebound)
        text = "public static class Recovered\n{\n" + "\n".join(snippets) + "\n}\n"
        suffix = config + ("-readable" if args.readable else "-details")
        source = OUT / f"Recovered-{suffix}.cs"
        write_changed(source, text)
        recovered = OUT / f"Recovered-{suffix}.dll"
        compile_ms += compile_cs(recovered, [source], references=[fixture])
        started = time.monotonic()
        result = run([DOTNET / "dotnet", harness, fixture, recovered])
        print(config + ": " + result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip(),
                     "elapsed_ms": round((time.monotonic() - started) * 1000)})
        if args.mutation_control:
            identity = re.search(r"public static object @Identity\(object a0\)\n\{\n.*?\n\}", text, re.S)
            if identity is None:
                raise ValueError("Recovered identity mutation shape missing")
            changed, count = re.subn(r"(?m)^(\s*)return [^;]+;", r"\1return null;", identity[0], count=1)
            if count != 1:
                raise ValueError("Recovered identity mutation return missing")
            mutant_source = OUT / f"Mutant-{suffix}.cs"
            write_changed(mutant_source, text[:identity.start()] + changed + text[identity.end():])
            mutant = OUT / f"Mutant-{suffix}.dll"
            compile_ms += compile_cs(mutant, [mutant_source], references=[fixture])
            negative = subprocess.run([str(DOTNET / "dotnet"), str(harness), str(fixture), str(mutant), "--forward-only"],
                                      capture_output=True, text=True, timeout=120)
            (OUT / f"mutation-{suffix}.log").write_text(negative.stdout + negative.stderr)
            if negative.returncode == 0 or "Recovered behavior differs: Forward" not in negative.stdout + negative.stderr:
                raise ValueError("Mutation did not detect borrowed/lost recovered helper behavior")
            print(config + ": mutation rejected; Forward executes the recovered Identity helper", flush=True)
    view = "readable" if args.readable else "details"
    (OUT / f"behavior-results-{view}.json").write_text(json.dumps({"compile_ms": compile_ms,
        "configurations": rows, "mutation_control": args.mutation_control, "view": view,
        "third_party_execution": False}, indent=2) + "\n")
    print(f"Owned test compilation: {compile_ms} ms; Rust compilation: none")


if __name__ == "__main__":
    main()
