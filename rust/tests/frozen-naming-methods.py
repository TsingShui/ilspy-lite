#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Offline paired execution of frozen naming scalar bodies from actual --type output."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/frozen-naming-methods"
# Pinned metadata signatures and IL digests; original bodies stay in ignored reports.
SPECS = {
    "0x0600468E": (2, "excluded-cctor", "200008", "84a0936ff1fba6d2811d094757231685673447aed2170738872305076026a316", []),
    "0x0600147B": (9, "string-getter", "20000E", "431766305968577215cc4ec963ff1217694b63cbc03419c98758fbd4f2f89a65", ["0x04000E40"]),
    "0x06002770": (10, "dispose-store", "200001", "6395ba04ed43e1e88d442eddbff0fd0540e4a87264c73b692a2148e8e97a74f8", ["0x040020A6"]),
    "0x06002772": (10, "string-getter", "20000E", "d88158fae6e935bf9813a0057bc12d7283ed75617b6b9ad969e4f65a4d5e534f", ["0x040020A7"]),
    "0x060021A3": (11, "empty-return", "200101128095", "684888c0ebb17f374298b65ee2807526c066094c701bcc7ebbe1c1095f494fc1", ["0x04001B90"]),
    "0x060021A5": (11, "clear-visible", "200001", "ae9118b2cb15339cb9af6d742e42c2badcf513ba188fe476cd3e56b4d9c4ec6e", ["0x04001B90"]),
    "0x060045A6": (11, "negate-visible", "200002", "f71f99350c31e97b7d453f0f35a295513618808f27566694da49b464071ff252", ["0x04003AE3", "0x04001B90"]),
    "0x0600338F": (18, "string-getter", "20000E", "432280ced84b542ed79df1a198c2c3e7faafb64ffab53fd912052f7fcf5bd61d", ["0x04002AD9"]),
}
PAIRS = {"string-getter": 30, "dispose-store": 30, "empty-return": 9, "clear-visible": 9, "negate-visible": 12}
MISMATCH = {"string-getter": "return-identity", "dispose-store": "store-state", "empty-return": "preserved-state",
            "clear-visible": "store-state", "negate-visible": "return-value"}


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


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


def extract(text, catalog, owner, method, family):
    declarations = []
    for match in re.finditer(r"(?m)^([ \t]*)(?:[A-Za-z_]+ )*class (@?[A-Za-z_][A-Za-z_0-9]*)[^\n]*\n[ \t]*\{", text):
        declarations.append((match.start(), closing(text, text.index("{", match.start())), match[2], match[1]))
    root_path = catalog["types"][0]["path"]
    wanted = [alias(p) for p in owner["path"][len(root_path) - 1:]]
    matches = [d for d in declarations if [p[2] for p in declarations if p[0] <= d[0] and d[1] <= p[1]] == wanted]
    require(len(matches) == 1, "Independent owner/default tree correspondence is ambiguous")
    start, end, _, indent = matches[0]
    friendly = {"0x060045A6": ("<.ctor>b__1", "CtorLambda1"),
                "0x0600338F": ("<.ctor>b__2", "CtorLambda2")}
    if method["token"] in friendly:
        raw, spelling = friendly[method["token"]]
        require(method["name"] == raw, "Independent selected generated method name changed")
    else:
        spelling = alias(method["name"])
    result_type = "string" if family == "string-getter" else "bool" if family == "negate-visible" else "int" if family == "excluded-cctor" else "void"
    parameters = "OpenRA.Actor self" if family == "empty-return" else ""
    pattern = (r"(?m)^" + re.escape(indent + "    ") + r"(private virtual|private|internal) " + result_type +
               " " + spelling + r"\(" + re.escape(parameters) + r"\)\n[ \t]*\{")
    found = list(re.compile(pattern).finditer(text, start, end))
    require(len(found) == 1, "Unique metadata name/signature/default method correspondence missing")
    match = found[0]
    opening = text.index("{", match.start(), match.end())
    finish = closing(text, opening)
    require(finish < end, "Method escaped its independent owner")
    body = text[opening:finish]
    require("fallback" not in body.lower(), "Selected body is not recovered")
    header = "public " + result_type + " " + spelling + "(" + ("object self" if parameters else "") + ")\n"
    return {"start": match.start(), "end": finish, "bodyStart": opening, "actualHeader": text[match.start():opening],
            "actualMember": text[match.start():finish], "body": body, "shellHeader": header, "methodName": spelling,
            "bodySha256": hashlib.sha256(body.encode()).hexdigest(),
            "adaptation": "Public nonvirtual test header; unused Actor parameter becomes object for null-only calls" if parameters else
                          "Public nonvirtual test header; actual name, parameter and return shapes retained"}


