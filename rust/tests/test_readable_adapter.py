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

    def test_dependency_reports_stay_outside_executed_method_statements(self):
        readable = ('public int Read()\n{\n    return this.Value;\n}\n'
                    '\n// Dependency unresolved: Missing; reference identity retained.\n')
        details = ('// field token 0x04000001\npublic static int @Read(global::Owned.Cell a0)\n{\n'
                   '    return a0.Value;\n}\n\n.dependency 0x23000001 status=missing\n')
        adapted = roundtrip.adapt_readable_body(readable, details)
        self.assertIn('return a0.Value;', adapted)
        self.assertNotIn('Dependency unresolved', adapted)
        self.assertNotIn('.dependency ', adapted)
        self.assertTrue(roundtrip.detailed_method(details).endswith('}\n'))
        self.assertTrue(roundtrip.readable_method(readable).endswith('}\n'))

    def test_constructor_initializer_uses_its_arguments_not_detailed_body_logic(self):
        readable = 'public Child(int value) : base(value)\n{\n    this.Value = value;\n}\n'
        details = ('// token 0x06000001 instance void Owned.Child::.ctor(int32)\n'
                   '// call token 0x0A000001: [Dependency]Owned.Base::.ctor\n'
                   '// field token 0x04000001\n'
                   'public static void __cil_ctor_body_06000001(global::Owned.Child a0, int a1)\n{\n'
                   '    throw new System.Exception("must not execute detail statements");\n}\n')
        adapted = roundtrip.adapt_readable_body(readable, details)
        self.assertIn('__cil_ctor_call_0A000001(a0, a1);', adapted)
        self.assertLess(adapted.index('__cil_ctor_call_'), adapted.index('a0.Value = a1;'))
        self.assertNotIn('must not execute', adapted)
        ambiguous = details.replace('// field token', '// call token 0x0A000002: [Dependency]Owned.Base::.ctor\n// field token')
        with self.assertRaises(ValueError):
            roundtrip.adapt_readable_body(readable, ambiguous)

    def test_base_property_leaves_compile_the_actual_source_access_without_borrowed_logic(self):
        readable = ('public bool Read(bool value)\n{\n    base.Enabled = !value;\n'
                    '    string text = "base.Enabled";\n    return base.Enabled;\n}\n')
        details = ('// call token 0x0A000001: [Library]Owned.Base::get_Enabled\n'
                   '// call token 0x0A000002: [Library]Owned.Base::set_Enabled\n'
                   'public static bool @Read(global::Owned.Child a0, bool a1)\n{\n'
                   '    throw new System.Exception("do not borrow these statements");\n}\n')
        adapted = roundtrip.adapt_readable_body(readable, details)
        self.assertIn('__cil_memberref_call_0A000002(a0, !a1);', adapted)
        self.assertIn('return __cil_memberref_call_0A000001(a0);', adapted)
        self.assertIn('// source property token 0x0A000001: get base.Enabled', adapted)
        self.assertIn('// source property token 0x0A000002: set base.Enabled', adapted)
        self.assertIn('"base.Enabled"', adapted)
        self.assertNotIn('do not borrow', adapted)
        with self.assertRaises(ValueError):
            roundtrip.adapt_readable_body(readable, details.replace('::get_Enabled', '::get_Other'))

    def test_this_property_uses_a_compiled_leaf_whose_accessor_is_recovered(self):
        readable = 'public bool Read()\n{\n    return this.Flag;\n}\n'
        details = ('// call token 0x06000002: Owned.Cell::get_Flag\n'
                   'public static bool @Read(global::Owned.Cell a0)\n{\n'
                   '    throw new System.Exception("not this body");\n}\n')
        adapted = roundtrip.adapt_readable_body(readable, details)
        self.assertIn('__cil_source_this_property_06000002(a0)', adapted)
        self.assertIn('// source property token 0x06000002: get this.Flag', adapted)
        self.assertNotIn('not this body', adapted)

    def test_ambiguous_parameter_bindings_fail_instead_of_merging_values(self):
        details = 'public static int @Difference(int a0, int a1)\n{\n}\n'
        readable = 'public static int Difference(int value, int value)\n{\n    return value - value;\n}\n'
        with self.assertRaises(ValueError):
            roundtrip.adapt_readable_body(readable, details)


if __name__ == "__main__":
    unittest.main()
