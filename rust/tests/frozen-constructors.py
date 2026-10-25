#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Optional, offline gate for two unchanged original frozen-random20 constructors.

Requires existing .NET 10 SDK/reference packs and the pinned OpenRA input cache.
Generated OpenRA text and executables stay under ignored rust/report only.
"""
import argparse
import json
import os
from pathlib import Path
import re
import shlex
import subprocess

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/frozen-constructors"
COMMON_MVID = "daac77a9-0bf7-4155-8229-f659cf68d982"
SELECTED = [
    ("MustBeDestroyed", "0x0200047E", "0x060017D4", "FrozenMustReceiver"),
    ("ProximityCapturable", "0x02000514", "0x06001A46", "FrozenProximityReceiver"),
]


def run(*args, expected=0, **kwargs):
    print("+ " + shlex.join(map(str, args)), flush=True)
    result = subprocess.run(list(map(str, args)), text=True, capture_output=True,
                            timeout=120, **kwargs)
    with (OUT / "commands.log").open("a") as log:
        log.write(shlex.join(map(str, args)) + f"\nexit={result.returncode}\n")
        log.write(result.stdout + result.stderr + "\n")
    if expected is not None and result.returncode != expected:
        raise RuntimeError(f"Command exit {result.returncode}: {result.stdout}\n{result.stderr}")
    return result


def member_view(text):
    end = text.find("\n}\n")
    if end < 0:
        raise ValueError("Incomplete default member view")
    return text[:end + 3]


def adapt(readable, details):
    """Rebind the actual receiver; metadata binds initializer target, never logic."""
    header, statements = readable.split("\n{\n", 1)
    assert statements.endswith("}\n")
    statements = statements[:-2]
    initializer = ""
    if " : " in header:
        header, suffix = header.split(" : ", 1)
        match = re.fullmatch(r"base\(([^()]*)\)", suffix)
        if match is None:
            raise ValueError("Unsupported actual default initializer")
        metadata = re.split(r"^\.dependency", details, maxsplit=1, flags=re.M)[0]
        targets = set(re.findall(r"^\s*// call token 0x([0-9A-F]{8}): (.+)::\.ctor$", metadata, re.M))
        if len(targets) != 1:
            raise ValueError("Ambiguous initializer identity")
        token, owner = targets.pop()
        if owner != "[System.Runtime]System.Object" or token != "0A000109":
            raise ValueError("Only the pinned core Object initializer is supported")
        arguments = match[1]
        initializer = f"    __cil_core_object_ctor_{token}(this{', ' + arguments if arguments else ''});\n"
    if not re.fullmatch(r"public (MustBeDestroyed|ProximityCapturable)\([^()]+\)", header):
        raise ValueError("Unsupported actual default declaration")
    # No selected body comes from details: preserve the default statements verbatim
    # except for the explicit test-carrier receiver name.
    lexical = re.compile(r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|@?[A-Za-z_][A-Za-z_0-9]*')
    return lexical.sub(lambda m: "receiver" if m[0] == "this" else m[0], initializer + statements)


def latest(path):
    return max((p for p in path.iterdir() if re.fullmatch(r"10\.\d+\.\d+", p.name)),
               key=lambda p: tuple(map(int, p.name.split("."))))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", type=Path, default=ROOT / "report/openra")
    parser.add_argument("--mutate", choices=["must-info", "prox-order"],
                        help="Negative control; execution must exit nonzero")
    args = parser.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "commands.log").write_text("")
    corpus = args.corpus.resolve()
    assembly = corpus / "assemblies/OpenRA.Mods.Common.dll"
    frozen = json.loads((ROOT / "acceptance/openra/random-classes.json").read_text())
    assert frozen["count"] == 20 and len(frozen["classes"]) == 20
    for name, token, _, _ in SELECTED:
        entries = [c for c in frozen["classes"] if c["token"] == token]
        assert len(entries) == 1 and entries[0]["name"] == "OpenRA.Mods.Common.Traits." + name
        assert entries[0]["mvid"] == COMMON_MVID
    assert not any(c["name"] == "OpenRA.Mods.Common.Traits.ProximityCapturableBase"
                   for c in frozen["classes"])
    run("cargo", "build", "--release", "--locked", "--offline", cwd=ROOT)
    cli = ROOT / "target/release/ilspy-lite"
    il_evidence = {}
    for token in ["0x060017D4", "0x06001A46", "0x06001A4F"]:
        text = run(cli, "il", assembly, "--method", token).stdout
        (OUT / f"{token}-il.txt").write_text(text)
        chunks = re.findall(r"^\.code-bytes IL_[0-9A-F]+: \[([^]]+)\]$", text, re.M)
        if not chunks:
            raise ValueError("Missing original IL evidence")
        il_evidence[token[2:]] = "".join(re.findall(r"\b[0-9A-F]{2}\b", " ".join(chunks)))
    evidence = OUT / "il-evidence.json"
    evidence.write_text(json.dumps(il_evidence, indent=2))
    sources = []
    for name, _, token, shell in SELECTED:
        def extract(*selector, details=False):
            result = run(cli, "decompile", assembly, *selector,
                         *(["--details"] if details else []), expected=None)
            # Whole classes contain unrelated, explicitly unsupported members.
            if result.returncode != 0 and not (selector[0] == "--type" and result.returncode == 2):
                raise ValueError("CLI failed or selected constructor is partial")
            return result.stdout
        whole = extract("--type", "OpenRA.Mods.Common.Traits." + name)
        method = member_view(extract("--method", token))
        details = extract("--method", token, details=True)
        for suffix, text in [("type", whole), ("method", method), ("details", details)]:
            (OUT / f"{name}-{suffix}.txt").write_text(text)
        # Whitespace normalization permits class indentation, never borrowed statements.
        assert re.sub(r"\s+", "", method) in re.sub(r"\s+", "", whole)
        body = adapt(method, details)
        if args.mutate == "must-info" and name == "MustBeDestroyed":
            assert body.count("receiver.Info = info;") == 1
            body = body.replace("receiver.Info = info;", "")
        if args.mutate == "prox-order" and name == "ProximityCapturable":
            call = "__cil_ctor_call_06001A4F(receiver, init, info);"
            write = "receiver.Info = info;"
            assert body.count(call) == body.count(write) == 1
            body = body.replace(call, "").replace(write, write + "\n" + call)
        must = name == "MustBeDestroyed"
        info = "OpenRA.Mods.Common.Traits." + name + "Info"
        base = "object" if must else "OpenRA.Mods.Common.Traits.ProximityCapturableBase"
        parameters = f"{info} info" if must else f"OpenRA.ActorInitializer init, {info} info"
        stubs = "" if must else """
    protected override int CreateTrigger(OpenRA.Actor self) => throw new FrozenConstructors.CallbackReached();
    protected override void RemoveTrigger(OpenRA.Actor self, int trigger) => throw new FrozenConstructors.CallbackReached();
    protected override void TickInner(OpenRA.Actor self) => throw new FrozenConstructors.CallbackReached();
    protected override OpenRA.Graphics.IRenderable GetRenderable(OpenRA.Actor self, OpenRA.Graphics.WorldRenderer wr)
        => throw new FrozenConstructors.CallbackReached();
