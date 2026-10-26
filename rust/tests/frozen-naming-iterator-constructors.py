#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Paired actual-default argument constructor execution, not iterator reconstruction."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/frozen-naming-iterator-constructors"


def require(value, reason):
    if not value:
        raise RuntimeError(reason)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", required=True, type=Path)
    args = parser.parse_args()
    require(args.corpus.is_absolute(), "--corpus must be absolute")
    corpus = args.corpus.resolve()
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "summary.json").unlink(missing_ok=True)
    (OUT / "commands.jsonl").write_text("")
    helper_spec = importlib.util.spec_from_file_location("field_free_ctor_test", ROOT / "tests/frozen-naming-constructors.py")
    helper = importlib.util.module_from_spec(helper_spec)
    helper_spec.loader.exec_module(helper)

    def run(command, log, allowed=(0,), cwd=ROOT):
        result = subprocess.run(list(map(str, command)), cwd=cwd, text=True, capture_output=True, timeout=120)
        log.with_suffix(".stdout.txt").write_text(result.stdout)
        log.with_suffix(".stderr.txt").write_text(result.stderr)
        with (OUT / "commands.jsonl").open("a") as file:
            file.write(json.dumps({"command": list(map(str, command)), "exit": result.returncode, "log": str(log)}) + "\n")
        require(result.returncode in allowed, f"Unexpected exit {result.returncode}: {log}\n{result.stdout[-3000:]}{result.stderr[-3000:]}")
        return result

    dotnet = Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10")).resolve()
    sdk = helper.latest10(dotnet / "sdk")
    refs = sorted((helper.latest10(dotnet / "packs/Microsoft.NETCore.App.Ref") / "ref/net10.0").glob("*.dll"))
    require(bool(refs), "Installed reference pack is empty")

    def compile_cs(source, dll, executable=False):
        rsp = dll.with_suffix(".rsp")
        rsp.write_text("\n".join(["-nologo", "-langversion:14", "-warnaserror+", "-deterministic+", "-optimize+",
            "-target:exe" if executable else "-target:library", f'-out:"{dll}"',
            *[f'-r:"{p}"' for p in refs], f'"{source}"']) + "\n")
        run([dotnet / "dotnet", sdk / "Roslyn/bincore/csc.dll", "@" + str(rsp)], dll.with_suffix(".compile"))

    host = OUT / "FrozenNamingIteratorConstructors.dll"
    compile_cs(ROOT / "tests/dotnet/FrozenNamingIteratorConstructors.cs", host, True)
    host.with_suffix(".runtimeconfig.json").write_text(json.dumps({"runtimeOptions": {"tfm": "net10.0",
        "framework": {"name": "Microsoft.NETCore.App", "version": helper.latest10(dotnet / "shared/Microsoft.NETCore.App").name}}}))
    run(["cargo", "build", "--release", "--locked", "--offline"], OUT / "cargo-build")
    cli = ROOT / "target/release/ilspy-lite"
    frozen = json.loads((ROOT / "acceptance/openra/random-classes.json").read_text())
    require(frozen["count"] == len(frozen["classes"]) == 20, "Frozen20 must not change")
    pins = json.loads((ROOT / "acceptance/openra/manifest.json").read_text())["assemblies"]
    jobs = []
    for index, token in [(8, "0x06001477"), (9, "0x0600276F")]:
        entry = frozen["classes"][index]
        directory = OUT / token
        directory.mkdir(exist_ok=True)
        assembly = corpus / "assemblies" / entry["assembly"]
        require(hashlib.sha256(assembly.read_bytes()).hexdigest() == pins[entry["assembly"]]["sha256"], "Pinned input changed")
        catalog = json.loads(run([dotnet / "dotnet", host, "catalog", assembly, token], directory / "catalog").stdout)
        require(catalog["mvid"] == entry["mvid"] and catalog["ownerName"] == entry["name"] and catalog["owner"] == entry["token"],
                "Frozen owner/token/MVID mismatch")
        text = run([cli, "decompile", assembly, "--type", entry["name"]], directory / "default", (0, 2)).stdout
        declarations = []
        for match in re.finditer(r"(?m)^([ \t]*)(?:[A-Za-z_]+ )*class (@?[A-Za-z_][A-Za-z_0-9]*)[^\n]*\n[ \t]*\{", text):
            declarations.append((match.start(), helper.closing(text, text.index("{", match.start())), match[2], match[1]))
        name = helper.alias(catalog["path"][-1])
        owners = [d for d in declarations if d[2] == name and not any(p[0] < d[0] and d[1] < p[1] for p in declarations)]
        require(len(owners) == 1, "Independent root owner/default declaration mismatch")
        start, end, _, indent = owners[0]
        pattern = r"(?m)^" + re.escape(indent + "    ") + r"public " + re.escape(name) + r"\(int (@?[A-Za-z_][A-Za-z_0-9]*)\) : base\(\)\n[ \t]*\{"
        matches = list(re.compile(pattern).finditer(text, start, end))
        require(len(matches) == 1, "Unique actual one-I4 constructor missing")
        match = matches[0]
        finish = helper.closing(text, text.index("{", match.start(), match.end()))
        require(finish < end, "Constructor escaped owner")
        member = text[match.start():finish]
        require("fallback" not in member.lower(), "Constructor must be supported")
        # Test syntax adapter only: csc forbids explicit accessor method syntax.
        # The loaded IL guard proves the replacement calls that exact core getter.
        old = "System.Environment.get_CurrentManagedThreadId()"
        new = "global::System.Environment.CurrentManagedThreadId"
        require(member.count(old) == 1, "Exactly one actual thread getter call expected")
        adapted = member.replace(old, new)
        for role in ("state", "thread"):
            catalog[role]["alias"] = "__il_field_" + catalog[role]["token"][2:]
        state, thread = catalog["state"]["alias"], catalog["thread"]["alias"]
        prefix = f"public class IteratorShell\n{{\npublic class {name}\n{{\npublic int {state};\npublic int {thread};\n"
        suffix = "\n}\n}\n"
        catalog.update(assembly=entry["assembly"], recoveredType="IteratorShell+" + name,
                       sourceStart=match.start(), sourceEnd=finish, actualMember=member, adaptedMember=adapted,
                       syntaxAdapter={"from": old, "to": new, "count": 1},
                       limits="Observed two-I4 fields; no full iterator/class ABI or cross-runtime proof")
        evidence = directory / "evidence.json"
        evidence.write_text(json.dumps(catalog, indent=2) + "\n")
        source = directory / "Recovered.cs"
        source.write_text(prefix + adapted + suffix)
        recovered = source.with_suffix(".dll")
        compile_cs(source, recovered)
        command = [dotnet / "dotnet", host, "execute", corpus, evidence]
        positive = json.loads(run(command + [recovered], directory / "positive").stdout)
        require(positive["status"] == "passed" and positive["pairs"] == 140 and positive["nullPairs"] == 14 and
                len(set(positive["threads"])) == 2, "Finite two-thread constructor gate failed")
        mutations = []
        for role, original, replacement, mismatch in [
            ("state", f"this.{state} = {match[1]};", f"this.{state} = 0;", "state-write"),
            ("thread", f"this.{thread} = v4;", f"this.{thread} = unchecked(v4 - v4);", "thread-write"),
        ]:
            require(adapted.count(original) == 1, "Mutation must change one actual store")
            mutant = directory / f"Mutated-{role}.cs"
            mutant.write_text(prefix + adapted.replace(original, replacement) + suffix)
            mutant_dll = mutant.with_suffix(".dll")
            compile_cs(mutant, mutant_dll)
            negative = json.loads(run(command + [mutant_dll], directory / (role + "-mutation"), (42,)).stdout)
            require(negative["status"] == "behavior-mismatch" and negative["kind"] == mismatch and negative["token"] == token,
                    "Wrong mutation failure kind")
            mutations.append(negative)
        jobs.append({"assembly": entry["assembly"], "token": token, "positive": positive, "mutations": mutations, "evidence": str(evidence)})
        print(f"{entry['assembly']} {token}: 140 pairs, 14 null pairs, two threads; two compiled store mutants rejected", flush=True)
    summary = {"status": "passed", "constructors": 2, "pairs": 280, "nullPairs": 28, "mutations": 4, "jobs": jobs,
               "limits": "Actual constructor fragments with exact getter syntax adaptation; no iterator protocol, full ABI, gameplay or other-runtime proof"}
    (OUT / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(f"Report: {OUT / 'summary.json'}")


if __name__ == "__main__":
    main()
