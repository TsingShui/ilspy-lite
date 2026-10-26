#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Offline actual-default Object-only constructor gate for frozen20 + supplemental3."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/frozen-naming-constructors"


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def latest10(directory):
    return max((p for p in directory.iterdir() if re.fullmatch(r"10\.\d+\.\d+", p.name)),
               key=lambda p: tuple(map(int, p.name.split("."))))


def alias(raw):
    require("`" not in raw, "Generic owner is outside this shell gate")
    return raw if re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", raw) and not raw.startswith(
        ("__name_", "__cil_", "__il_")) else "__name_" + raw.encode().hex().upper()


def closing(text, start):
    # Comments and literals cannot contribute structural braces.
    tokens = re.compile(r'//[^\n]*|/\*.*?\*/|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|[{}]', re.S)
    depth = 0
    for token in tokens.finditer(text, start):
        if token[0] == "{":
            depth += 1
        elif token[0] == "}":
            depth -= 1
            if depth == 0:
                return token.end()
    raise RuntimeError("Unclosed default declaration")


def extract(text, catalog, owner):
    """Bind the actual default declaration by independent owner tree and zero parameters."""
    declarations = []
    for match in re.finditer(r"(?m)^([ \t]*)(?:[A-Za-z_]+ )*class (@?[A-Za-z_][A-Za-z_0-9]*)[^\n]*\n[ \t]*\{", text):
        opening = text.index("{", match.start())
        declarations.append((match.start(), closing(text, opening), match[2].lstrip("@"), match[1]))
    root_path = catalog["types"][0]["path"]
    wanted = [alias(p) for p in owner["path"][len(root_path) - 1:]]
    matches = []
    for start, end, name, indent in declarations:
        chain = [d[2] for d in declarations if d[0] <= start and end <= d[1]]
        if chain == wanted:
            matches.append((start, end, name, indent))
    require(len(matches) == 1, f"Ambiguous/missing owner correspondence: {owner['name']}")
    start, end, name, indent = matches[0]
    pattern = (r"(?m)^" + re.escape(indent + "    ") + r"public " + re.escape(name) +
               r"\(\) : base\(\)\n[ \t]*\{")
    ctors = list(re.compile(pattern).finditer(text, start, end))
    require(len(ctors) == 1, f"Missing unique actual default constructor: {owner['name']}")
    match = ctors[0]
    finish = closing(text, text.index("{", match.start(), match.end()))
    require(finish < end, "Constructor escaped owner")
    member = text[match.start():finish]
    require("fallback" not in member.lower(), "Selected constructor has fallback")
    return name, member


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", type=Path, required=True, help="Existing absolute read-only OpenRA report directory")
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
        with (OUT / "commands.jsonl").open("a") as file:
            file.write(json.dumps({"command": list(map(str, command)), "exit": result.returncode,
                                   "log": str(log)}) + "\n")
        require(result.returncode in allowed, f"Command exit {result.returncode}: {log}\n{result.stderr[-4000:]}")
        return result

    dotnet = Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10")).resolve()
    sdk = latest10(dotnet / "sdk")
    refs = sorted((latest10(dotnet / "packs/Microsoft.NETCore.App.Ref") / "ref/net10.0").glob("*.dll"))
    require(bool(refs), "Missing installed reference pack")
    runtime = latest10(dotnet / "shared/Microsoft.NETCore.App")

    def compile_cs(source, target, executable=False):
        rsp = target.with_suffix(".rsp")
        rsp.write_text("\n".join(["-nologo", "-langversion:14", "-warnaserror+", "-deterministic+", "-optimize+",
                                  "-target:exe" if executable else "-target:library", f'-out:"{target}"',
                                  *[f'-r:"{p}"' for p in refs], f'"{source}"']) + "\n")
        run([dotnet / "dotnet", sdk / "Roslyn/bincore/csc.dll", "@" + str(rsp)], target.with_suffix(".compile"))

    host = OUT / "FrozenNamingConstructors.dll"
    compile_cs(ROOT / "tests/dotnet/FrozenNamingConstructors.cs", host, True)
    host.with_suffix(".runtimeconfig.json").write_text(json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    run(["cargo", "build", "--release", "--locked", "--offline"], OUT / "cargo-build")
    cli = ROOT / "target/release/ilspy-lite"
    frozen = json.loads((ROOT / "acceptance/openra/random-classes.json").read_text())
    supplemental = json.loads((ROOT / "acceptance/openra/naming-samples.json").read_text())
    require(frozen["count"] == len(frozen["classes"]) == 20 and len(supplemental["classes"]) == 3,
            "Frozen selection count changed")
    pins = json.loads((ROOT / "acceptance/openra/manifest.json").read_text())["assemblies"]
    input_hashes = {}
    for filename, pin in pins.items():
        actual = hashlib.sha256((corpus / "assemblies" / filename).read_bytes()).hexdigest()
        require(actual == pin["sha256"], f"Pinned assembly hash mismatch: {filename}")
        input_hashes[filename] = actual
    classified, candidates, jobs = [], [], []
    for index, entry in enumerate(frozen["classes"] + supplemental["classes"], 1):
        directory = OUT / f"{index:02}-{entry['token']}"
        directory.mkdir(exist_ok=True)
        assembly = corpus / "assemblies" / entry["assembly"]
        catalog = json.loads(run([dotnet / "dotnet", host, "catalog", assembly, entry["token"]], directory / "inventory").stdout)
        require(catalog["mvid"] == entry["mvid"] and catalog["types"][0]["name"] == entry["name"], "Frozen identity mismatch")
        require(not catalog["moduleInitializers"], "Unproven module initializer: do not execute input")
        text = run([cli, "decompile", assembly, "--type", entry["name"]], directory / "default", (0, 2)).stdout
        details = run([cli, "decompile", assembly, "--type", entry["name"], "--details"], directory / "details", (0, 2)).stdout
        statuses = dict(re.findall(r"(?m)^\.method-status (0x[0-9A-F]{8}) (\w+)$", details))
        methods = [m for t in catalog["types"] for m in t["methods"]]
        require(set(statuses) == {m["token"] for m in methods}, "Method-local status/inventory mismatch")
        if index <= 20:
            baseline_dirs = list((corpus / "random-classes/final-21c469ba6").glob(f"{index:02}-*"))
            require(len(baseline_dirs) == 1, "Missing frozen per-method baseline")
            baseline = json.loads((baseline_dirs[0] / "class.json").read_text())
            require(baseline["selection"] == entry, "Baseline selection mismatch")
            old = {m["token"]: m["statuses"]["automatic"] for m in baseline["methods"]}
            require(set(old) == set(statuses), "Baseline method inventory mismatch")
            (directory / "baseline.json").write_text(json.dumps(baseline, indent=2) + "\n")
        else:
            require(entry["baseline"]["decompiled"] == 0 and entry["baseline"]["partial"] == len(methods),
                    "Supplemental all-partial baseline changed")
            old = {m["token"]: "Partial" for m in methods}
            (directory / "baseline.json").write_text(json.dumps(entry, indent=2) + "\n")
        def original_il(method):
            result = run([cli, "il", assembly, "--method", method["token"]], directory / (method["token"] + "-il"))
            chunks = re.findall(r"(?m)^\.code-bytes IL_[0-9A-F]+: \[([^]]+)\]$", result.stdout)
            raw = "".join(re.findall(r"\b[0-9A-F]{2}\b", " ".join(chunks)))
            require(raw == method["il"], "CLI/independent PE original IL mismatch")

        for owner in catalog["types"]:
            for initializer in owner["initializers"]:
                original_il(initializer)
            for method in owner["methods"]:
                record = {"assembly": entry["assembly"], "mvid": entry["mvid"], "owner": owner["token"],
                          "ownerName": owner["name"], "token": method["token"], "name": method["name"],
                          "baseline": old[method["token"]], "current": statuses[method["token"]]}
                if method["name"] == ".ctor" and method["signature"] == "200001" and method["objectOnly"]:
                    # A previously recovered constructor becoming Partial is a
                    # regression, not a reason to silently shrink this gate.
                    require(record["current"] == "Decompiled", "Object-only constructor recovery regressed: " + method["token"])
                if record["current"] != "Decompiled":
                    reason = "not Decompiled; " + record["current"]
                elif record["baseline"] == "Decompiled":
                    reason = "already Decompiled in frozen baseline"
                elif method["name"] != ".ctor":
                    reason = "not an instance constructor (method/static initializer)"
                elif method["signature"] != "200001":
                    reason = "argument/generic constructor; outside parameterless gate"
                elif not method["objectOnly"]:
                    reason = "original IL is not exact core Object-only ldarg.0/call/ret"
                elif not owner["objectBase"] or owner["finalizer"]:
                    reason = "non-Object ancestry or finalizer; unsafe shell execution"
                elif owner["initializers"]:
                    reason = "unproven type initialization; .cctor IL saved in independent inventory"
                elif owner["generic"]:
                    reason = "generic owner requires unapproved shell context"
                else:
                    reason = "eligible"
                record["reason"] = reason
                classified.append(record)
                if reason != "eligible":
                    continue
                require(sum(m["name"] == ".ctor" and m["signature"] == "200001" for m in owner["methods"]) == 1,
                        "Ambiguous metadata constructor shape")
                original_il(method)
                name, member = extract(text, catalog, owner)
                # Only unused fields and the rest of the class are omitted. The
                # actual declaration, initializer, and body remain byte-for-byte.
                prefix = "public class ConstructorShell\n{\n    public class " + name + "\n    {\n"
                suffix = "\n    }\n}\n"
                source = directory / f"{method['token']}.cs"
                source.write_text(prefix + member + suffix)
                recovered = source.with_suffix(".dll")
                compile_cs(source, recovered)
                evidence = directory / f"{method['token']}.json"
                job = {**record, "il": method["il"], "recoveredType": "ConstructorShell+" + name}
                evidence.write_text(json.dumps(job, indent=2) + "\n")
                command = [dotnet / "dotnet", host, "execute", corpus, evidence, recovered]
                positive = json.loads(run(command, directory / f"{method['token']}-positive").stdout)
                require(positive["status"] == "passed" and positive["pairs"] == 8, "Positive execution did not cover finite pairs")
                # Mutate the actual selected body, not a stand-in or an ABI guard.
                opening = member.index("{")
                mutated = member[:opening + 1] + '\nthrow new System.InvalidOperationException("frozen-ctor-mutation");\n}'
                mutant_source = directory / f"{method['token']}-mutant.cs"
                mutant_source.write_text(prefix + mutated + suffix)
                mutant = mutant_source.with_suffix(".dll")
                compile_cs(mutant_source, mutant)
                negative = run(command[:-1] + [mutant, "mutation"], directory / f"{method['token']}-mutation", (42,))
                mutation = json.loads(negative.stdout)
                require(mutation["status"] == "behavior-mismatch" and mutation["kind"] == "recovered-constructor-threw" and
                        mutation["exception"] == "System.InvalidOperationException" and mutation["message"] == "frozen-ctor-mutation",
                        "Mutation rejected for unrelated reason")
                candidates.append(record)
                jobs.append({**record, "positive": positive, "mutation": mutation, "evidence": str(evidence)})
                print(f"{entry['assembly']} {method['token']}: 8 pairs; compiled throw mutant rejected at runtime", flush=True)
    require(bool(candidates), "No eligible constructors; vacuous gate")
    summary = {"status": "passed", "selectedTrees": 20, "supplementalTrees": 3,
               "eligibleCount": len(candidates), "pairs": 8 * len(candidates), "inputHashes": input_hashes,
               "classified": classified, "jobs": jobs,
               "limits": "Field-free constructor shells only; no full class ABI, interface, game, or omitted-field equivalence proof. No null receiver claims."}
    (OUT / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(f"Passed: {len(candidates)} eligible constructors, {summary['pairs']} pairs, {len(jobs)} runtime mutation rejections; {OUT}")


if __name__ == "__main__":
    main()
