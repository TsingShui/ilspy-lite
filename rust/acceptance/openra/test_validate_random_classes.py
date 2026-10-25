#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Synthetic public collector CLI checks; no external bodies or .NET required."""
import copy
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve().parent


class FrozenStaticCollector(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        # A private copy also exercises the installed runner's relative imports.
        for name in ("validate_random_classes.py", "inventory.py", "random-classes.json", "manifest.json"):
            shutil.copyfile(HERE / name, self.root / name)
        manifest = json.loads((self.root / "random-classes.json").read_text())
        self.entry = manifest["classes"][0]
        self.data = {}
        for entry in manifest["classes"]:
            ty = {"token": entry["token"], "name": entry["name"], "flags": 1,
                  "parent": "0x00000000", "base_type": "0x00000000", "interfaces": []}
            for key in ("class_layout", "custom_attributes", "security_declarations",
                        "method_implementations", "marshalling", "imports", "generic_parameters",
                        "fields", "properties", "events"):
                ty[key] = []
            ty["methods"] = [{"token": "0x06000001", "name": "Read", "flags": 22,
                "impl_flags": 0, "signature": "000008", "has_rva": True,
                "generic_parameters": [], "parameters": []}]
            il = ("// token 0x06000001 int32 Owned::Read()\n"
                  ".method-flags 0x0016 .impl-flags 0x0000\n"
                  ".maxstack 8\n.initlocals false\n.code-bytes IL_0000: [17, 2A]\n"
                  "IL_0000: ldc.i4.1 \nIL_0001: ret \n")
            details = (f"// type token {entry['token']} {entry['name']}\n"
                ".type-flags 0x00000001 class public\n.extends none\n"
                ".method 0x06000001 flags=0x0016 impl-flags=0x0000 rva=0x00002000 "
                "public static int32 Owned::Read() // signature-bytes=[00, 00, 08]\n"
                ".method-status 0x06000001 Partial\n// IL FALLBACK: owned unsupported\n" +
                "".join("// " + line + "\n" for line in il.splitlines()))
            raw_name = entry["name"].rsplit("/", 1)[-1].rsplit(".", 1)[-1].split("`", 1)[0]
            alias = raw_name if re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", raw_name) else "__name_" + raw_name.encode().hex().upper()
            readable = (f"public class {alias}\n{{\n    public static int Read()\n    {{\n"
                "        // IL fallback: owned unsupported\n" +
                "".join("        // " + line + "\n" for line in il.splitlines()[2:]) + "    }\n}\n")
            self.data[entry["name"]] = {"snapshot": {"root": entry["token"],
                "mvid": entry["mvid"], "types": [ty]}, "il": il,
                "details": details, "readable": readable,
                "summary": "decompiled=0 declarations=0 partial=1 failed=0\n", "code": 2}
        self.save()
        self.cli = self.root / "fake-cli"
        self.cli.write_text(f"#!{sys.executable}\n" + '''import json, pathlib, sys
root = pathlib.Path(__file__).parent
data = json.loads((root / 'data.json').read_text())
args = sys.argv[1:]
if args[0] == 'il':
    row = data[(root / 'active-name.txt').read_text()]
    print(row['il'].replace('0x06000001', args[args.index('--method') + 1]), end='')
    sys.exit(0)
row = data[args[args.index('--type') + 1]]
print(row['details' if '--details' in args else 'readable'], end='')
print(row.get('readable_summary', row['summary']) if '--details' not in args else row['summary'], end='', file=sys.stderr)
sys.exit(row.get('readable_code', row['code']) if '--details' not in args else row['code'])
''')
        self.cli.chmod(0o755)
        self.dotnet = self.root / "fake-dotnet"
        self.dotnet.write_text(f"#!{sys.executable}\n" + '''import json, pathlib, sys
root = pathlib.Path(__file__).parent
data = json.loads((root / 'data.json').read_text())
(root / 'active-name.txt').write_text(sys.argv[-1])
print(json.dumps(data[sys.argv[-1]]['snapshot']))
''')
        self.dotnet.chmod(0o755)
        (self.root / "oracle.dll").touch()
        self.assemblies = self.root / "assemblies"
        self.assemblies.mkdir()
        for source in manifest["population_sources"]:
            (self.assemblies / source["assembly"]).touch()

    def save(self):
        (self.root / "data.json").write_text(json.dumps(self.data))

    def run_collector(self):
        self.save()
        result = subprocess.run([sys.executable, str(self.root / "validate_random_classes.py"),
            "--cli", str(self.cli), "--assemblies", str(self.assemblies),
            "--output", str(self.root / "report"), "--dotnet", str(self.dotnet),
            "--oracle", str(self.root / "oracle.dll")], capture_output=True, text=True, timeout=60)
        report = self.root / "report/report.json"
        self.assertTrue(report.exists(), result.stderr)
        return result, json.loads(report.read_text())

    def test_public_cli_keeps_all_twenty_partial_rows_and_four_real_commands(self):
        result, report = self.run_collector()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(report["static_pass_count"], 20)
        self.assertEqual(len(report["classes"]), 20)
        self.assertTrue(report["noRuntimeProof"])
        row = json.loads((self.root / "report" / report["classes"][0]["path"]).read_text())
        self.assertEqual(len(row["configurations"]), 2)
        for config in row["configurations"].values():
            self.assertEqual(config["counts"]["partial"], 1)
            self.assertEqual(config["readable"]["exit_code"], 2)
            self.assertEqual(config["details"]["exit_code"], 2)
        explicit = row["configurations"]["explicit"]["details"]["command"]
        self.assertIn("--no-auto-references", explicit)
        self.assertIn("--reference", explicit)
        self.assertEqual(row["supported_methods"], [])

    def test_lost_fallback_instruction_fails_but_preserves_all_rows(self):
        row = self.data[self.entry["name"]]
        row["details"] = row["details"].replace("// IL_0001: ret \n", "")
        result, report = self.run_collector()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(len(report["classes"]), 20)
        self.assertEqual(report["static_pass_count"], 19)
        self.assertIn("fallback", json.dumps(report).lower())

    def test_readable_fallback_loss_fails(self):
        row = self.data[self.entry["name"]]
        row["readable"] = row["readable"].replace("// IL_0001: ret \n", "")
        result, report = self.run_collector()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(report["static_pass_count"], 19)

    def test_member_mismatch_fails(self):
        row = self.data[self.entry["name"]]
        row["details"] = row["details"].replace(".method 0x06000001", ".method 0x06000002")
        result, report = self.run_collector()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(report["static_pass_count"], 19)

    def test_invalid_frozen_mvid_fails(self):
        self.data[self.entry["name"]]["snapshot"]["mvid"] = "00000000-0000-0000-0000-000000000000"
        result, report = self.run_collector()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(report["static_pass_count"], 19)

    def test_readable_missing_fallback_marker_fails(self):
        row = self.data[self.entry["name"]]
        row["readable"] = row["readable"].replace("// IL fallback:", "// hidden fallback:")
        result, report = self.run_collector()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(report["static_pass_count"], 19)

    def test_method_name_disagreement_fails(self):
        row = self.data[self.entry["name"]]
        row["details"] = row["details"].replace("Owned::Read()", "Owned::Wrong()", 1)
        result, report = self.run_collector()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(report["static_pass_count"], 19)

    def test_style_summary_disagreement_fails(self):
        row = self.data[self.entry["name"]]
        row["readable_summary"] = "decompiled=1 declarations=0 partial=0 failed=0\n"
        row["readable_code"] = 0
        result, report = self.run_collector()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(report["static_pass_count"], 19)

    def test_supported_method_is_a_candidate_not_runtime_proof(self):
        row = self.data[self.entry["name"]]
        row["details"] = row["details"].split(".method-status", 1)[0] + (
            ".method-status 0x06000001 Decompiled\n"
            "public static int Read() { return 1; }\n")
        header = row["readable"].splitlines()[0]
        row["readable"] = header + "\n{\n    public static int Read()\n    {\n        return 1;\n    }\n}\n"
        row["summary"] = "decompiled=1 declarations=0 partial=0 failed=0\n"
        row["code"] = 0
        result, report = self.run_collector()
        self.assertEqual(result.returncode, 0, result.stderr)
        detail = json.loads((self.root / "report" / report["classes"][0]["path"]).read_text())
        supported = detail["supported_methods"]
        self.assertEqual(len(supported), 1)
        self.assertEqual(supported[0]["token"], "0x06000001")
        self.assertEqual(supported[0]["signature_bytes"], "000008")
        self.assertEqual(supported[0]["potential_behavior_test_applicability"], "candidate_not_validated")
        self.assertTrue(supported[0]["noRuntimeProof"])
        self.assertTrue(supported[0]["reason"])

    def add_owned_declarations(self):
        row = self.data[self.entry["name"]]
        root = row["snapshot"]["types"][0]
        field = {"token": "0x04000001", "name": "state", "flags": 6,
                 "signature": "0608", "constant": None}
        root["fields"] = [field]
        row["details"] = row["details"].replace(".method 0x06000001", (
            ".field 0x04000001 flags=0x0006 public int32 @state // name=state signature-bytes=[06, 08]\n"
            ".method 0x06000001"), 1)
        row["readable"] = row["readable"].replace("{\n", "{\n    public int state;\n", 1)
        for token, name in (("0x020000F1", "Child"), ("0x020000F2", "Sibling")):
            child = copy.deepcopy(root)
            child.update(token=token, name=root["name"] + "/" + name, parent=root["token"], fields=[], methods=[])
            if name == "Child":
                child["fields"] = [{**field, "token": "0x04000002", "name": "<>payload"}]
            row["snapshot"]["types"].append(child)
            row["details"] += (f"// type token {token} {child['name']}\n"
                f".type-flags 0x00000001 class public\n.enclosing-type {root['token']} Owned\n.extends none\n")
            if name == "Child":
                row["details"] += (".field 0x04000002 flags=0x0006 public int32 __il_field_04000002 "
                                   "// name=<>payload signature-bytes=[06, 08]\n")
        row["readable"] = row["readable"].rsplit("}\n", 1)[0] + (
            "    public class Child\n    {\n        public int __il_field_04000002;\n    }\n"
            "    public class Sibling\n    {\n    }\n}\n")
        return row

    def test_readable_catalog_includes_each_owner_and_aliased_field(self):
        self.add_owned_declarations()
        result, report = self.run_collector()
        self.assertEqual(result.returncode, 0, result.stderr + json.dumps(report))
        detail = json.loads((self.root / "report" / report["classes"][0]["path"]).read_text())
        for config in detail["configurations"].values():
            readable = config["readable_inventory"]
            self.assertEqual(len(readable["types"]), 3)
            self.assertEqual(readable["types"][1]["parent"], self.entry["token"])

    def test_default_only_lost_field_fails(self):
        row = self.add_owned_declarations()
        row["readable"] = row["readable"].replace("    public int state;\n", "")
        result, report = self.run_collector()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(report["static_pass_count"], 19)

    def test_default_only_member_moved_to_sibling_fails(self):
        row = self.add_owned_declarations()
        row["readable"] = row["readable"].replace("        public int __il_field_04000002;\n", "")
        row["readable"] = row["readable"].replace("    public class Sibling\n    {\n",
            "    public class Sibling\n    {\n        public int __il_field_04000002;\n")
        result, report = self.run_collector()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(report["static_pass_count"], 19)

    def test_default_only_child_moved_under_sibling_fails(self):
        row = self.add_owned_declarations()
        child = "    public class Child\n    {\n        public int __il_field_04000002;\n    }\n"
        row["readable"] = row["readable"].replace(child, "")
        row["readable"] = row["readable"].replace("    public class Sibling\n    {\n",
            "    public class Sibling\n    {\n" + "\n".join("    " + line for line in child.splitlines()) + "\n")
        result, report = self.run_collector()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(report["static_pass_count"], 19)

    def test_default_only_supported_constructor_removed_fails(self):
        row = self.data[self.entry["name"]]
        method = row["snapshot"]["types"][0]["methods"][0]
        method.update(name=".ctor", flags=6278, signature="200001")
        row["details"] = row["details"].split(".method 0x06000001", 1)[0] + (
            ".method 0x06000001 flags=0x1886 impl-flags=0x0000 rva=0x00002000 "
            "public instance void Owned::.ctor() // signature-bytes=[20, 00, 01]\n"
            ".method-status 0x06000001 Decompiled\n"
            "public static void __cil_ctor_body_06000001() { return; }\n")
        # Header and body are missing only from default output; reported counts stay complete.
        row["readable"] = row["readable"].splitlines()[0] + "\n{\n}\n"
        row["summary"] = "decompiled=1 declarations=0 partial=0 failed=0\n"
        row["code"] = 0
        result, report = self.run_collector()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(report["static_pass_count"], 19)

    def test_lost_original_il_fails(self):
        for row in self.data.values():
            row["il"] = row["il"].replace(".code-bytes IL_0000: [17, 2A]\n", "")
        result, report = self.run_collector()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(report["static_pass_count"], 0)


class ReadableDeclarationInventory(unittest.TestCase):
    def setUp(self):
        self.ty = {"token": "0x02000001", "parent": "0x00000000", "name": "Owned`1",
            "generic_parameters": [{"name": "T"}], "fields": [
                {"token": "0x04000001", "name": "class"},
                {"token": "0x04000002", "name": "<>payload"},
                {"token": "0x04000003", "name": "__il_field_user"}],
            "methods": [
                {"token": "0x06000001", "name": ".ctor", "flags": 6, "signature": "200001",
                 "generic_parameters": [], "has_rva": True},
                {"token": "0x06000002", "name": "Read", "flags": 6, "signature": "300102011E000E",
                 "generic_parameters": [{"name": "U"}], "has_rva": True},
                {"token": "0x06000003", "name": "get_Item", "flags": 6, "signature": "20010808",
                 "generic_parameters": [], "has_rva": False},
                {"token": "0x06000004", "name": "add_event", "flags": 6, "signature": "2001011C",
                 "generic_parameters": [], "has_rva": False}],
            "properties": [{"token": "0x17000001", "name": "Item", "signature": "28010808",
                "getter": "0x06000003", "setter": "0x00000000", "others": []}],
            "events": [{"token": "0x14000001", "name": "event", "adder": "0x06000004",
                "remover": "0x00000000", "raiser": "0x00000000", "others": []}]}
        self.snapshot = {"root": self.ty["token"], "types": [self.ty]}
        self.text = '''public class Owned<T>
{
    public int @class;
    public string __il_field_04000002;
    public object __il_field_04000003;
    public Owned()
    {
        return;
    }
    public void Read<U>(System.Func<int, int> callback, string text = "a,b)")
    {
        return;
    }
    public int get_Item(int index0);
    public void add_event(object value);
    // property int this[int index0] { get: get_Item; }
    // event object @event { add: add_event; }
}
'''

    def check(self, text=None):
        from validate_random_classes import check_readable_inventory
        return check_readable_inventory(self.snapshot, text if text is not None else self.text)

    def test_aliases_parameters_generic_arity_and_relationships_match(self):
        report = self.check()
        self.assertEqual(report["types"][0]["methods"], [m["token"] for m in self.ty["methods"]])
        self.assertEqual(report["types"][0]["observed_methods"][1]["parameters"], 2)
        self.assertEqual(report["types"][0]["observed_methods"][1]["generic_arity"], 1)

    def test_type_and_method_arity_parameter_count_and_body_boundary_are_required(self):
        for before, after in (("Owned<T>", "Owned"), ("Read<U>", "Read<U, V>"),
            ('string text = "a,b)"', 'string text = "a,b)", int extra'),
            ("    public Owned()\n    {\n        return;\n    }", "    public Owned();")):
            with self.subTest(before=before), self.assertRaises(ValueError):
                self.check(self.text.replace(before, after))

    def test_relationship_name_endpoint_or_count_loss_fails(self):
        for before, after in (("get: get_Item;", "get: Read;"), ("@event { add:", "Other { add:"),
            ("// event object @event { add: add_event; }", "// missing event relationship"),
            ("this[int index0]", "this[int index0, int index1]")):
            with self.subTest(before=before), self.assertRaises(ValueError):
                self.check(self.text.replace(before, after))

    def test_explicit_unreadable_token_markers_preserve_identity_without_shape_claim(self):
        text = self.text.replace("public string __il_field_04000002;",
                                "// Unreadable field 0x04000002; see --details.")
        text = text.replace("public void Read<U>(System.Func<int, int> callback, string text = \"a,b)\")",
                            "void __method_06000002()")
        report = self.check(text)
        methods = report["types"][0]["observed_methods"]
        self.assertEqual(methods[1]["token_marker"], "0x06000002")
        with self.assertRaises(ValueError):
            self.check(text.replace("Unreadable field 0x04000002", "Unreadable field 0x04000009"))

    def test_signature_prefix_counts_use_compressed_integers(self):
        from validate_random_classes import signature_counts
        self.assertEqual(signature_counts("20808001"), (128, 0))
        self.assertEqual(signature_counts("308080C000400001"), (16384, 128))
        for blob in ("", "20", "2080", "20E0"):
            with self.subTest(blob=blob), self.assertRaises(ValueError):
                signature_counts(blob)


if __name__ == "__main__":
    unittest.main()
