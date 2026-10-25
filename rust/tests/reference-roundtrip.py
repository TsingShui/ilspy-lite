#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute owned fixtures or explicitly selected, pinned OpenRA bodies.

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
    corpus_choice.add_argument("--random-state", action="store_true", help="execute pinned Generate/Next on controlled state; seed construction is not recovered")
    corpus_choice.add_argument("--random-seed", action="store_true", help="execute recovered pinned seed initialization and Generate/Next closure")
    corpus_choice.add_argument("--random-default", action="store_true", help="validate pinned default constructor with controlled clock and recovered seed body")
    corpus_choice.add_argument("--random-ranges", action="store_true", help="execute pinned range overloads with recovered seed and state dependencies")
    corpus_choice.add_argument("--random-float", action="store_true", help="execute pinned NextFloat with recovered seed and state dependencies")
    corpus_choice.add_argument("--random-class", action="store_true", help="execute all seven frozen random-class bodies and their combined gates")
    corpus_choice.add_argument("--activity", action="store_true", help="execute the frozen independent Activity.Queue body on finite chains")
    corpus_choice.add_argument("--arrays", action="store_true", help="execute the owned integer-array corpus")
    corpus_choice.add_argument("--native", action="store_true", help="execute the owned native-integer storage corpus")
    corpus_choice.add_argument("--conversions", action="store_true", help="execute the owned integer-conversion corpus")
    corpus_choice.add_argument("--addresses", action="store_true", help="execute the owned managed-element-address corpus")
    corpus_choice.add_argument("--constructors", action="store_true", help="execute owned core-base constructor bodies, not unsupported constructor chains")
    corpus_choice.add_argument("--clock", action="store_true", help="execute an exact core clock read with a controlled test clock")
    corpus_choice.add_argument("--delegated", action="store_true", help="validate owned delegated constructor with a controlled clock")
    corpus_choice.add_argument("--strings", action="store_true", help="execute owned UTF-16 literals and string reference state")
    corpus_choice.add_argument("--exceptions", action="store_true", help="execute owned bounded exception-object allocation")
    corpus_choice.add_argument("--floats", action="store_true", help="execute owned nominal R4 conversion and snapshots")
    corpus_choice.add_argument("--external", action="store_true", help="execute owned opaque external-reference identity bodies")
    parser.add_argument("--from-class", action="store_true", help="execute bodies extracted from the public --type output")
    parser.add_argument("--verify-inputs", action="store_true", help="explicitly rehash the pinned OpenRA inputs before execution")
    args = parser.parse_args()
    if args.random_class and not args.from_class:
        parser.error("--random-class requires --from-class")
    if args.verify_inputs and not (args.openra or args.activity or args.random_state or args.random_seed or args.random_default or args.random_ranges or args.random_float or args.random_class):
        parser.error("--verify-inputs requires an OpenRA behavior selection")
    dotnet = Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10"))
    def latest(path):
        return max((p for p in path.iterdir() if p.name.startswith("10.")),
                   key=lambda p: tuple(map(int, p.name.split("."))))
    sdk = latest(dotnet / "sdk")
    ref = latest(dotnet / "packs/Microsoft.NETCore.App.Ref") / "ref/net10.0"
    runtime = latest(dotnet / "shared/Microsoft.NETCore.App")
    out = ROOT / "report/reference-roundtrip"
    out.mkdir(parents=True, exist_ok=True)

    def compile_cs(output, sources, executable=False, references=()):
        response = output.with_suffix(".rsp")
        response.write_text("\n".join([
            "-nologo", "-langversion:14", "-warnaserror+", "-nowarn:0164,0219", "-target:exe" if executable else "-target:library",
            f'-out:"{output}"', *[f'-r:"{p}"' for p in sorted(ref.glob("*.dll"))],
            *[f'-r:"{p}"' for p in references],
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
    if args.openra or args.activity or args.random_state or args.random_seed or args.random_default or args.random_ranges or args.random_float or args.random_class:
        manifest = json.loads((ROOT / "acceptance/openra/manifest.json").read_text())
        corpus = ROOT / "report/openra"
        assembly = corpus / "assemblies" / ("OpenRA.Mods.Common.dll" if args.openra else "OpenRA.Game.dll")
        if args.verify_inputs:
            expected = manifest["assemblies"][assembly.name]["sha256"]
            if hashlib.sha256(assembly.read_bytes()).hexdigest() != expected:
                raise ValueError("OpenRA assembly hash mismatch")
            for name, entry in manifest["assemblies"].items():
                path = corpus / "runtime-deps" / name
                if hashlib.sha256(path.read_bytes()).hexdigest() != entry["sha256"]:
                    raise ValueError(f"OpenRA runtime dependency mismatch: {name}")
        if args.activity:
            frozen = json.loads((ROOT / "acceptance/openra/classes.json").read_text())
            activity = next(c for c in frozen["classes"] if c["id"] == "activity")
            if len(activity["required_methods"]) != 1:
                raise ValueError("Frozen Activity selection drift")
            token = activity["required_methods"][0]
            selected = [{"token": token, "signature": f'instance void {activity["name"]}::Queue(class {activity["name"]})'}]
            name = "activity"
        elif args.random_class:
            frozen = json.loads((ROOT / "acceptance/openra/classes.json").read_text())
            random = next(c for c in frozen["classes"] if c["id"] == "random")
            listing = dict(line.split(" ", 1) for line in run(cli, "list", assembly, capture_output=True).stdout.splitlines())
            if len(random["required_methods"]) != 7 or set(random["required_methods"]) != set(random["frozen_inventory"]["methods"]):
                raise ValueError("Complete random class selection drift")
            selected = [{"token": token, "signature": listing[token]} for token in random["required_methods"]]
            name = "random-class"
        elif args.random_state or args.random_seed or args.random_default or args.random_ranges or args.random_float:
            targets = {"random-next", "random-generate"}
            if args.random_seed or args.random_default or args.random_ranges or args.random_float:
                targets.add("random-seed")
            selected = [m for m in manifest["methods"] if m["id"] in targets]
            if {m["id"] for m in selected} != targets or len(selected) != len(targets):
                raise ValueError("Random method selection drift")
            if args.random_float:
                frozen = json.loads((ROOT / "acceptance/openra/classes.json").read_text())
                random = next(c for c in frozen["classes"] if c["id"] == "random")
                token = random["required_methods"][random["required_method_names"].index("NextFloat()")]
                selected.append({"token": token, "signature": f'instance float32 {random["name"]}::NextFloat()'})
            if args.random_ranges:
                frozen = json.loads((ROOT / "acceptance/openra/classes.json").read_text())
                random = next(c for c in frozen["classes"] if c["id"] == "random")
                for method_name, signature in [("Next(int32,int32)", "int32, int32"), ("Next(int32)", "int32")]:
                    token = random["required_methods"][random["required_method_names"].index(method_name)]
                    selected.append({"token": token, "signature": f'instance int32 {random["name"]}::Next({signature})'})
            if args.random_default:
                frozen = json.loads((ROOT / "acceptance/openra/classes.json").read_text())
                random = next(c for c in frozen["classes"] if c["id"] == "random")
                token = random["required_methods"][random["required_method_names"].index(".ctor()")]
                selected.append({"token": token, "signature": f'instance void {random["name"]}::.ctor()'})
            name = "random-float" if args.random_float else "random-ranges" if args.random_ranges else "random-default" if args.random_default else "random-seed" if args.random_seed else "random-state"
        else:
            selected = [m for m in manifest["methods"] + manifest["support_methods"] if m["stage"] == 1]
            name = "openra"
        jobs.append((name, assembly, corpus / "runtime-deps", [(m["token"], m["signature"]) for m in selected]))
    else:
        corpus_name = "external" if args.external else "float" if args.floats else "exception" if args.exceptions else "string" if args.strings else "constructor" if args.constructors or args.clock or args.delegated else "address" if args.addresses else "conversion" if args.conversions else "native" if args.native else "array" if args.arrays else "reference"
        for config in ("release", "debug"):
            assembly = ROOT / f"tests/fixtures/{corpus_name}-{config}.bin"
            listing = run(cli, "list", assembly, capture_output=True).stdout.splitlines()
            selected = []
            for line in listing:
                token, signature = line.split(" ", 1)
                if args.external:
                    if not any(f"::{name}(" in signature for name in ("Unused", "Identity", "Other", "DirectFlag", "DirectConstant", "DirectVirtual")):
                        continue
                elif args.delegated:
                    if "ConstructorCases.Root::.ctor(" not in signature:
                        continue
                elif args.clock:
                    if "ConstructorCases.Root::ClockValue()" not in signature:
                        continue
                elif args.constructors:
                    if not any(owner + "::.ctor(int32)" in signature for owner in ("ConstructorCases.Base", "ConstructorCases.Root")):
                        continue
                elif "::.ctor(" in signature or "ReferenceCases.ValueCell::" in signature or "::Unsupported" in signature or "::Anchor" in signature:
                    continue
                selected.append((token, signature))
            expected_count = 6 if args.external else 21 if args.floats else 8 if args.exceptions else 11 if args.strings else 2 if args.delegated else 1 if args.clock else 2 if args.constructors else 15 if args.addresses else 16 if args.conversions else 7 if args.native else 21 if args.arrays else 14
            if len(selected) != expected_count:
                raise ValueError(f"{corpus_name} fixture selection drift: {len(selected)}")
            name = f"external-{config}" if args.external else f"float-{config}" if args.floats else f"exception-{config}" if args.exceptions else f"string-{config}" if args.strings else f"delegated-{config}" if args.delegated else f"clock-{config}" if args.clock else f"{corpus_name}-{config}" if args.constructors or args.addresses or args.conversions or args.native or args.arrays else config
            dependencies = assembly.parent
            if args.external:
                dependencies = out / name / "dependencies"
                dependencies.mkdir(parents=True, exist_ok=True)
                (dependencies / "ExternalLibrary.dll").write_bytes((ROOT / f"tests/fixtures/external-library-{config}.bin").read_bytes())
            jobs.append((name, assembly, dependencies, selected))

    for name, assembly, dependencies, selected in jobs:
        directory = ROOT / "report/openra" / name if args.activity or args.random_state or args.random_seed or args.random_default or args.random_ranges or args.random_float or args.random_class else out / name
        directory.mkdir(parents=True, exist_ok=True)
        entries = []
        class_bodies = {}
        if args.from_class:
            owners = sorted({s.split("::", 1)[0].split()[-1] for _, s in selected})
            for owner in owners:
                result = subprocess.run([str(cli), "decompile", str(assembly), "--type", owner],
                                        capture_output=True, text=True, timeout=120)
                if result.returncode not in (0, 2) or (args.random_class and result.returncode != 0):
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
        kind = "external" if args.external else "activity" if args.activity else "random-class" if args.random_class else "random-float" if args.random_float else "floats" if args.floats else "random-ranges" if args.random_ranges else "exceptions" if args.exceptions else "strings" if args.strings else "default" if args.delegated or args.random_default else "clock" if args.clock else "constructors" if args.constructors else "random-seed" if args.random_seed else "random-state" if args.random_state else "openra" if args.openra else "addresses" if args.addresses else "conversions" if args.conversions else "native" if args.native else "arrays" if args.arrays else "references"
        job.write_text(json.dumps({"kind": kind, "assembly": str(assembly),
                                   "dependencies": str(dependencies), "methods": entries}, indent=2))
        source = directory / "Recovered.cs"
        run(dotnet / "dotnet", harness, "prepare", job, source)
        recovered = directory / "Recovered.dll"
        sources = [source]
        if "__cil_F" in source.read_text():
            sources.append(ROOT / "tests/dotnet/FloatSemantics.cs")
        references = json.loads(source.with_suffix(".references.json").read_text())
        compile_cs(recovered, sources, references=references)
        run(dotnet / "dotnet", harness, "run", job, recovered)


if __name__ == "__main__":
    main()