"""
        leaf = (f"static void __cil_core_object_ctor_0A000109({shell} receiver)\n"
                "        => FrozenConstructors.Leaf(receiver, 0x0A000109, new object[0]);" if must else
                f"static void __cil_ctor_call_06001A4F({shell} receiver, OpenRA.ActorInitializer init, {info} info)\n"
                "        => FrozenConstructors.Leaf(receiver, 0x06001A4F, new object[] { init, info });")
        source = OUT / f"{shell}.cs"
        source.write_text(f"""// Generated from the actual default member view; test-only mutable Info.
public sealed class {shell} : {base}
{{
    public {'new ' if not must else ''}{info} Info;
    public {shell}() : base({'' if must else 'default, default'})
        => throw new System.InvalidOperationException("Dummy constructor executed");
{stubs}}}
public static class {shell}Body
{{
    public static void Run({shell} receiver, {parameters})
    {{
{body}
    }}
    {leaf}
}}
""")
        sources.append(source)
    dotnet = Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10"))
    sdk = latest(dotnet / "sdk")
    ref = latest(dotnet / "packs/Microsoft.NETCore.App.Ref") / "ref/net10.0"
    runtime = latest(dotnet / "shared/Microsoft.NETCore.App")
    harness = OUT / "FrozenConstructors.dll"
    response = OUT / "compile.rsp"
    response.write_text("\n".join([
        "-nologo", "-langversion:14", "-warnaserror+", "-nowarn:1701", "-target:exe",
        f'-out:"{harness}"', *[f'-r:"{p}"' for p in sorted(ref.glob("*.dll"))],
        *[f'-r:"{corpus / "assemblies" / name}"' for name in ["OpenRA.Game.dll", "OpenRA.Mods.Common.dll"]],
        f'"{ROOT / "tests/dotnet/FrozenConstructors.cs"}"', *[f'"{p}"' for p in sources],
    ]))
    run(dotnet / "dotnet", sdk / "Roslyn/bincore/csc.dll", "@" + str(response))
    harness.with_suffix(".runtimeconfig.json").write_text(json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    result = run(dotnet / "dotnet", harness, corpus, evidence, expected=None)
    (OUT / "result.txt").write_text(result.stdout + result.stderr)
    print(result.stdout + result.stderr, end="")
    if args.mutate:
        marker = "Must Info identity" if args.mutate == "must-info" else "Leaf order/derived Info"
        if result.returncode == 0 or marker not in result.stdout + result.stderr:
            raise RuntimeError("Mutation was not rejected by its intended assertion")
        print(f"Negative control {args.mutate}: rejected as expected")
    elif result.returncode != 0:
        raise RuntimeError("Frozen constructor behavior mismatch")


if __name__ == "__main__":
    main()
