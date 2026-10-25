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


def adapt_readable_body(readable, details):
    """Execute readable statements, using only unchanged detailed header/identity evidence.

    No selected body is substituted. Parameter/receiver names are rebound to the
    existing test shell signature; field token comments identify shell state only.
    An explicit initializer becomes one same-receiver operation at body entry;
    call-token metadata binds its target, never its arguments or detailed statements.
    """
    readable = readable.rstrip() + "\n"
    detailed = re.search(r"^public static [^\n]+\n\{\n", details, re.M)
    rendered = re.search(r"^[^\n]+\n\{\n", readable)
    if detailed is None or rendered is None or not readable.endswith("}\n"):
        raise ValueError("Readable body needs a supported ordinary declaration")
    original_header = detailed.group(0)
    header = readable[:rendered.end()].splitlines()[0]
    initializer = ""
    if " : " in header:
        header, suffix = header.split(" : ", 1)
        match = re.fullmatch(r"(base|this)\(([^()]*)\)", suffix)
        caller = re.search(r"^// token 0x[0-9A-F]{8} instance void (.+)::\.ctor\(", details, re.M)
        if match is None or caller is None or "__cil_ctor_body_" not in original_header:
            raise ValueError("Unsupported readable constructor initializer")
        kind, arguments = match.groups()
        targets = {(token, owner) for token, owner in re.findall(
            r"^\s*// call token 0x([0-9A-F]{8}): (.+)::\.ctor$", details, re.M)
            if (owner == caller[1]) == (kind == "this")}
        if len(targets) != 1:
            raise ValueError("Constructor initializer target metadata is missing or ambiguous")
        token, owner = targets.pop()
        operation = "core_object_ctor" if owner.rsplit("]", 1)[-1] == "System.Object" else "ctor_call"
        initializer = f'    __cil_{operation}_{token}(this{", " + arguments if arguments else ""});\n'
    parameters = header[header.index("(") + 1:header.rindex(")")]
    names = [p.strip().split(" = ", 1)[0].split()[-1] for p in parameters.split(",") if p.strip()]
    if len(names) != len(set(names)):
        raise ValueError("Readable parameter names are ambiguous")
    bindings = re.findall(r"\ba\d+\b", original_header)
    if len(names) == len(bindings) - 1:
        mapping = {"this": bindings[0], **dict(zip(names, bindings[1:]))}
    elif len(names) == len(bindings):
        mapping = dict(zip(names, bindings))
    else:
        raise ValueError("Readable parameter shape differs from detailed signature")
    statements = initializer + readable[rendered.end():-2]
    lexical = re.compile(r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|@?[A-Za-z_][A-Za-z_0-9]*')
    statements = lexical.sub(lambda m: m.group(0) if statements[:m.start()].rstrip().endswith((".", "::"))
                             else mapping.get(m.group(0), m.group(0)), statements)
    comments = "\n".join(line for line in details.splitlines() if line.strip().startswith("// field token "))
    return comments + "\n" + original_header + statements + "}\n"


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
    corpus_choice.add_argument("--wait", action="store_true", help="execute the frozen independent Wait.Tick body with its unchanged exact base getter")
    corpus_choice.add_argument("--wait-constructors", action="store_true", help="execute both frozen Wait constructors and subsequent Tick calls on fresh state")
    corpus_choice.add_argument("--arrays", action="store_true", help="execute the owned integer-array corpus")
    corpus_choice.add_argument("--native", action="store_true", help="execute the owned native-integer storage corpus")
    corpus_choice.add_argument("--conversions", action="store_true", help="execute the owned integer-conversion corpus")
    corpus_choice.add_argument("--addresses", action="store_true", help="execute the owned managed-element-address corpus")
    corpus_choice.add_argument("--constructors", action="store_true", help="execute owned core-base constructor bodies")
    corpus_choice.add_argument("--base-constructors", action="store_true", help="execute local/external base construction with parameter side effects and base exceptions")
    corpus_choice.add_argument("--clock", action="store_true", help="execute an exact core clock read with a controlled test clock")
    corpus_choice.add_argument("--delegated", action="store_true", help="validate owned delegated constructor with a controlled clock")
    corpus_choice.add_argument("--strings", action="store_true", help="execute owned UTF-16 literals and string reference state")
    corpus_choice.add_argument("--exceptions", action="store_true", help="execute owned bounded exception-object allocation")
    corpus_choice.add_argument("--floats", action="store_true", help="execute owned nominal R4 conversion and snapshots")
    corpus_choice.add_argument("--external", action="store_true", help="execute owned opaque external-reference identity bodies")
    parser.add_argument("--from-class", action="store_true", help="execute bodies extracted from the public --type output")
    parser.add_argument("--readable", action="store_true", help="execute actual readable bodies; detailed metadata supplies test shell identities only")
    parser.add_argument("--verify-inputs", action="store_true", help="explicitly rehash the pinned OpenRA inputs before execution")
    args = parser.parse_args()
    wait_family = args.wait or args.wait_constructors
    if args.random_class and not args.from_class:
        parser.error("--random-class requires --from-class")
    if args.verify_inputs and not (args.openra or args.activity or wait_family or args.random_state or args.random_seed or args.random_default or args.random_ranges or args.random_float or args.random_class):
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

    def compile_cs(output, sources, executable=False, references=(), runtime_unification=False):
        response = output.with_suffix(".rsp")
        response.write_text("\n".join([
            "-nologo", "-langversion:14", "-warnaserror+", "-nowarn:0164,0219", "-target:exe" if executable else "-target:library",
            f'-out:"{output}"', *[f'-r:"{p}"' for p in sorted(ref.glob("*.dll"))],
            *[f'-r:"{p}"' for p in references],
            *(["-nowarn:1701"] if runtime_unification else []),
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
    if args.openra or args.activity or wait_family or args.random_state or args.random_seed or args.random_default or args.random_ranges or args.random_float or args.random_class:
        manifest = json.loads((ROOT / "acceptance/openra/manifest.json").read_text())
        corpus = ROOT / "report/openra"
        assembly = corpus / "assemblies" / ("OpenRA.Mods.Common.dll" if args.openra or wait_family else "OpenRA.Game.dll")
        if args.verify_inputs:
            expected = manifest["assemblies"][assembly.name]["sha256"]
            if hashlib.sha256(assembly.read_bytes()).hexdigest() != expected:
                raise ValueError("OpenRA assembly hash mismatch")
            for name, entry in manifest["assemblies"].items():
                path = corpus / "runtime-deps" / name
                if hashlib.sha256(path.read_bytes()).hexdigest() != entry["sha256"]:
                    raise ValueError(f"OpenRA runtime dependency mismatch: {name}")
        if wait_family:
            frozen = json.loads((ROOT / "acceptance/openra/classes.json").read_text())
            wait = next(c for c in frozen["classes"] if c["id"] == "wait")
            if len(wait["required_methods"]) != 1:
                raise ValueError("Frozen Wait selection drift")
            token = wait["required_methods"][0]
            selected = [{"token": token, "signature": f'instance bool {wait["name"]}::Tick(class [OpenRA.Game]OpenRA.Actor)'}]
            name = "wait"
            if args.wait_constructors:
                listing = dict(line.split(" ", 1) for line in run(cli, "list", assembly, capture_output=True).stdout.splitlines())
                tokens = wait["frozen_inventory"]["methods"]
                if len(tokens) != 3:
                    raise ValueError("Frozen complete Wait selection drift")
                selected = [{"token": token, "signature": listing[token]} for token in tokens]
                name = "wait-constructors"
        elif args.activity:
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
        corpus_name = "base-constructor" if args.base_constructors else "external" if args.external else "float" if args.floats else "exception" if args.exceptions else "string" if args.strings else "constructor" if args.constructors or args.clock or args.delegated else "address" if args.addresses else "conversion" if args.conversions else "native" if args.native else "array" if args.arrays else "reference"
        for config in ("release", "debug"):
            assembly = ROOT / f"tests/fixtures/{corpus_name}-{config}.bin"
            listing = run(cli, "list", assembly, capture_output=True).stdout.splitlines()
            selected = []
            for line in listing:
                token, signature = line.split(" ", 1)
                if args.base_constructors:
                    if not ("::.ctor(int32, int32)" in signature or "BaseConstructorCases.Probe::Mark(int32)" in signature):
                        continue
                elif args.external:
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
            expected_count = 4 if args.base_constructors else 6 if args.external else 21 if args.floats else 8 if args.exceptions else 11 if args.strings else 2 if args.delegated else 1 if args.clock else 2 if args.constructors else 15 if args.addresses else 16 if args.conversions else 7 if args.native else 21 if args.arrays else 14
            if len(selected) != expected_count:
                raise ValueError(f"{corpus_name} fixture selection drift: {len(selected)}")
            name = f"base-constructor-{config}" if args.base_constructors else f"external-{config}" if args.external else f"float-{config}" if args.floats else f"exception-{config}" if args.exceptions else f"string-{config}" if args.strings else f"delegated-{config}" if args.delegated else f"clock-{config}" if args.clock else f"{corpus_name}-{config}" if args.constructors or args.addresses or args.conversions or args.native or args.arrays else config
            dependencies = assembly.parent
            if args.external or args.base_constructors:
                dependencies = out / name / "dependencies"
                dependencies.mkdir(parents=True, exist_ok=True)
                library = "BaseConstructorLibrary.dll" if args.base_constructors else "ExternalLibrary.dll"
                prefix = "base-constructor" if args.base_constructors else "external"
                (dependencies / library).write_bytes((ROOT / f"tests/fixtures/{prefix}-library-{config}.bin").read_bytes())
            jobs.append((name, assembly, dependencies, selected))

    for name, assembly, dependencies, selected in jobs:
        directory = ROOT / "report/openra" / name if wait_family or args.activity or args.random_state or args.random_seed or args.random_default or args.random_ranges or args.random_float or args.random_class else out / name
        directory.mkdir(parents=True, exist_ok=True)
        entries = []
        class_bodies = {}
        readable_classes = {}
        if args.from_class:
            owners = sorted({s.split("::", 1)[0].split()[-1] for _, s in selected})
            for owner in owners:
                result = subprocess.run([str(cli), "decompile", str(assembly), "--type", owner, "--details"],
                                        capture_output=True, text=True, timeout=120)
                if result.returncode not in (0, 2) or (args.random_class and result.returncode != 0):
                    raise ValueError(f"Class selection failed: {owner}: {result.stderr}")
                (directory / f"{owner}.class.txt").write_text(result.stdout)
                if args.readable:
                    readable = subprocess.run([str(cli), "decompile", str(assembly), "--type", owner],
                                              capture_output=True, text=True, timeout=120)
                    if readable.returncode != result.returncode or readable.stderr != result.stderr:
                        raise ValueError("Readable presentation changed method admission/summary")
                    readable_classes[owner] = "\n".join(line.strip() for line in readable.stdout.splitlines())
                    (directory / f"{owner}.readable.txt").write_text(readable.stdout)
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
                result = run(cli, "decompile", assembly, "--method", token, "--details", capture_output=True)
                if "decompiled=1 declarations=0 partial=0 failed=0" != result.stderr.strip():
                    raise ValueError(f"Not a decompiled method: {signature}")
                body = result.stdout
            if args.readable:
                readable = run(cli, "decompile", assembly, "--method", token, capture_output=True).stdout
                (directory / f"{token}.readable.txt").write_text(readable)
                normalized = "\n".join(line.strip() for line in readable.rstrip().splitlines())
                if args.from_class and not any(normalized in text for text in readable_classes.values()):
                    raise ValueError("Selected method view differs from actual readable class body")
                body = adapt_readable_body(readable, body)
            source = directory / f"{token}.cs.txt"
            source.write_text(body)
            entries.append({"token": int(token, 16), "source": str(source)})
        job = directory / "job.json"
        kind = "base-constructors" if args.base_constructors else "wait-constructors" if args.wait_constructors else "wait" if args.wait else "external" if args.external else "activity" if args.activity else "random-class" if args.random_class else "random-float" if args.random_float else "floats" if args.floats else "random-ranges" if args.random_ranges else "exceptions" if args.exceptions else "strings" if args.strings else "default" if args.delegated or args.random_default else "clock" if args.clock else "constructors" if args.constructors else "random-seed" if args.random_seed else "random-state" if args.random_state else "openra" if args.openra else "addresses" if args.addresses else "conversions" if args.conversions else "native" if args.native else "arrays" if args.arrays else "references"
        job.write_text(json.dumps({"kind": kind, "assembly": str(assembly),
                                   "dependencies": str(dependencies), "methods": entries}, indent=2))
        source = directory / "Recovered.cs"
        run(dotnet / "dotnet", harness, "prepare", job, source)
        recovered = directory / "Recovered.dll"
        sources = [source]
        if "__cil_F" in source.read_text():
            sources.append(ROOT / "tests/dotnet/FloatSemantics.cs")
        references = json.loads(source.with_suffix(".references.json").read_text())
        # The pinned Wait leaf targets .NET 6; this optional gate intentionally
        # uses the installed .NET 10 runtime's framework-reference unification.
        compile_cs(recovered, sources, references=references, runtime_unification=wait_family)
        run(dotnet / "dotnet", harness, "run", job, recovered)


if __name__ == "__main__":
    main()
