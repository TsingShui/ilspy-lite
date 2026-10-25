#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Independent static BindingCases metadata comparison; never executes input assemblies."""
import argparse
import copy
import json
import os
from pathlib import Path
import subprocess
import unittest

RUST = Path(__file__).resolve().parents[1]


class ComparisonTests(unittest.TestCase):
    def test_missing_binding_is_rejected(self):
        with self.assertRaisesRegex(AssertionError, "request inventory"):
            check_results([], [{"kind": "type", "token": 0x02000002}], lambda *_: None)

    def test_duplicate_binding_is_rejected(self):
        request = {"kind": "type", "token": 0x02000002}
        with self.assertRaisesRegex(AssertionError, "request inventory"):
            check_results([dict(request), dict(request)], [request], lambda *_: None)

    def test_error_binding_is_rejected(self):
        request = {"kind": "type", "token": 0x02000002}
        with self.assertRaisesRegex(AssertionError, "unexpected binding error"):
            check_results([{**request, "error": "missing"}], [request], lambda *_: None)

    def test_changed_signature_is_rejected(self):
        with self.assertRaisesRegex(AssertionError, "signature"):
            equal("200008", "200002", "definition signature")


def equal(actual, expected, label):
    assert actual == expected, f"{label}: {actual!r} != {expected!r}"


def check_results(results, requests, check):
    keys = lambda rows: [(r["kind"], r["token"]) for r in rows]
    actual, expected = keys(results), keys(requests)
    assert len(actual) == len(set(actual)) and set(actual) == set(expected), "request inventory mismatch"
    for row in results:
        assert "error" not in row, f"unexpected binding error: {row}"
        check(row["kind"], row["token"], row["value"])


def unique(rows, label):
    assert len(rows) == 1, f"expected one {label}, found {len(rows)}"
    return rows[0]


def indexed(rows):
    result = {r["token"]: r for r in rows}
    assert len(rows) == len(result), "duplicate catalog token"
    return result


