#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Negative checks for the class-output acceptance gate; no external input."""
import copy
import unittest

from inventory import check_output


class ClassOutputGate(unittest.TestCase):
    def setUp(self):
        self.snapshot = {"root": "0x02000001", "types": [{
            "token": "0x02000001", "name": "Example", "flags": 1,
            "parent": "0x00000000", "base_type": "0x00000000", "interfaces": [],
            "class_layout": [], "generic_parameters": [], "fields": [], "properties": [], "events": [],
            "methods": [{"token": "0x06000001", "name": "Read", "flags": 22,
                         "impl_flags": 0, "signature": "000008", "has_rva": True,
                         "generic_parameters": [], "parameters": []}]
        }]}
        self.text = ("// type token 0x02000001 Example\n"
                     ".type-flags 0x00000001 class public\n.extends none\n"
                     ".method 0x06000001 flags=0x0016 impl-flags=0x0000 rva=0x00002000 "
                     "public static int32 Example::Read() // signature-bytes=[00, 00, 08]\n"
                     ".method-status 0x06000001 Decompiled\n"
                     "// token 0x06000001 int32 Example::Read()\n"
                     "public static int @Read() { return 17; }\n")
        self.stderr = "decompiled=1 declarations=0 partial=0 failed=0\n"

    def check(self, text=None, snapshot=None, stderr=None, code=0):
        return check_output(snapshot or self.snapshot, text if text is not None else self.text,
                            stderr if stderr is not None else self.stderr, code)

    def test_complete_output_passes(self):
        self.assertEqual(self.check()["decompiled"], 1)

    def test_class_layout_identity_values_and_raw_record_are_required(self):
        snapshot = copy.deepcopy(self.snapshot)
        layout = {"token": "0x0F000001", "parent": "0x02000001", "packing": 4,
                  "size": 8, "raw_row": "0400080000000100"}
        snapshot["types"][0]["class_layout"] = [layout]
        record = (".class-layout 0x0F000001 parent=0x02000001 packing=4 size=8 "
                  "raw-row-bytes=[04, 00, 08, 00, 00, 00, 01, 00]\n")
        text = self.text.replace(".method 0x06000001", record + ".method 0x06000001")
        self.assertEqual(self.check(text=text, snapshot=snapshot)["decompiled"], 1)
        for invalid in [self.text, text.replace("packing=4", "packing=0"),
                        text.replace("size=8", "size=0"), text.replace("parent=0x02000001", "parent=0x02000002"),
                        text.replace("[04, 00", "[00, 00"), text.replace(record, record + record)]:
            with self.assertRaises(ValueError):
                self.check(text=invalid, snapshot=snapshot)

    def test_missing_or_duplicate_method_fails(self):
        header = self.text.split(".method-status", 1)[0].split(".method ", 1)[1]
        for text in [self.text.replace(".method 0x06000001", ".missing 0x06000001"),
                     self.text + ".method " + header]:
            with self.assertRaises(ValueError):
                self.check(text=text)

    def test_summary_and_exit_mismatch_fail(self):
        with self.assertRaises(ValueError):
            self.check(stderr="decompiled=2 declarations=0 partial=0 failed=0\n")
        with self.assertRaises(ValueError):
            self.check(code=2)

    def test_raw_fallback_is_not_decompiled_success(self):
        with self.assertRaises(ValueError):
            self.check(text=self.text + "// IL FALLBACK: unsupported\n")

    def test_field_in_wrong_type_is_not_complete_inventory(self):
        snapshot = copy.deepcopy(self.snapshot)
        snapshot["types"][0]["fields"] = [{"token": "0x04000001", "flags": 6,
                                           "signature": "0608", "constant": None}]
        text = self.text + ("// type token 0x02000002 Sibling\n"
                            ".field 0x04000001 flags=0x0006 int32 @value // signature-bytes=[06, 08]\n")
        with self.assertRaises(ValueError):
            self.check(text=text, snapshot=snapshot)


if __name__ == "__main__":
    unittest.main()
