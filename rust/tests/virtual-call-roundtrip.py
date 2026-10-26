#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Execute recovered callvirt bodies; dispatch and null checks stay target-runtime."""
import argparse, importlib.util, json, re, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "report/virtual-call-roundtrip"

def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec); spec.loader.exec_module(result); return result

support = module("object_roundtrip", ROOT / "tests/object-roundtrip.py")
adapter = module("reference_roundtrip", ROOT / "tests/reference-roundtrip.py")
leaves = module("exact_reference_leaves", ROOT / "tests/support/exact_reference_leaves.py")
generic = module("generic_declaring", ROOT / "tests/generic-declaring-roundtrip.py")

SOURCES = ([ROOT / "tests/dotnet/GenericReferenceRoundtrip.cs"]
           + sorted((ROOT / "tests/dotnet").glob("*Behavior.cs")))



def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/ilspy-lite")
    parser.add_argument("--readable", action="store_true")
    parser.add_argument("--mutation-control", action="store_true")
    args = parser.parse_args()
    if not args.cli.is_file(): parser.error("Existing Rust CLI required")
    OUT.mkdir(parents=True, exist_ok=True)
    print(support.run([sys.executable, ROOT / "tests/build-virtual-call-fixtures.py"]).stdout, end="", flush=True)
    harness = OUT / "VirtualCallRoundtrip.dll"
    compile_ms = support.compile_cs(harness, SOURCES, executable=True)
    runtime = support.latest(support.DOTNET / "shared/Microsoft.NETCore.App")
    support.write_changed(harness.with_suffix(".runtimeconfig.json"), json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    rows = []
    for config in ("release", "debug"):
        fixture = ROOT / f"tests/fixtures/virtual-call-{config}.bin"
        names, owners = {}, {}
        for line in support.run([args.cli, "list", fixture]).stdout.splitlines():
            token, signature = line.split(" ", 1)
            method = re.search(r"(\S+)::([A-Za-z][A-Za-z0-9_]*)\(", signature)
            if method:
                owners[token] = method.group(1)
                names[token] = method.group(2)
        # Selection follows the production scan, so the behavior gate always
        # matches exactly what the decompiler admits.
        selected = [(record["token"], names[record["token"]])
                    for record in map(json.loads, support.run([args.cli, "scan", fixture]).stdout.splitlines())
                    if record["status"] == "Decompiled"
                    and owners.get(record["token"]) == "VirtualCallCases.Ops"]
        if not selected: raise ValueError("virtual-call selection is empty")
        snippets = []
        for token, name in selected:
            details = support.run([args.cli, "decompile", fixture, "--method", token, "--details", "--no-auto-references"])
            if details.stderr.strip() != "decompiled=1 declarations=0 partial=0 failed=0":
                raise ValueError("virtual-call method not recovered: " + name)
            (OUT / f"{config}-{name}.details.txt").write_text(details.stdout)
            statements = details.stdout
            if args.readable:
                readable = support.run([args.cli, "decompile", fixture, "--method", token, "--no-auto-references"])
                if readable.stderr != details.stderr: raise ValueError("view changed admission")
                (OUT / f"{config}-{name}.readable.txt").write_text(readable.stdout)
                statements = adapter.adapt_readable_body(readable.stdout, details.stdout)

            if name == "UnsupportedInterface":
                readable = support.run([args.cli, "decompile", fixture, "--method", token, "--no-auto-references"]).stdout
                aliases = re.findall(r"structural signature (__cil_ref_\w+) bytes=\S+: (.*?);", readable)
                for alias, _ in aliases:
                    readable = readable.replace(alias, "global::System.Collections.Generic.IEnumerable<int>")
                il = support.run([args.cli, "il", fixture, "--method", token]).stdout
                if "System.Linq.Enumerable::Count<int32>" not in il:
                    raise ValueError("Unexpected generic extension target")
                statements, count = re.subn(r"__cil_methodspec_call_[0-9A-F]{8}\(([^()]*)\)",
                                           r"global::System.Linq.Enumerable.Count<int>(\1)", readable)
                if count != 1: raise ValueError("Generic extension call missing")
            def dispatch(match):
                target = "0x" + match.group(1)
                if target not in names:
                    raise ValueError("callvirt target is unresolved: " + match.group(0))
                arguments = [part.strip() for part in match.group(2).split(",")]
                # Supply the target-runtime dispatch the recovered body leaves to
                # the runtime: receiver.Name(args) is the same virtual call.
                return f"{arguments[0]}.{names[target]}({', '.join(arguments[1:])})"

            # The recovered body leaves local allocation to the runtime as
            # `Type v = __cil_newobj_TOKEN(args)`; the concrete type is on the
            # declaration, so the same fresh identity is `new Type(args)`.
            def allocate(match):
                indent, declared, variable, arguments = match.groups()
                return f"{indent}{declared} {variable} = new {declared}({arguments});"

            allocated = re.sub(
                r"(?m)^(\s*)([\w.@:<>,]+) (\w+) = __cil_newobj_[0-9A-F]{8}\(([^()]*)\);$",
                allocate,
                statements,
            )
            if "__cil_newobj_" in allocated:
                raise ValueError("nested allocation arguments need an explicit shell: " + name)
            # Ordinary reference targets are bound by exact MemberRef metadata;
            # local MethodDefs keep their established explicit dispatch shell.
            bound = re.sub(r"__cil_callvirt_(06[0-9A-F]{6})\(([^()]*)\)", dispatch, allocated)
            snippets.append(bound)
        aliases, helpers = leaves.plan(support, harness, fixture, OUT, snippets, "VirtualCallCases.Ops")
        carrier = aliases + "namespace VirtualCallCases\n{\npublic static class Recovered\n{\n" + helpers + "\n".join(snippets) + "\n}\n}\n"
        source = OUT / f"Recovered-{config}{'-readable' if args.readable else '-details'}.cs"
        receiver_snippets = []
        for owner, name in [("VirtualCallCases.Shape", "Describe"), ("VirtualCallCases.Value", "ToString")]:
            token = next(token for token in names if owners[token] == owner and names[token] == name)
            details = support.run([args.cli, "decompile", fixture, "--method", token, "--details", "--no-auto-references"]).stdout
            readable = support.run([args.cli, "decompile", fixture, "--method", token, "--no-auto-references"]).stdout
            il = support.run([args.cli, "il", fixture, "--method", token]).stdout
            if "System.Int32::ToString" not in il: raise ValueError("Unexpected scalar receiver leaf")
            body = generic.inline_structural_types(adapter.adapt_readable_body(readable, details))
            receiver = "ref global::VirtualCallCases.Value a0" if owner.endswith(".Value") else "global::VirtualCallCases.Shape a0"
            body = "public static string " + owner.rsplit(".", 1)[1] + "_Describe(" + receiver + ")" + body[body.index("\n{"):]
            body = "\n".join(line if line.lstrip().startswith("//") else re.sub(r"\bthis\b", "a0", line) for line in body.splitlines())
            body = re.sub(r"__cil_memberref_call_[0-9A-F]{8}\(ref (\w+)\)", r"\1.ToString()", body)
            body = re.sub(r'__cil_ldstr_[0-9A-F]{8}\(("(?:shape|value)")\)', r"\1", body)
            body = re.sub(r"__cil_callvirt_([0-9A-F]{8})\(([^()]*)\)", dispatch, body)
            executable = "\n".join(line for line in body.splitlines() if not line.strip().startswith("//"))
            if "__cil_" in support.without_boolean_helpers(executable): raise ValueError("Unbound recovered receiver body")
            receiver_snippets.append(body)
        receiver_source = "\nnamespace VirtualCallCases { public static class RecoveredReceivers {\n" + "\n".join(receiver_snippets) + "\n} }\n"
        carrier += receiver_source
        support.write_changed(source, carrier)
        recovered = source.with_suffix(".dll")
        compile_ms += support.compile_cs(recovered, [source], references=[fixture])
        result = support.run([support.DOTNET / "dotnet", harness, fixture, "--virtual-check", recovered])
        print(config + ": " + result.stdout.strip(), flush=True)
        receiver_result = support.run([support.DOTNET / "dotnet", harness, fixture, "--value-receiver-check", recovered])
        print(config + ": " + receiver_result.stdout.strip(), flush=True)
        rows.append({"configuration": config, "behavior": result.stdout.strip(), "receivers": receiver_result.stdout.strip()})
        if args.mutation_control:
            # A direct call to the declared method loses dispatch; the caller's
            # concrete override must then disagree.
            bad, count = re.subn(r"(\w+)\.Area\(([^)]*)\)", r"((global::VirtualCallCases.Shape)\1).Fixed()", carrier, count=1)
            if count != 1: raise ValueError("virtual-call mutation target missing")
            path = OUT / f"DirectDispatch-{config}.cs"
            support.write_changed(path, bad)
            binary = path.with_suffix(".dll")
            compile_ms += support.compile_cs(binary, [path], references=[fixture])
            negative = subprocess.run([str(support.DOTNET / "dotnet"), str(harness), str(fixture), "--virtual-check", str(binary)],
                                      text=True, capture_output=True, timeout=120)
            (OUT / f"DirectDispatch-mutation-{config}.log").write_text(negative.stdout + negative.stderr)
            if negative.returncode == 0:
                raise ValueError("downgraded dispatch was not detected")
            print(config + ": rejected direct-dispatch mutant", flush=True)
            for label, result_expression in [("Shape", "a0.Describe()"), ("Value", '"value"')]:
                pattern = r"(public static string " + label + r"_Describe\([^\n]+\)\n\{)(.*?)(\n\})"
                target = re.search(pattern, carrier, re.S)
                if target is None: raise ValueError("Receiver mutation target missing")
                body, count = re.subn(r"return [^;]+;", "return " + result_expression + ";", target[2], count=1)
                if count != 1: raise ValueError("Receiver mutation return missing")
                path = OUT / f"Wrong-{label}-{config}.cs"
                support.write_changed(path, carrier[:target.start(2)] + body + carrier[target.end(2):])
                binary = path.with_suffix(".dll")
                compile_ms += support.compile_cs(binary, [path], references=[fixture])
                negative = subprocess.run([str(support.DOTNET / "dotnet"), str(harness), str(fixture),
                    "--value-receiver-check", str(binary)], text=True, capture_output=True, timeout=120)
                (OUT / f"Wrong-{label}-{config}.log").write_text(negative.stdout + negative.stderr)
                if negative.returncode == 0 or "Recovered value receiver differs: " + label not in negative.stderr:
                    raise ValueError("Receiver semantic mutant was not detected: " + negative.stdout + negative.stderr)
                print(config + ": rejected " + label + " receiver mutant", flush=True)
                rows.append({"configuration": config, "mutation": label + "-receiver", "detected": True})
    view = "readable" if args.readable else "details"
    (OUT / f"behavior-results-{view}.json").write_text(json.dumps({"configurations": rows, "compile_ms": compile_ms,
        "view": view, "mutation_control": args.mutation_control, "third_party_execution": False}, indent=2) + "\n")
    print(f"Owned compilation: {compile_ms} ms; Rust compilation: none")

if __name__ == "__main__":
    main()
