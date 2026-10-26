#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Optional .NET 10 gate for actual default closure-class output, without restore."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
CLOSURE = "NamingCases.Factory/<>c__DisplayClass0_0"
OUT = ROOT / "report/naming-roundtrip"
EXPECTED = {
    "status": "passed", "constructorPairs": 24, "methodPairs": 147,
    "nullPairs": 7, "totalPairs": 178, "coreObjectShapes": 2,
    "noOriginalProxy": True,
}


def invoke(command, log, cwd=ROOT):
    """Keep compiler/build output on disk, showing only a bounded failure tail."""
    with log.open("w") as output:
        result = subprocess.run(list(map(str, command)), cwd=cwd, text=True,
                                stdout=output, stderr=subprocess.STDOUT, timeout=120)
    if result.returncode:
        raise RuntimeError(f"Command failed ({result.returncode}): {command}\n" +
                           log.read_text()[-6000:])


def latest10(path):
    return max((p for p in path.iterdir() if re.fullmatch(r"10\.\d+\.\d+", p.name)),
               key=lambda p: tuple(map(int, p.name.split("."))))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mutate-store", action="store_true", help=(
        "after both positive gates, replace the actual recovered store with zero in each "
        "configuration and require a runtime field-write mismatch (exit 42)"))
    args = parser.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    summary_path = OUT / "summary.json"
    summary_path.unlink(missing_ok=True)
    dotnet = Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10")).resolve()
    sdk = latest10(dotnet / "sdk")
    refs = sorted((latest10(dotnet / "packs/Microsoft.NETCore.App.Ref") / "ref/net10.0").glob("*.dll"))
    runtime = latest10(dotnet / "shared/Microsoft.NETCore.App")
    if not refs:
        raise RuntimeError("Installed .NET 10 reference pack is empty")

    def compile_cs(output, source, executable=False, optimized=True):
        response = output.with_suffix(".rsp")
        # Framework reference packs only: no original fixture reference, adapters,
        # detailed body, generated replacement method, or NuGet restore.
        response.write_text("\n".join([
            "-nologo", "-langversion:14", "-warnaserror+", "-deterministic+",
            "-target:exe" if executable else "-target:library",
            "-optimize+" if optimized else "-optimize-", f'-out:"{output}"',
            *[f'-r:"{p}"' for p in refs], f'"{source}"',
        ]) + "\n")
        invoke([dotnet / "dotnet", sdk / "Roslyn/bincore/csc.dll", f"@{response}"],
               output.with_suffix(".compile.log"))

    invoke(["cargo", "build", "--release", "--locked", "--offline"], OUT / "cargo-build.log")
    cli = ROOT / "target/release/ilspy-lite"
    host = OUT / "NamingRoundtrip.dll"
    compile_cs(host, ROOT / "tests/dotnet/NamingRoundtrip.cs", executable=True)
    host.with_suffix(".runtimeconfig.json").write_text(json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}) + "\n")

    def execute(assembly, recovered, log):
        result = subprocess.run([str(dotnet / "dotnet"), str(host), str(assembly), str(recovered)],
                                cwd=ROOT, capture_output=True, text=True, timeout=120)
        log.write_text(result.stdout + result.stderr)
        # Compiler failures, malformed output, IL/ABI validation failures, and
        # unrelated exceptions must never count as a successful negative control.
        try:
            report = json.loads(result.stdout)
        except json.JSONDecodeError as failure:
            raise RuntimeError(f"Execution did not produce a behavior report: {log}\n" +
                               (result.stdout + result.stderr)[-6000:]) from failure
        return result.returncode, report

    jobs = []
    for config in ("release", "debug"):
        directory = OUT / config
        directory.mkdir(exist_ok=True)
        assembly = ROOT / f"tests/fixtures/naming-{config}.bin"
        command = [str(cli), "decompile", str(assembly), "--type", CLOSURE]
        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=120)
        (directory / "default.stdout.cs").write_text(result.stdout)
        (directory / "default.stderr.log").write_text(result.stderr)
        if result.returncode or not re.search(r"^decompiled=2 declarations=0 partial=0 failed=0$", result.stderr, re.M):
            raise RuntimeError(f"Default class output is not fully admitted: {directory}")
        # The private nested declaration needs its original ordinary parent. The
        # entire actual stdout fragment (including bodies and comments) is intact.
        prefix = "namespace NamingCases\n{\npublic static class Factory\n{\n"
        suffix = "\n}\n}\n"
        source = directory / "Recovered.cs"
        source.write_text(prefix + result.stdout + suffix)
        recovered = directory / "Recovered.dll"
        compile_cs(recovered, source, optimized=config == "release")
        code, report = execute(assembly, recovered, directory / "baseline.log")
        if code != 0 or report != EXPECTED:
            raise RuntimeError(f"Positive execution gate failed: {directory}: {report}")
        print(f"{config}: 24 constructor + 147 nonnull method + 7 null method pairs passed", flush=True)
        jobs.append({"configuration": config, "command": command, "baseline": report,
                     "fragment": str(directory / "default.stdout.cs")})

    if args.mutate_store:
        for job in jobs:
            config = job["configuration"]
            directory = OUT / config
            fragment = (directory / "default.stdout.cs").read_text()
            # Change just the actual recovered captured-field store to a wrong
            # constant. Keep a store so warnings-as-errors still accepts the field.
            mutated, count = re.subn(r"(?m)^([ \t]*)this\.value = [^\r\n]+;[ \t]*$",
                                     r"\1this.value = 0;", fragment)
            if count != 1:
                raise RuntimeError(f"Expected exactly one recovered value store, found {count}")
            source = directory / "Mutated.cs"
            source.write_text(prefix + mutated + suffix)
            recovered = directory / "Mutated.dll"
            compile_cs(recovered, source, optimized=config == "release")
            code, report = execute(ROOT / f"tests/fixtures/naming-{config}.bin", recovered,
                                   directory / "mutation.log")
            if code != 42 or report.get("status") != "behavior-mismatch" or report.get("kind") != "field-write":
                raise RuntimeError(f"Mutation was not rejected by the field-write gate: {directory}: exit={code}, {report}")
            job["mutation"] = {"exitCode": code, **report}
            print(f"{config}: zero-store mutation rejected by runtime field-write comparison (exit 42)", flush=True)
    summary_path.write_text(json.dumps({"jobs": jobs, "totalPositivePairs": 356,
                                       "mutationRequested": args.mutate_store}, indent=2) + "\n")
    print(f"Report: {summary_path}")


if __name__ == "__main__":
    main()