class CatalogComparison:
    """Fixture-scoped catalog lookup, not a CLR binder or execution oracle."""
    def __init__(self, snapshots):
        self.main = snapshots[0]
        self.by_assembly = {s["identity"]["name"]: s for s in snapshots}
        assert len(self.by_assembly) == len(snapshots), "ambiguous scenario catalog"
        for snapshot in snapshots:
            snapshot["type_rows"] = indexed(snapshot["types"])
            for group in ("type_refs", "type_specs", "member_refs", "method_specs", "assembly_refs"):
                snapshot[group + "_rows"] = indexed(snapshot[group])
            for group in ("methods", "fields", "properties"):
                snapshot[group + "_rows"] = indexed([r for t in snapshot["types"] for r in t[group]])

    def named_type(self, snapshot, name):
        rows = [t for t in snapshot["types"] if t["name"] == name]
        if rows:
            return snapshot, unique(rows, name), []
        ns, _, simple = name.rpartition(".")
        export = unique([r for r in snapshot["exported_types"] if r["ns"] == ns and r["name"] == simple], name)
        assert export["flags"] & 0x00200000, "expected forwarder flag"
        reference = snapshot["assembly_refs_rows"][export["implementation"]]
        target = self.by_assembly[reference["identity"]["name"]]
        equal(reference["identity"], target["identity"], "forwarder assembly identity")
        target, definition, forwarders = self.named_type(target, name)
        return target, definition, [(snapshot, export)] + forwarders

    def expected_type(self, snapshot, token):
        table = token >> 24
        arguments = []
        if table == 0x1b:
            spec = snapshot["type_specs_rows"][token]
            target, row, _, forwarders = self.expected_type(snapshot, spec["definition"])
            arguments = spec["arguments"]
            equal(len(arguments), len(row["generics"]), "TypeSpec generic positions")
            equal([g["index"] for g in row["generics"]], list(range(len(arguments))), "declared generic positions")
            return target, row, arguments, forwarders
        if table == 2:
            return snapshot, snapshot["type_rows"][token], arguments, []
        equal(table, 1, "supported fixture type token")
        reference = snapshot["type_refs_rows"][token]
        scope = reference["scope"]
        if scope >> 24 == 1:
            target, parent, _, forwarders = self.expected_type(snapshot, scope)
            equal(reference["ns"], "", "nested TypeRef namespace")
            target, row, nested = self.named_type(target, parent["name"] + "/" + reference["name"])
            return target, row, [], forwarders + nested
        equal(scope >> 24, 0x23, "TypeRef assembly scope")
        identity = snapshot["assembly_refs_rows"][scope]["identity"]
        target = self.by_assembly[identity["name"]]
        equal(identity, target["identity"], "TypeRef assembly identity")
        name = (reference["ns"] + "." if reference["ns"] else "") + reference["name"]
        target, row, forwarders = self.named_type(target, name)
        return target, row, [], forwarders

    def check_location(self, actual, snapshot, row):
        identity = snapshot["identity"]
        # Owned fixtures are unsigned; do not normalize or authenticate arbitrary keys.
        equal(identity["key"], "", "owned fixture unsigned key")
        equal(actual["assembly"], {"name": identity["name"], "version": identity["version"],
              "culture": identity["culture"], "public_key_token": "", "content_type": identity["flags"] & 0xe00}, "definition assembly")
        equal(actual["source"], snapshot["source"], "definition source")
        equal(actual["token"], row["token"], "definition token")
        equal(actual["kind"], snapshot["kind"], "static assembly classification")

    def check_type(self, actual, expected):
        snapshot, row, arguments, forwarders = expected
        self.check_location(actual["definition"], snapshot, row)
        equal(actual["name"], row["name"], "type name")
        equal(actual["flags"], row["flags"], "type flags")
        equal(actual["exact_identity"], True, "exact metadata identity")
        equal(actual["generic_arguments"], arguments, "TypeSpec generic arguments")
        equal(len(actual["forwarders"]), len(forwarders), "forwarder count")
        for actual_forwarder, (source, export) in zip(actual["forwarders"], forwarders):
            self.check_location(actual_forwarder, source, export)

    def expected_member(self, token, kind):
        snapshot = self.main
        method_spec = None
        if token >> 24 == 0x2b:
            method_spec = snapshot["method_specs_rows"][token]
            token = method_spec["method"]
        if token >> 24 == 0x0a:
            reference = snapshot["member_refs_rows"][token]
            equal(reference["kind"], kind, "MemberRef signature kind")
            owner = self.expected_type(snapshot, reference["owner"])
            target, type_row, _, _ = owner
            # DecodeSignature retains VAR/MVAR positions, including overloads that
            # would otherwise collapse after Generic<int> substitution.
            selected = unique([r for r in type_row[kind + "s"] if r["name"] == reference["name"]
                               and r["shape"] == reference["shape"]], "declared member signature")
        else:
            reference = snapshot[kind + "s_rows"][token]
            owner = self.expected_type(snapshot, reference["owner"])
            target, selected = snapshot, reference
        return target, selected, reference, owner, method_spec

    def check_member(self, actual, token, kind):
        target, selected, reference, owner, method_spec = self.expected_member(token, kind)
        self.check_location(actual["definition"], target, selected)
        self.check_type(actual["owner"], owner)
        for key in ("name", "flags"):
            equal(actual[key], selected[key], "member " + key)
        equal(actual["reference_signature"], reference["signature"], "raw reference signature")
        equal(actual["definition_signature"], selected["signature"], "raw definition signature")
        assert actual["signature_identity"], "missing scoped signature identity"
        if kind == "method":
            equal(actual["method_arguments"], method_spec["arguments"] if method_spec else [], "MethodSpec generic arguments")
            equal(actual["instantiation_signature"], method_spec["signature"] if method_spec else "", "raw MethodSpec signature")

    def check_property(self, actual, token):
        target, method, _, owner, _ = self.expected_member(token, "method")
        rows = [r for r in target["method_semantics"] if r["method"] == method["token"] and r["association"] >> 24 == 0x17]
        if not rows:
            equal(actual, None, "non-property method has no MethodSemantics property relation")
            return
        semantics = unique(rows, "MethodSemantics property accessor")
        assert semantics["flags"] in (1, 2), "expected getter/setter semantics"
        role, key = ("get", "getter") if semantics["flags"] == 2 else ("set", "setter")
        property_row = target["properties_rows"][semantics["association"]]
        equal(property_row[key], method["token"], "independent GetAccessors endpoint")
        equal(property_row["owner"], owner[1]["token"], "property declaring type")
        assert actual is not None, "missing property relation"
        self.check_location(actual["definition"], target, property_row)
        self.check_type(actual["owner"], owner)
        equal(actual["name"], property_row["name"], "property name")
        equal(actual["role"], role, "MethodSemantics role")
        equal(actual["signature"], property_row["signature"], "raw property signature")
        self.check_member(actual["method"], token, "method")

    def check_relation(self, actual, snapshot, token):
        equal(actual["reference_token"], token, "raw type relation token")
        try:
            expected = self.expected_type(snapshot, token)
        except KeyError as error:
            # Only absent framework assemblies are expected in this bounded gate.
            assert str(error.args[0]).startswith("System."), f"unexpected missing relation: {error}"
            equal(actual["resolved"], None, "unavailable framework relation")
            assert actual["error"], "missing unresolved relation diagnostic"
        else:
            equal(actual["error"], None, "resolved relation diagnostic")
            self.check_type(actual["resolved"], expected)

    def check(self, kind, token, actual):
        if kind == "type":
            self.check_type(actual, self.expected_type(self.main, token))
        elif kind in ("method", "field"):
            self.check_member(actual, token, kind)
        elif kind == "property":
            self.check_property(actual, token)
        elif kind == "relations":
            expected = self.expected_type(self.main, token)
            self.check_type(actual["owner"], expected)
            snapshot, owner, _, _ = expected
            if owner["base_type"]:
                assert actual["base_type"] is not None, "missing base relation"
                self.check_relation(actual["base_type"], snapshot, owner["base_type"])
            else:
                equal(actual["base_type"], None, "no base relation")
            equal(len(actual["interfaces"]), len(owner["interfaces"]), "interface relation count")
            for relation, reference in zip(actual["interfaces"], owner["interfaces"]):
                self.check_relation(relation, snapshot, reference)
        else:
            raise AssertionError(f"unknown check kind {kind}")


