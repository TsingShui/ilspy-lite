#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Offline post-preinitialization actual-default gate for three frozen singleton bodies."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/frozen-naming-singletons"
MVID = "daac77a9-0bf7-4155-8229-f659cf68d982"
CTOR_DIGEST = "f91e46a8b64a0553b4eec853ebcd316707ca3ffc72efd78572c42da029ec0730"
CONSTANT_DIGEST = "84a0936ff1fba6d2811d094757231685673447aed2170738872305076026a316"
# Protocol facts and hashes only. Raw input IL belongs in ignored reports.
OWNERS = [
    (2, "0x02000F97", "0x0600468B", "0x0600468C", "0x04003BD9",
     "83ba9efb6fc607bc6e5e3b2de66c9e78021d2a33efd57cd23c43db0d3e68af8c", "0x0600468E"),
    (19, "0x02000985", "0x06002C75", "0x06002C76", "0x040024C3",
     "4ab5764b580d3a5521d6e604d502481ddca1998d4d4a1d265d195de69d714b95", None),
]


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def digest(text):
    return hashlib.sha256(bytes.fromhex(text)).hexdigest()


def alias(raw):
    require("`" not in raw, "Unapproved generic shell context")
    return raw if re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", raw) and not raw.startswith(
        ("__name_", "__cil_", "__il_")) else "__name_" + raw.encode().hex().upper()


def closing(text, opening):
    depth = 0
    for token in re.finditer(r'//[^\n]*|/\*.*?\*/|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|[{}]', text[opening:], re.S):
        if token[0] == "{":
            depth += 1
        elif token[0] == "}":
            depth -= 1
            if depth == 0:
                return opening + token.end()
    raise RuntimeError("Unclosed actual default declaration")


def extract(text, owner, constant):
    """Bind unique metadata names/shapes inside the independently cataloged root owner."""
    name = alias(owner["path"][-1])
    declarations = list(re.finditer(r"(?m)^private sealed class ([A-Za-z_][A-Za-z_0-9]*)\n\{", text))
    require(len(declarations) == 1 and declarations[0][1] == name, "Unique independent root owner correspondence missing")
    declaration = declarations[0]
    end = closing(text, text.index("{", declaration.start(), declaration.end()))

    def member(pattern):
        matches = list(re.finditer(pattern, text[declaration.end():end]))
        require(len(matches) == 1, "Unique actual default member correspondence missing: " + pattern)
        start = declaration.end() + matches[0].start()
        opening = text.index("{", start, declaration.end() + matches[0].end())
        finish = closing(text, opening)
        require(finish < end, "Actual member escaped owner")
        actual = text[start:finish]
        require("fallback" not in actual.lower(), "Selected member contains fallback")
        return {"start": start, "end": finish, "bodyStart": opening, "member": actual,
                "body": text[opening:finish], "sha256": hashlib.sha256(actual.encode()).hexdigest()}

    ctor = member(r"(?m)^    public " + re.escape(name) + r"\(\) : base\(\)\n    \{")
    field_name = "__il_field_" + owner["singleton"][2:]
    original_type = owner["namespace"] + "." + ".".join(alias(p) for p in owner["path"])
    field_declaration = "    public static readonly " + original_type + " " + field_name + ";"
    require(text[declaration.end():end].count(field_declaration) == 1, "Actual singleton declaration target/type missing")
    result = {"constructor": ctor, "singletonDeclaration": field_declaration, "singletonName": field_name,
              "className": name, "adaptations": [
                  "Private sealed root becomes public sealed nested class inside ordinary SingletonShell context",
                  "Unused original fields/methods and fallback initializer omitted; no recovered type initializer",
                  "Singleton own-type qualification shortened to shell type; readonly removed for host fixture assignment",
                  "Actual constructor declaration, base initializer and body retained byte-for-byte",
                  "Actual internal constant declaration and body retained byte-for-byte when selected"]}
    if constant is not None:
        result["constantName"] = alias(constant["name"])
        result["constant"] = member(r"(?m)^    internal int " + re.escape(result["constantName"]) + r"\(\)\n    \{")
    return result


