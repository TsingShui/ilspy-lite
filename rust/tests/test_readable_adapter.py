#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Test the optional behavior adapter's variable-only rebinding, no .NET needed."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("roundtrip", Path(__file__).with_name("reference-roundtrip.py"))
roundtrip = importlib.util.module_from_spec(spec)
spec.loader.exec_module(roundtrip)


class ReadableAdapter(unittest.TestCase):
    def test_member_identifiers_are_not_rebound_as_parameter_variables(self):
        readable = 'public void Set(int Value)\n{\n    this.Value = Value;\n    string text = "this Value";\n}\n'
        details = '// field token 0x04000001\npublic static void @Set(global::Cases.Cell a0, int a1)\n{\n}\n'
        adapted = roundtrip.adapt_readable_body(readable, details)
        self.assertIn('a0.Value = a1;', adapted)
        self.assertIn('"this Value"', adapted)
        self.assertNotIn('a0.a1', adapted)

    def test_ambiguous_parameter_bindings_fail_instead_of_merging_values(self):
        details = 'public static int @Difference(int a0, int a1)\n{\n}\n'
        readable = 'public static int Difference(int value, int value)\n{\n    return value - value;\n}\n'
        with self.assertRaises(ValueError):
            roundtrip.adapt_readable_body(readable, details)


if __name__ == "__main__":
    unittest.main()