def requests_for(comparison, forward_only=False):
    main = comparison.main
    if forward_only:
        types = [r["token"] for r in main["type_refs"] if r["name"] in ("Peer", "Outer`1", "Inner`1")]
        equal(len(types), 3, "required forwarded type coverage")
        types += [r["token"] for r in main["type_specs"] if len(r["arguments"]) == 2]
        equal(len(types), 4, "required forwarded nested TypeSpec coverage")
        return [{"kind": "type", "token": t} for t in types]
    types = [t["token"] for t in main["types"] if t["name"] != "<Module>"]
    # Framework TypeRefs and attribute constructors are cataloged, but not bound:
    # this gate supplies only owned BindingCases metadata, not framework images.
    types += [r["token"] for r in main["type_refs"] if r["full_name"].startswith("BindingLibrary::")]
    types += [r["token"] for r in main["type_specs"]]
    requests = [{"kind": kind, "token": t} for t in types for kind in ("type", "relations")]
    for kind in ("method", "field"):
        tokens = list(main[kind + "s_rows"])
        tokens += [r["token"] for r in main["member_refs"] if r["kind"] == kind
                   and (r["owner"] >> 24 == 0x1b or r["owner"] in types)]
        if kind == "method":
            tokens += [r["token"] for r in main["method_specs"]]
        requests += [{"kind": kind, "token": t} for t in tokens]
        if kind == "method":
            requests += [{"kind": "property", "token": t} for t in tokens]
    assert requests and len(requests) == len({(r["kind"], r["token"]) for r in requests}), "empty or duplicate requests"
    return requests