def shell(extraction, mutation=False, constructor_job=False):
    ctor = extraction["constructor"]["member"]
    if mutation and constructor_job:
        opening = ctor.index("{")
        ctor = ctor[:opening + 1] + '\n        throw new System.InvalidOperationException("frozen-singleton-ctor-mutation");\n    }'
    constant = extraction.get("constant", {}).get("member", "")
    if mutation and not constructor_job:
        require(constant.count("return 0;") == 1, "Constant mutation must alter one actual return")
        constant = constant.replace("return 0;", "return 1;")
    return ("public class SingletonShell\n{\npublic sealed class " + extraction["className"] + "\n{\n" +
            "public static " + extraction["className"] + " " + extraction["singletonName"] + ";\n" +
            ctor + "\n" + constant + "\n}\n}\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", required=True, type=Path, help="Existing absolute read-only OpenRA report directory")
    args = parser.parse_args()
    require(args.corpus.is_absolute(), "--corpus must be absolute")
    corpus = args.corpus.resolve()
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "summary.json").unlink(missing_ok=True)
    (OUT / "commands.jsonl").write_text("")

    def run(command, log, allowed=(0,), cwd=ROOT):
        result = subprocess.run(list(map(str, command)), cwd=cwd, capture_output=True, text=True, timeout=120)
        log.with_suffix(".stdout.txt").write_text(result.stdout)
        log.with_suffix(".stderr.txt").write_text(result.stderr)
        with (OUT / "commands.jsonl").open("a") as stream:
            stream.write(json.dumps({"command": list(map(str, command)), "exit": result.returncode, "log": str(log)}) + "\n")
        require(result.returncode in allowed, f"Unexpected exit {result.returncode}: {log}\n{result.stderr[-4000:]}")
        return result

    def latest10(directory):
        return max((p for p in directory.iterdir() if re.fullmatch(r"10\.\d+\.\d+", p.name)),
                   key=lambda p: tuple(map(int, p.name.split("."))))

    dotnet = Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10")).resolve()
    sdk = latest10(dotnet / "sdk")
    refs = sorted((latest10(dotnet / "packs/Microsoft.NETCore.App.Ref") / "ref/net10.0").glob("*.dll"))
    require(bool(refs), "Installed .NET 10 reference pack missing")

    def compile_cs(source, target, executable=False):
        response = target.with_suffix(".rsp")
        response.write_text("\n".join(["-nologo", "-langversion:14", "-warnaserror+", "-deterministic+", "-optimize+",
                                       "-target:exe" if executable else "-target:library", f'-out:"{target}"',
                                       *[f'-r:"{p}"' for p in refs], f'"{source}"']) + "\n")
        run([dotnet / "dotnet", sdk / "Roslyn/bincore/csc.dll", "@" + str(response)], target.with_suffix(".compile"))

    host = OUT / "FrozenNamingSingletons.dll"
    compile_cs(ROOT / "tests/dotnet/FrozenNamingSingletons.cs", host, True)
    host.with_suffix(".runtimeconfig.json").write_text(json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": latest10(dotnet / "shared/Microsoft.NETCore.App").name}}}))
    run(["cargo", "build", "--release", "--locked", "--offline"], OUT / "cargo-build")
    cli = ROOT / "target/release/ilspy-lite"
    frozen = json.loads((ROOT / "acceptance/openra/random-classes.json").read_text())
    require(frozen["count"] == len(frozen["classes"]) == 20, "Frozen selection changed")
    filename = "OpenRA.Mods.Common.dll"
    assembly = corpus / "assemblies" / filename
    assembly_hash = hashlib.sha256(assembly.read_bytes()).hexdigest()
    pin = json.loads((ROOT / "acceptance/openra/manifest.json").read_text())["assemblies"][filename]["sha256"]
    require(assembly_hash == pin, "Pinned original assembly hash mismatch")
    jobs = []
    for index, owner_token, initializer_token, ctor_token, singleton_token, init_digest, constant_token in OWNERS:
        entry = frozen["classes"][index - 1]
        require(entry["assembly"] == filename and entry["mvid"] == MVID and entry["token"] == owner_token, "Frozen owner identity changed")
        directory = OUT / f"{index:02}-{owner_token}"
        directory.mkdir(exist_ok=True)
        catalog = json.loads(run([dotnet / "dotnet", host, "catalog", assembly, owner_token], directory / "inventory").stdout)
        owner = catalog["owner"]
        require(catalog["mvid"] == MVID and owner["name"] == entry["name"] and not catalog["moduleInitializers"], "Static owner/MVID/module guard failed")
        require(owner["constructor"] == ctor_token and owner["singleton"] == singleton_token and owner["initializer"] == initializer_token,
                "Verified initializer target differs from pinned own constructor/singleton")
        methods = {m["token"]: m for m in owner["methods"]}
        for token, signature, expected_digest in [(initializer_token, "000001", init_digest), (ctor_token, "200001", CTOR_DIGEST)] + (
                [(constant_token, "200008", CONSTANT_DIGEST)] if constant_token else []):
            method = methods[token]
            require(method["signature"] == signature and digest(method["il"]) == expected_digest and method["eh"] == 0, "Pinned method protocol mismatch: " + token)
            raw = run([cli, "il", assembly, "--method", token], directory / (token + "-il")).stdout
            chunks = re.findall(r"(?m)^\.code-bytes IL_[0-9A-F]+: \[([^]]+)\]$", raw)
            require("".join(re.findall(r"\b[0-9A-F]{2}\b", " ".join(chunks))) == method["il"], "CLI/independent PE raw IL mismatch")
        text = run([cli, "decompile", assembly, "--type", entry["name"]], directory / "default", (0, 2)).stdout
        # Details contributes only method-local statuses, never source statements.
        details = run([cli, "decompile", assembly, "--type", entry["name"], "--details"], directory / "details", (0, 2)).stdout
        statuses = dict(re.findall(r"(?m)^\.method-status (0x[0-9A-F]{8}) (\w+)$", details))
        baseline_dirs = list((corpus / "random-classes/final-21c469ba6").glob(f"{index:02}-*"))
        require(len(baseline_dirs) == 1, "Missing frozen method baseline")
        baseline = json.loads((baseline_dirs[0] / "class.json").read_text())
        require(baseline["selection"] == entry, "Frozen baseline owner identity changed")
        old = {m["token"]: m["statuses"]["automatic"] for m in baseline["methods"]}
        require(set(old) == set(statuses) == set(methods), "Method-local inventory/status mismatch")
        (directory / "baseline.json").write_text(json.dumps(baseline, indent=2) + "\n")
        for token in [ctor_token] + ([constant_token] if constant_token else []):
            require(old[token] != "Decompiled" and statuses[token] == "Decompiled", "Selected restored body missing or baseline changed: " + token)
            constructor_job = token == ctor_token
            extraction = extract(text, owner, None if constructor_job else methods[token])
            record = {"assembly": filename, "catalog": catalog, "token": token, "baseline": old[token], "current": statuses[token],
                      "recoveredType": "SingletonShell+" + extraction["className"], "singletonName": extraction["singletonName"],
                      "extraction": extraction}
            if not constructor_job:
                record["constantName"] = extraction["constantName"]
            source = directory / (token + ".cs")
            source.write_text(shell(extraction))
            for member_name in ["constructor"] + ([] if constructor_job else ["constant"]):
                require(source.read_text().count(extraction[member_name]["member"]) == 1, "Actual positive member must occur once unchanged")
            evidence = directory / (token + ".json")
            evidence.write_text(json.dumps(record, separators=(",", ":")) + "\n")
            command = [dotnet / "dotnet", host, "execute", corpus, evidence]
            mutant_source = directory / (token + "-mutant.cs")
            mutant_source.write_text(shell(extraction, True, constructor_job))
            mutant = mutant_source.with_suffix(".dll")
            compile_cs(mutant_source, mutant)
            # Red first. A compiled body must execute and produce this specific
            # runtime mismatch; compiler errors and ABI/protocol guards do not count.
            negative = json.loads(run(command + [mutant, "mutation"], directory / (token + "-mutation"), (42,)).stdout)
            require(negative["status"] == "behavior-mismatch" and negative["token"] == token and negative["side"] == "recovered" and
                    negative["originalOracleSetupSucceeded"], "Unrelated mutation failure")
            if constructor_job:
                require(negative["kind"] == "recovered-constructor-threw" and negative["phase"] == "recovered-preinitialization" and
                        negative["exception"] == "System.InvalidOperationException" and negative["message"] == "frozen-singleton-ctor-mutation" and
                        negative["completedPairs"] == 0 and negative["selectedConstructor"] == token, "Ctor mutation must fail during actual independent fixture setup")
            else:
                require(negative["kind"] == "return-value" and negative["phase"] == "fresh-receiver", "Constant mutation did not reach postinit runtime comparison")
            recovered = source.with_suffix(".dll")
            compile_cs(source, recovered)
            positive = json.loads(run(command + [recovered], directory / (token + "-positive")).stdout)
            require(positive["status"] == "passed" and positive["pairs"] == (15 if constructor_job else 18) and
                    positive["recoveredIndependentSetupSucceeded"] and positive["singletonIdentityPreserved"] and not positive["originalStateCopied"],
                    "Positive postinit pair matrix incomplete")
            jobs.append({"token": token, "positive": positive, "mutation": negative, "evidence": str(evidence)})
            print(f"{token}: {positive['pairs']} positive pairs; compiled mutation exit 42 in {negative['phase']}", flush=True)
    require({j["token"] for j in jobs} == {"0x0600468C", "0x0600468E", "0x06002C76"} and len(jobs) == 3, "All three bodies must execute; no exclusions")
    summary = {"status": "passed", "bodies": 3, "pairs": sum(j["positive"]["pairs"] for j in jobs), "mutations": 3,
               "inputSha256": assembly_hash, "jobs": jobs,
               "limits": "Post-preinitialization fragments only, not full startup/type-initializer equivalence. Original cctor is oracle-only. Constructor mutants fail during recovered preinitialization, not the postinit matrix. Field-free warmed receivers have no dirty instance fields. No constructor null-this claim or final naming-goal approval."}
    (OUT / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(f"Passed: three bodies, {summary['pairs']} positive pairs, three runtime mutations; {OUT}")


if __name__ == "__main__":
    main()
