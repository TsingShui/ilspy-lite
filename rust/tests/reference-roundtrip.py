#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute only owned reference/array fixtures or the five OpenRA stage-1 bodies.

Generated state shells and direct-call adapters are test-only. Original bodies
are called via Reflection.Emit IL call, independently of the recovered C# text.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def run(*args, **kwargs):
    return subprocess.run(list(map(str, args)), check=True, text=True, timeout=120, **kwargs)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    corpus_choice = parser.add_mutually_exclusive_group()
    corpus_choice.add_argument("--openra", action="store_true", help="execute pinned stage-1 bodies; never launch the game")
    corpus_choice.add_argument("--arrays", action="store_true", help="execute the owned integer-array corpus")
    corpus_choice.add_argument("--native", action="store_true", help="execute the owned native-integer storage corpus")
    corpus_choice.add_argument("--conversions", action="store_true", help="execute the owned integer-conversion corpus")
    parser.add_argument("--from-class", action="store_true", help="execute bodies extracted from the public --type output")
    parser.add_argument("--verify-inputs", action="store_true", help="explicitly rehash the pinned OpenRA inputs before execution")
    args = parser.parse_args()
    if args.verify_inputs and not args.openra:
        parser.error("--verify-inputs requires --openra")
    dotnet = Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10"))
    def latest(path):
        return max((p for p in path.iterdir() if p.name.startswith("10.")),
                   key=lambda p: tuple(map(int, p.name.split("."))))
    sdk = latest(dotnet / "sdk")
    ref = latest(dotnet / "packs/Microsoft.NETCore.App.Ref") / "ref/net10.0"
    runtime = latest(dotnet / "shared/Microsoft.NETCore.App")
    out = ROOT / "report/reference-roundtrip"
    out.mkdir(parents=True, exist_ok=True)

    def compile_cs(output, sources, executable=False):
        response = output.with_suffix(".rsp")
        response.write_text("\n".join([
            "-nologo", "-langversion:14", "-warnaserror+", "-nowarn:0164,0219", "-target:exe" if executable else "-target:library",
            f'-out:"{output}"', *[f'-r:"{p}"' for p in sorted(ref.glob("*.dll"))],
            *[f'"{p}"' for p in sources],
        ]))
        run(dotnet / "dotnet", sdk / "Roslyn/bincore/csc.dll", f"@{response}")

    harness = out / "ReferenceRoundtrip.dll"
    compile_cs(harness, [ROOT / "tests/dotnet/ReferenceRoundtrip.cs"], True)
    harness.with_suffix(".runtimeconfig.json").write_text(json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    run("cargo", "build", "--release", "--locked", cwd=ROOT)
    cli = ROOT / "target/release/ilspy-lite"
    jobs = []
    if args.openra:
        manifest = json.loads((ROOT / "acceptance/openra/manifest.json").read_text())
        corpus = ROOT / "report/openra"
        assembly = corpus / "assemblies/OpenRA.Mods.Common.dll"
        if args.verify_inputs:
            expected = manifest["assemblies"][assembly.name]["sha256"]
            if hashlib.sha256(assembly.read_bytes()).hexdigest() != expected:
                raise ValueError("OpenRA assembly hash mismatch")
            for name, entry in manifest["assemblies"].items():
                path = corpus / "runtime-deps" / name
                if hashlib.sha256(path.read_bytes()).hexdigest() != entry["sha256"]:
                    raise ValueError(f"OpenRA runtime dependency mismatch: {name}")
        selected = [m for m in manifest["methods"] + manifest["support_methods"] if m["stage"] == 1]
        jobs.append(("openra", assembly, corpus / "runtime-deps", [(m["token"], m["signature"]) for m in selected]))
    else:
        corpus_name = "conversion" if args.conversions else "native" if args.native else "array" if args.arrays else "reference"
        for config in ("release", "debug"):
            assembly = ROOT / f"tests/fixtures/{corpus_name}-{config}.bin"
            listing = run(cli, "list", assembly, capture_output=True).stdout.splitlines()
            selected = []
            for line in listing:
                token, signature = line.split(" ", 1)
                if "::.ctor(" in signature or "ReferenceCases.ValueCell::" in signature or "::Unsupported" in signature:
                    continue
                selected.append((token, signature))
            expected_count = 11 if args.conversions else 7 if args.native else 11 if args.arrays else 14
            if len(selected) != expected_count:
                raise ValueError(f"{corpus_name} fixture selection drift: {len(selected)}")
            name = f"{corpus_name}-{config}" if args.conversions or args.native or args.arrays else config
            jobs.append((name, assembly, assembly.parent, selected))

    for name, assembly, dependencies, selected in jobs:
        directory = out / name
        directory.mkdir(exist_ok=True)
        entries = []
        class_bodies = {}
        if args.from_class:
            owners = sorted({s.split("::", 1)[0].split()[-1] for _, s in selected})
            for owner in owners:
                result = subprocess.run([str(cli), "decompile", str(assembly), "--type", owner],
                                        capture_output=True, text=True, timeout=120)
                if result.returncode not in (0, 2):
                    raise ValueError(f"Class selection failed: {owner}: {result.stderr}")
                (directory / f"{owner}.class.txt").write_text(result.stdout)
                for match in re.finditer(
                        r"^\.method-status (0x[0-9A-F]{8}) (\w+)\n(.*?)(?=^\.method 0x|^// type token 0x|\Z)",
                        result.stdout, re.M | re.S):
                    token, status, body = match.groups()
                    if token in class_bodies:
                        raise ValueError(f"Duplicate method in class output: {token}")
                    class_bodies[token] = (status, body.rstrip() + "\n")
        for token, signature in selected:
            if args.from_class:
                status, body = class_bodies[token]
                if status != "Decompiled":
                    raise ValueError(f"Not a decompiled class method: {signature}: {status}")
            else:
                result = run(cli, "decompile", assembly, "--method", token, capture_output=True)
                if "decompiled=1 declarations=0 partial=0 failed=0" != result.stderr.strip():
                    raise ValueError(f"Not a decompiled method: {signature}")
                body = result.stdout
            source = directory / f"{token}.cs.txt"
            source.write_text(body)
            entries.append({"token": int(token, 16), "source": str(source)})
        job = directory / "job.json"
        kind = "openra" if args.openra else "conversions" if args.conversions else "native" if args.native else "arrays" if args.arrays else "references"
        job.write_text(json.dumps({"kind": kind, "assembly": str(assembly),
                                   "dependencies": str(dependencies), "methods": entries}, indent=2))
        source = directory / "Recovered.cs"
        run(dotnet / "dotnet", harness, "prepare", job, source)
        recovered = directory / "Recovered.dll"
        compile_cs(recovered, [source])
        run(dotnet / "dotnet", harness, "run", job, recovered)


if __name__ == "__main__":
    main()