def run_logged(command, log, timeout=120):
    result = subprocess.run([str(x) for x in command], text=True, capture_output=True, timeout=timeout)
    log.write_text(result.stdout + result.stderr)
    assert result.returncode == 0, f"command failed ({result.returncode}): {command}; see {log}"
    return result.stdout


def compile_oracle(output):
    dotnet = Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10"))
    def latest(path):
        return max((p for p in path.iterdir() if p.name.startswith("10.")),
                   key=lambda p: tuple(map(int, p.name.split("."))))
    sdk = latest(dotnet / "sdk")
    ref = latest(dotnet / "packs/Microsoft.NETCore.App.Ref") / "ref/net10.0"
    runtime = latest(dotnet / "shared/Microsoft.NETCore.App")
    oracle = output / "BindingInventory.dll"
    response = oracle.with_suffix(".rsp")
    response.write_text("\n".join([
        "-nologo", "-target:exe", "-langversion:14", "-warnaserror+", "-deterministic+",
        f'-out:"{oracle}"', *[f'-r:"{p}"' for p in sorted(ref.glob("*.dll"))],
        f'"{RUST / "tests/dotnet/BindingInventory.cs"}"',
    ]))
    oracle.with_suffix(".runtimeconfig.json").write_text(json.dumps({"runtimeOptions": {
        "tfm": "net10.0", "framework": {"name": "Microsoft.NETCore.App", "version": runtime.name}}}))
    run_logged([dotnet / "dotnet", sdk / "Roslyn/bincore/csc.dll", f"@{response}"], output / "oracle-build.log")
    return [dotnet / "dotnet", oracle]


def patch_forward_scope(path, catalog, output):
    """Redirect AssemblyRef.Name using a report-only repurposed method-name slot."""
    reference = unique([r for r in catalog["assembly_refs"] if r["identity"]["name"] == "BindingLibrary"], "BindingLibrary reference")
    slot = unique([m for t in catalog["types"] for m in t["methods"] if m["name"] == "get_VirtualFlag"], "type-only scenario name slot")
    replacement = b"BindingFacade"
    assert len(replacement) <= len(slot["name"]), "forwarder name slot too short"
    data = bytearray(path.read_bytes())
    offset = slot["name_offset"]
    equal(data[offset:offset + len(slot["name"])].decode("ascii"), slot["name"], "independent string file offset")
    data[offset:offset + len(slot["name"])] = replacement.ljust(len(slot["name"]), b"\0")
    offset = reference["name_index_offset"]
    data[offset:offset + reference["name_width"]] = slot["name_index"].to_bytes(reference["name_width"], "little")
    output.write_bytes(data)


