#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Synthetic public collector CLI checks; no external bodies or .NET required."""
import json
from pathlib import Path
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
            readable = ("public class Owned {\n public static int Read() {\n"
                "  // IL fallback: owned unsupported\n" +
                "".join("  // " + line + "\n" for line in il.splitlines()[2:]) + " }\n}\n")
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
    print(next(iter(data.values()))['il'], end='')
    sys.exit(0)
row = data[args[args.index('--type') + 1]]
print(row['details' if '--details' in args else 'readable'], end='')
print(row.get('readable_summary', row['summary']) if '--details' not in args else row['summary'], end='', file=sys.stderr)
sys.exit(row.get('readable_code', row['code']) if '--details' not in args else row['code'])
''')
        self.cli.chmod(0o755)
        self.dotnet = self.root / "fake-dotnet"
        self.dotnet.write_text(f"#!{sys.executable}\n" + '''import json, pathlib, sys
data = json.loads((pathlib.Path(__file__).parent / 'data.json').read_text())
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
        row["readable"] = "public class Owned { public static int Read() { return 1; } }\n"
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

    def test_lost_original_il_fails(self):
        for row in self.data.values():
            row["il"] = row["il"].replace(".code-bytes IL_0000: [17, 2A]\n", "")
        result, report = self.run_collector()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(report["static_pass_count"], 0)


if __name__ == "__main__":
    unittest.main()