def shell(owner, fields, header, body):
    prefix = "namespace " + owner["namespace"] + "\n{\n"
    for depth, raw in enumerate(owner["path"], 1):
        prefix += "public class " + alias(raw) + "\n{\n"
        for field in fields:
            if field["ownerPath"] == owner["path"][:depth]:
                prefix += "public " + field["shellType"] + " " + field["shellName"] + ";\n"
    return prefix + header + body + "\n" + "}\n" * (len(owner["path"]) + 1)


def mutant(body, family):
    old, new = {
        "string-getter": ("return v1;", "return null;"),
        "dispose-store": ("= -2;", "= 0;"),
        "empty-return": ("return;", "this.visible = true;\n        return;"),
        "clear-visible": ("this.visible = (0 != 0);", "this.visible = (1 != 0);"),
        "negate-visible": ("return (v2 == 0);", "return (v2 != 0);"),
    }[family]
    require(body.count(old) == 1, "Mutation must alter exactly one actual body occurrence")
    return body.replace(old, new)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", required=True, type=Path, help="Existing absolute read-only OpenRA report cache")
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

    def latest10(path):
        return max((p for p in path.iterdir() if re.fullmatch(r"10\.\d+\.\d+", p.name)),
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

    host = OUT / "FrozenNamingMethods.dll"
    compile_cs(ROOT / "tests/dotnet/FrozenNamingMethods.cs", host, True)
    host.with_suffix(".runtimeconfig.json").write_text(json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": latest10(dotnet / "shared/Microsoft.NETCore.App").name}}}))
    run(["cargo", "build", "--release", "--locked", "--offline"], OUT / "cargo-build")
    cli = ROOT / "target/release/ilspy-lite"
    manifest = json.loads((ROOT / "acceptance/openra/random-classes.json").read_text())
    require(manifest["count"] == len(manifest["classes"]) == 20, "Frozen20 changed")
    hashes = {}
    for filename, pin in json.loads((ROOT / "acceptance/openra/manifest.json").read_text())["assemblies"].items():
        hashes[filename] = hashlib.sha256((corpus / "assemblies" / filename).read_bytes()).hexdigest()
        require(hashes[filename] == pin["sha256"], "Pinned original assembly hash mismatch")
    jobs, exclusions, observed = [], [], set()
    for index, entry in enumerate(manifest["classes"], 1):
        directory = OUT / f"{index:02}-{entry['token']}"
        directory.mkdir(exist_ok=True)
        assembly = corpus / "assemblies" / entry["assembly"]
        catalog = json.loads(run([dotnet / "dotnet", host, "catalog", assembly, entry["token"]], directory / "inventory").stdout)
        require(catalog["mvid"] == entry["mvid"] and catalog["types"][0]["name"] == entry["name"], "Frozen owner identity mismatch")
        require(not catalog["moduleInitializers"], "Unsafe/unproven module initialization")
        text = run([cli, "decompile", assembly, "--type", entry["name"]], directory / "default", (0, 2)).stdout
        # Only status records are consumed from Details, never source statements.
        details = run([cli, "decompile", assembly, "--type", entry["name"], "--details"], directory / "details", (0, 2)).stdout
        statuses = dict(re.findall(r"(?m)^\.method-status (0x[0-9A-F]{8}) (\w+)$", details))
        baseline_dirs = list((corpus / "random-classes/final-21c469ba6").glob(f"{index:02}-*"))
        require(len(baseline_dirs) == 1, "Missing independent frozen per-method baseline")
        baseline = json.loads((baseline_dirs[0] / "class.json").read_text())
        require(baseline["selection"] == entry, "Baseline owner selection mismatch")
        old = {m["token"]: m["statuses"]["automatic"] for m in baseline["methods"]}
        methods = [m for t in catalog["types"] for m in t["methods"]]
        require(set(statuses) == set(old) == {m["token"] for m in methods}, "Method-local inventory/status correspondence failed")
        (directory / "baseline.json").write_text(json.dumps(baseline, indent=2) + "\n")
        for owner in catalog["types"]:
            for method in owner["methods"]:
                token = method["token"]
                is_target = token in SPECS and SPECS[token][0] == index
                newly_recovered = old[token] != "Decompiled" and statuses[token] == "Decompiled" and method["name"] not in (".ctor", ".cctor")
                require(is_target == newly_recovered, "New scalar method set changed or recovery regressed; scope decision required: " + token)
                if not is_target:
                    continue
                observed.add(token)
                _, family, signature, il_hash, field_tokens = SPECS[token]
                require(method["signature"] == signature and hashlib.sha256(bytes.fromhex(method["il"])).hexdigest() == il_hash and method["eh"] == 0, "Pinned independent method protocol mismatch")
                require(sum(m["name"] == method["name"] and m["signature"] == signature for m in owner["methods"]) == 1,
                        "Ambiguous independent owner/name/signature")
                original_il = run([cli, "il", assembly, "--method", token], directory / (token + "-il")).stdout
                chunks = re.findall(r"(?m)^\.code-bytes IL_[0-9A-F]+: \[([^]]+)\]$", original_il)
                require("".join(re.findall(r"\b[0-9A-F]{2}\b", " ".join(chunks))) == method["il"], "CLI/PEReader original IL mismatch")
                extraction = extract(text, catalog, owner, method, family)
                record = {"assembly": entry["assembly"], "mvid": entry["mvid"], "owner": owner, "method": method,
                          "family": family, "extraction": extraction, "baseline": old[token], "current": statuses[token]}
                if family == "excluded-cctor":
                    require(len(owner["initializers"]) == 1, "Expected allocating cctor exclusion missing")
                    initializer = owner["initializers"][0]
                    require(initializer["token"] == "0x0600468B" and hashlib.sha256(bytes.fromhex(initializer["il"])).hexdigest() == "83ba9efb6fc607bc6e5e3b2de66c9e78021d2a33efd57cd23c43db0d3e68af8c", "Pinned cctor allocator changed")
                    run([cli, "il", assembly, "--method", initializer["token"]], directory / "excluded-cctor-il")
                    record["reason"] = "Excluded: .cctor allocates selected-original ctor 0x0600468C and writes singleton field; never executed"
                    exclusions.append(record)
                    continue
                require(not owner["initializers"] and not owner["generic"] and not owner["finalizer"], "Unsafe original owner")
                fields = []
                for field_token in field_tokens:
                    owners = [(t, f) for t in catalog["types"] for f in t["fields"] if f["token"] == field_token]
                    require(len(owners) == 1, "Independent observed field identity missing")
                    field_owner, field = owners[0]
                    require(not field_owner["initializers"] and not field_owner["finalizer"], "Unsafe referent initialization")
                    field_type = {"060E": "string", "0608": "int", "0602": "bool"}.get(field["signature"])
                    if field_type is None:
                        require(family == "negate-visible" and field_token == "0x04003AE3" and field["signature"] == "06129C50",
                                "Unapproved observed reference field")
                        field_type = "global::OpenRA.Mods.Common.Traits.Render.WithHarvestOverlay"
                    fields.append({**field, "ownerToken": field_owner["token"], "ownerPath": field_owner["path"],
                                   "shellName": field["name"] if re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", field["name"]) else "__il_field_" + field_token[2:],
                                   "shellType": field_type})
                record["fields"] = fields
                record["recoveredType"] = owner["namespace"] + "." + "+".join(alias(p) for p in owner["path"])
                source = directory / (token + ".cs")
                source.write_text(shell(owner, fields, extraction["shellHeader"], extraction["body"]))
                require(source.read_text().count(extraction["body"]) == 1, "Actual body must occur once, unchanged, in wrapped source")
                evidence = directory / (token + ".json")
                evidence.write_text(json.dumps(record, indent=2) + "\n")
                mutation_source = directory / (token + "-mutant.cs")
                mutation_source.write_text(shell(owner, fields, extraction["shellHeader"], mutant(extraction["body"], family)))
                mutation_dll = mutation_source.with_suffix(".dll")
                compile_cs(mutation_source, mutation_dll)
                command = [dotnet / "dotnet", host, "execute", corpus, evidence]
                # Behavioral red first: guards/compilation must succeed; only the
                # specific wrong return/store reported by execution is accepted.
                negative = json.loads(run(command + [mutation_dll], directory / (token + "-mutation"), (42,)).stdout)
                require(negative["status"] == "behavior-mismatch" and negative["kind"] == MISMATCH[family] and
                        negative["token"] == token and negative["side"] == "recovered", "Unrelated mutation rejection")
                recovered = source.with_suffix(".dll")
                compile_cs(source, recovered)
                positive = json.loads(run(command + [recovered], directory / (token + "-positive")).stdout)
                require(positive["status"] == "passed" and positive["pairs"] == PAIRS[family], "Finite positive cases not covered")
                jobs.append({"token": token, "assembly": entry["assembly"], "family": family, "positive": positive,
                             "mutation": negative, "evidence": str(evidence)})
                print(f"{entry['assembly']} {token}: {positive['pairs']} pairs; compiled mutation rejected: {negative['kind']} (42)", flush=True)
    require(observed == set(SPECS) and len(jobs) == 7 and len(exclusions) == 1, "Nonvacuous fixed eight-method coverage failed")
    summary = {"status": "passed", "safeMethods": len(jobs), "pairs": sum(j["positive"]["pairs"] for j in jobs),
               "inputHashes": hashes, "jobs": jobs, "exclusions": exclusions,
               "limits": "Bounded scalar/observed-field proof, not full class ABI/interface/game behavior. No original constructors or factories."}
    (OUT / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(f"Passed: seven methods, {summary['pairs']} pairs, seven runtime mutations; one allocating-cctor exclusion; {OUT}")


if __name__ == "__main__":
    main()