def negative_controls(results, requests, comparison, output, label):
    checks = []
    for name in ("selected-token", "definition-signature", "property-relation"):
        changed = copy.deepcopy(results)
        if name == "selected-token":
            selected = unique([r for r in changed if r["kind"] == "method" and r["value"]["name"] == "get_Enabled"], "negative-control getter")
            selected["value"]["definition"]["token"] ^= 1
        elif name == "definition-signature":
            selected = unique([r for r in changed if r["kind"] == "method" and r["value"]["name"] == "get_Enabled"], "negative-control getter")
            selected["value"]["definition_signature"] += "00"
        else:
            selected = unique([r for r in changed if r["kind"] == "property" and r["value"] is not None
                               and r["value"]["name"] == "Enabled" and r["value"]["role"] == "get"], "negative-control property")
            selected["value"] = None
        artifact = output / f"{label}.negative-{name}.json"
        artifact.write_text(json.dumps(changed, indent=2) + "\n")
        try:
            check_results(changed, requests, comparison.check)
        except AssertionError as error:
            checks.append({"mutation": name, "rejected": True, "reason": str(error)})
        else:
            raise AssertionError(f"negative control accepted: {name}")
    return checks


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=RUST / "report/dependency-oracle")
    parser.add_argument("--exporter", type=Path, help="use an already built public API exporter")
    parser.add_argument("--self-test", action="store_true", help="run comparator assertion tests only")
    args = parser.parse_args()
    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(ComparisonTests)
        assert unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful(), "comparator tests failed"
        return
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    summary = output / "summary.json"
    summary.unlink(missing_ok=True)
    oracle = compile_oracle(output)
    exporter = args.exporter
    if exporter is None:
        run_logged(["cargo", "build", "--manifest-path", RUST / "Cargo.toml", "--offline", "--locked",
                    "--example", "binding_inventory"], output / "exporter-build.log")
        exporter = RUST / "target/debug/examples/binding_inventory"
    exporter = exporter.resolve()
    def catalog(path, label, source, kind):
        text = run_logged([*oracle, path], output / f"{label}.catalog.json")
        result = json.loads(text)
        markers = result["reference_markers"]
        equal(len(markers), 1 if kind == "ReferenceContract" else 0, "independent reference-contract marker count")
        for marker in markers:
            equal((marker["name"], marker["signature"], marker["value"]),
                  (".ctor", "200001", "01000000"), "independent reference-contract marker bytes")
        result.update(source=source, kind=kind)
        return result
    report = []
    for config in ("debug", "release"):
        fixtures = RUST / "tests/fixtures"
        paths = {name: fixtures / f"binding{suffix}-{config}.bin" for name, suffix in
                 (("main", ""), ("library", "-library"), ("facade", "-facade"), ("contract", "-contract"))}
        catalogs = {name: catalog(path, f"{config}-{name}", "<main>" if name == "main" else str(path),
                                  "ReferenceContract" if name == "contract" else "ImplementationImage")
                    for name, path in paths.items()}
        equal(len(catalogs["main"]["method_specs"]), 1, "required MethodSpec coverage")
        required = {"get_Enabled", "set_Enabled", "get_Pretender", "get_Item", "Echo", "Right", "Read", "Convert"}
        assert required <= {r["name"] for r in catalogs["main"]["member_refs"]}, "required main MemberRefs missing"
        generic = unique([t for t in catalogs["library"]["types"] if t["name"] == "BindingLibrary.Generic`1"], "generic fixture")
        equal(len([m for m in generic["methods"] if m["name"] == "Echo"]), 2, "declared generic overload coverage")
        forwarded = output / f"{config}-forward-main.bin"
        patch_forward_scope(paths["main"], catalogs["main"], forwarded)
        forward_catalog = catalog(forwarded, f"{config}-forward-main", "<main>", "ImplementationImage")
        # A library-as-main scenario inventories both concrete and VAR overloads,
        # all local fields/properties, and MethodDef rather than MemberRef binding.
        library_main = copy.deepcopy(catalogs["library"])
        library_main["source"] = "<main>"
        scenarios = [
            ("implementation", paths["main"], [paths["library"]], [catalogs["main"], catalogs["library"]], False),
            ("contract", paths["main"], [paths["contract"]], [catalogs["main"], catalogs["contract"]], False),
            ("library-local", paths["library"], [], [library_main], False),
            ("forwarder", forwarded, [paths["facade"], paths["library"]],
             [forward_catalog, catalogs["facade"], catalogs["library"]], True),
        ]
        for name, main_path, references, snapshots, forward_only in scenarios:
            label = f"{config}-{name}"
            comparison = CatalogComparison(snapshots)
            requests = requests_for(comparison, forward_only)
            request_file = output / f"{label}.requests.json"
            request_file.write_text(json.dumps(requests, indent=2) + "\n")
            results = json.loads(run_logged([exporter, main_path, request_file, *references], output / f"{label}.actual.json"))
            check_results(results, requests, comparison.check)
            negatives = [] if forward_only else negative_controls(results, requests, comparison, output, label)
            counts = {k: sum(r["kind"] == k for r in requests) for k in ("type", "method", "field", "property", "relations")}
            report.append({"scenario": label, "checks": len(requests), "counts": counts, "negative_controls": negatives})
            print(f"{label}: {len(requests)} static binding comparisons passed; {len(negatives)} mutations rejected")
    summary.write_text(json.dumps({"scope": "static metadata only; no runtime/execution proof",
                                   "scenarios": report}, indent=2) + "\n")


if __name__ == "__main__":
    main()
