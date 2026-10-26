#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Compile actual default declaration/reference output from owned naming variants."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/naming-declarations"


def latest10(path):
    return max((p for p in path.iterdir() if re.fullmatch(r"10\.\d+\.\d+", p.name)),
               key=lambda p: tuple(map(int, p.name.split("."))))


def replace_string(image, old, new):
    assert len(old) == len(new)
    needle = old + b"\0"
    if image.count(needle) != 1:
        raise RuntimeError(f"Expected one owned string: {old!r}")
    return image.replace(needle, new + b"\0")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mutate-collision", action="store_true")
    args = parser.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "summary.json").unlink(missing_ok=True)
    dotnet = Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10")).resolve()
    sdk = latest10(dotnet / "sdk")
    refs = sorted((latest10(dotnet / "packs/Microsoft.NETCore.App.Ref") / "ref/net10.0").glob("*.dll"))
    if not refs:
        raise RuntimeError("Installed reference pack is empty")
    build = subprocess.run(["cargo", "build", "--release", "--locked", "--offline"],
                           cwd=ROOT, capture_output=True, text=True, timeout=120)
    (OUT / "cargo-build.log").write_text(build.stdout + build.stderr)
    build.check_returncode()
    cli = ROOT / "target/release/ilspy-lite"
    variants = [("ordinary", None, False), ("generated", b"<X>__", False),
                ("dotted-record-name", b"X.Y__", False), ("unicode", b"\xc3\xa9abc", False),
                ("namespace-keyword", None, True), ("generated-and-keyword", b"<X>__", True)]
    records = []

    def compile_source(directory, source, label, expect_duplicate=False):
        output = directory / f"{label}.dll"
        response = directory / f"{label}.rsp"
        response.write_text("\n".join([
            "-nologo", "-target:library", "-langversion:14", "-warnaserror+", "-deterministic+",
            f'-out:"{output}"', *[f'-r:"{p}"' for p in refs], f'"{source}"',
        ]) + "\n")
        result = subprocess.run([str(dotnet / "dotnet"), str(sdk / "Roslyn/bincore/csc.dll"),
                                 f"@{response}"], capture_output=True, text=True, timeout=120)
        (directory / f"{label}.compile.log").write_text(result.stdout + result.stderr)
        if expect_duplicate:
            if result.returncode == 0 or not re.search(r"error CS0102\b", result.stdout + result.stderr):
                raise RuntimeError("Alias mutation must fail specifically as a duplicate declaration")
        elif result.returncode:
            raise RuntimeError(f"Actual declarations do not compile: {source}\n{result.stdout}{result.stderr}")
        return result.returncode

    for config in ("release", "debug"):
        original = (ROOT / f"tests/fixtures/naming-{config}.bin").read_bytes()
        for label, typename, keyword_namespace in variants:
            directory = OUT / config / label
            directory.mkdir(parents=True, exist_ok=True)
            image = original if typename is None else replace_string(original, b"Plain", typename)
            if keyword_namespace:
                image = replace_string(image, b"NamingCases", b"namespace.a")
            assembly = directory / "Input.bin"
            assembly.write_bytes(image)
            namespace = "namespace.a" if keyword_namespace else "NamingCases"
            command = [str(cli), "decompile", str(assembly), "--type", namespace + ".CollisionHost"]
            result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=120)
            source = directory / "Actual.cs"
            source.write_text(result.stdout)
            (directory / "default.stderr.log").write_text(result.stderr)
            if result.returncode or not re.search(r"^decompiled=3 declarations=0 partial=0 failed=0$", result.stderr, re.M):
                raise RuntimeError(f"Owned declaration variant must remain supported: {directory}")
            compile_source(directory, source, "Actual")
            record = {"configuration": config, "variant": label, "command": command,
                      "source": str(source), "compileExitCode": 0, "noRuntimeProof": True}
            if args.mutate_collision and label == "generated":
                # The two distinct actual names must not be conflated. This is a
                # compiler sensitivity check, not a runtime-behavior comparison.
                user_alias = "__name_5F5F6E616D655F33433538334535463546"
                if result.stdout.count(user_alias) < 3:
                    raise RuntimeError("Expected the reserved user alias in declaration and references")
                mutant = directory / "Duplicate.cs"
                mutant.write_text(result.stdout.replace(user_alias, "__name_3C583E5F5F"))
                record["duplicateMutationExitCode"] = compile_source(directory, mutant, "Duplicate", True)
            records.append(record)
            print(f"{config}/{label}: actual declarations and references compile")
    (OUT / "summary.json").write_text(json.dumps({"compilations": records,
        "positiveCompilations": len(records), "noRuntimeProof": True}, indent=2) + "\n")
    print(f"Report: {OUT / 'summary.json'}")


if __name__ == "__main__":
    main()
