#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Public sampler CLI checks; synthetic metadata only, no .NET needed."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("sample_classes.py")


class RandomClassSelection(unittest.TestCase):
    def run_sampler(self, directory, output, extra=()):
        return subprocess.run([sys.executable, str(SCRIPT),
            "--input", f"owned.dll={directory / 'types.json'}", "--count", "2",
            "--seed", "000102030405060708090a0b0c0d0e0f", "--exclude", "Fixed",
            "--baseline-ref", "owned-baseline", "--output", str(output), *extra],
            text=True, capture_output=True, timeout=30)

    def test_same_seed_freezes_distinct_classes_without_support_filters(self):
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp)
            population = {"mvid": "owned-module", "types": [
                {"token": f"0x0200000{i}", "name": name, "kind": kind}
                for i, (name, kind) in enumerate([
                    ("Empty", "class"), ("Generic`1/Nested", "class"),
                    ("Fixed", "class"), ("IMarker", "interface"),
                    ("<Module>", "module"), ("Value", "value_type")], 1)]}
            (directory / "types.json").write_text(json.dumps(population))
            outputs = [directory / name for name in ("first.json", "second.json")]
            for output in outputs:
                result = self.run_sampler(directory, output)
                self.assertEqual(result.returncode, 0, result.stderr)
            first, second = [json.loads(p.read_text()) for p in outputs]
            self.assertEqual(first, second)
            self.assertEqual(first["eligible_count"], 2)
            self.assertEqual({c["name"] for c in first["classes"]}, {"Empty", "Generic`1/Nested"})
            self.assertEqual(len({c["token"] for c in first["classes"]}), 2)
            self.assertEqual(first["seed"], "000102030405060708090a0b0c0d0e0f")
            self.assertEqual(first["baseline_ref"], "owned-baseline")
            before = outputs[0].read_bytes()
            self.assertNotEqual(self.run_sampler(directory, outputs[0]).returncode, 0)
            self.assertEqual(outputs[0].read_bytes(), before)

    def test_duplicate_metadata_identity_is_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp)
            entry = {"token": "0x02000001", "name": "Class", "kind": "class"}
            (directory / "types.json").write_text(json.dumps({"mvid": "module", "types": [entry, entry]}))
            result = self.run_sampler(directory, directory / "sample.json")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("duplicate", result.stderr.lower())
            self.assertFalse((directory / "sample.json").exists())


if __name__ == "__main__":
    unittest.main()
