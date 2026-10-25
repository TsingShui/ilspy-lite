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
            "class_layout": [], "custom_attributes": [], "security_declarations": [], "method_implementations": [], "marshalling": [], "imports": [], "generic_parameters": [], "fields": [], "properties": [], "events": [],
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

    def test_custom_attribute_parent_payload_constructor_and_limit_are_required(self):
        snapshot = copy.deepcopy(self.snapshot)
        snapshot["types"][0]["custom_attributes"] = [{"token": "0x0C000001", "parent": "0x02000001",
            "constructor": "0x0A000001", "value_blob_index": 7, "value": "01000000",
            "constructor_signature": "200001", "constructor_name": ".ctor", "constructor_owner": "0x01000001",
            "raw_row": "23000B000700"}]
        record = (".custom-attribute 0x0C000001 parent=0x02000001 constructor=0x0A000001 "
                  "value-blob-index=0x00000007 value-bytes=[01, 00, 00, 00] "
                  "raw-row-bytes=[23, 00, 0B, 00, 07, 00] payload=uninterpreted\n"
                  ".attribute-constructor 0x0A000001 Attribute::.ctor owner=0x01000001 Attribute signature-bytes=[20, 00, 01]\n")
        text = self.text.replace(".method 0x06000001", record + ".method 0x06000001")
        self.assertEqual(self.check(text=text, snapshot=snapshot)["decompiled"], 1)
        for invalid in [self.text, text.replace("parent=0x02000001", "parent=0x02000002"),
                        text.replace("constructor=0x0A000001", "constructor=0x0A000002"),
                        text.replace("value-bytes=[01", "value-bytes=[00"),
                        text.replace("payload=uninterpreted", "payload=decoded"),
                        text.replace("signature-bytes=[20, 00, 01]", "signature-bytes=[20, 01, 01]"),
                        text.replace("Attribute::.ctor", "Unrelated::NotAConstructor"),
                        text.replace("owner=0x01000001", "owner=0x02000000")]:
            with self.assertRaises(ValueError):
                self.check(text=invalid, snapshot=snapshot)

    def test_security_scope_action_payload_and_limitation_are_required(self):
        snapshot = copy.deepcopy(self.snapshot)
        snapshot["types"][0]["security_declarations"] = [{"token": "0x0E000001", "parent": "0x02000001",
            "action": 2, "permission_set_blob_index": 9, "permission_set": "2E00", "raw_row": "020004000900"}]
        record = (".decl-security 0x0E000001 parent=0x02000001 action=0x0002 permission-set-blob-index=0x00000009 "
                  "permission-set-bytes=[2E, 00] raw-row-bytes=[02, 00, 04, 00, 09, 00] "
                  "payload=uninterpreted; security enforcement is runtime-owned\n")
        text = self.text.replace(".method 0x06000001", record + ".method 0x06000001")
        self.assertEqual(self.check(text=text, snapshot=snapshot)["decompiled"], 1)
        for invalid in [self.text, text.replace("parent=0x02000001", "parent=0x02000002"),
                        text.replace("action=0x0002", "action=0x0003"), text.replace("[2E, 00]", "[2E, 01]"),
                        text.replace("payload=uninterpreted", "payload=decoded"),
                        text.replace("security enforcement is runtime-owned", "security ignored")]:
            with self.assertRaises(ValueError):
                self.check(text=invalid, snapshot=snapshot)

    def test_method_mapping_endpoints_signatures_and_limit_are_required(self):
        snapshot = copy.deepcopy(self.snapshot)
        snapshot["types"][0]["method_implementations"] = [{"token": "0x19000001", "class": "0x02000001",
            "body": "0x06000001", "declaration": "0x0A000001", "body_signature": "200008",
            "declaration_signature": "200008", "body_name": "Read", "body_owner": "0x02000001",
            "declaration_name": "Read", "declaration_owner": "0x01000001", "raw_row": "010002000300"}]
        record = (".method-impl 0x19000001 class=0x02000001 body=0x06000001 declaration=0x0A000001 "
                  "raw-row-bytes=[01, 00, 02, 00, 03, 00]; dispatch/access/ancestry validation is not performed\n"
                  ".method-impl-body 0x06000001 Type::Read owner=0x02000001 signature-bytes=[20, 00, 08]\n"
                  ".method-impl-declaration 0x0A000001 Interface::Read owner=0x01000001 signature-bytes=[20, 00, 08]\n")
        text = self.text.replace(".method 0x06000001", record + ".method 0x06000001")
        self.assertEqual(self.check(text=text, snapshot=snapshot)["decompiled"], 1)
        for invalid in [self.text, text.replace("class=0x02000001", "class=0x02000002"),
                        text.replace("declaration=0x0A000001", "declaration=0x0A000002"),
                        text.replace("body=0x06000001", "body=0x06000002"),
                        text.replace("signature-bytes=[20, 00, 08]", "signature-bytes=[20, 01, 08]"),
                        text.replace("dispatch/access/ancestry validation is not performed", "override resolved"),
                        text.replace("Type::Read", "Unrelated::Different"),
                        text.replace("Interface::Read", "Unrelated::Different"),
                        text.replace("owner=0x02000001", "owner=0x02000000"),
                        text.replace("owner=0x01000001", "owner=0x02000000")]:
            with self.assertRaises(ValueError):
                self.check(text=invalid, snapshot=snapshot)

    def test_parameter_marshalling_parent_blob_row_and_runtime_limit_are_required(self):
        snapshot = copy.deepcopy(self.snapshot)
        snapshot["types"][0]["marshalling"] = [{"token": "0x0D000001", "parent": "0x08000001",
            "native_type_blob_index": 7, "native_type": "02", "raw_row": "03000700"}]
        record = (".field-marshal 0x0D000001 parent=0x08000001 native-type-blob-index=0x00000007 "
                  "bytes=[02] raw-row-bytes=[03, 00, 07, 00] payload=uninterpreted; interop marshaling is runtime-owned\n")
        text = self.text.replace(".method 0x06000001", record + ".method 0x06000001")
        self.assertEqual(self.check(text=text, snapshot=snapshot)["decompiled"], 1)
        for invalid in [self.text, text.replace("parent=0x08000001", "parent=0x08000002"),
                        text.replace("bytes=[02]", "bytes=[03]"), text.replace("[03, 00, 07, 00]", "[00, 00]"),
                        text.replace("payload=uninterpreted", "payload=decoded"),
                        text.replace("interop marshaling is runtime-owned", "marshaling ignored")]:
            with self.assertRaises(ValueError):
                self.check(text=invalid, snapshot=snapshot)

    def test_import_entry_module_identity_and_raw_records_are_required(self):
        snapshot = copy.deepcopy(self.snapshot)
        snapshot["types"][0]["imports"] = [{"token": "0x1C000001", "method": "0x06000001", "flags": 256,
            "name": "entry", "scope": "0x1A000001", "module": "native", "module_name_index": 3,
            "module_raw_row": "0300", "raw_row": "0001030001000100"}]
        record = (".pinvoke 0x1C000001 method=0x06000001 flags=0x0100 scope=0x1A000001 name=entry module=native "
                  "module-raw-row-bytes=[03, 00] module-name-index=0x00000003 raw-row-bytes=[00, 01, 03, 00, 01, 00, 01, 00]; "
                  "external native binding and interop execution are runtime-owned\n")
        text = self.text.replace(".method 0x06000001", record + ".method 0x06000001")
        self.assertEqual(self.check(text=text, snapshot=snapshot)["decompiled"], 1)
        for invalid in [self.text, text.replace("name=entry", "name=wrong"), text.replace("module=native", "module=wrong"),
                        text.replace("scope=0x1A000001", "scope=0x1A000002"), text.replace("module-raw-row-bytes=[03", "module-raw-row-bytes=[00"),
                        text.replace("external native binding and interop execution are runtime-owned", "native ignored")]:
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
