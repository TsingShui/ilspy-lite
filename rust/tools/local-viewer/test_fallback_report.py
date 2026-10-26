#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Unit checks for the static fallback aggregation used by the local viewer."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("fallback_report", Path(__file__).with_name("fallback_report.py"))
fallback_report = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fallback_report)

DETAILS = """// Class-level C#-like analysis; not a compilable type declaration.
// type token 0x02000002 Example.Root
// type token 0x02000003 Example.Root/<>c
.method-status 0x06000001 Decompiled
// token 0x06000001 void Example.Root::Run()
public static void __cil_body_1() { }
.method-status 0x06000002 Partial
// token 0x06000002 void Example.Root::Stop()
// IL FALLBACK: unsupported opcode callvirt at IL_000F
// // token 0x06000002 void Example.Root::Stop()
.method-status 0x06000003 Partial
// token 0x06000003 int Example.Root::Read()
// IL FALLBACK: unsupported signature type needs IL fallback
.method-status 0x06000004 Declaration
"""

LIST = """0x06000001 instance void Example.Root::Run()
0x06000002 instance void Example.Root::Stop()
0x06000003 instance int32 Example.Root/<>c::<Run>b__0(int32)
0x06000004 instance void Example.Other::Nested()
0x06000005 static void <Module>::Global()
0x06000006 instance void Other.Deep.Nested::Reset()
"""


class ParseDetails(unittest.TestCase):
    def test_statuses_and_normalized_reasons_are_collected(self):
        parsed = fallback_report.parse_details(DETAILS)
        self.assertEqual(parsed["token"], "0x02000002")
        self.assertEqual([(m["token"], m["status"]) for m in parsed["methods"]], [
            ("0x06000001", "Decompiled"), ("0x06000002", "Partial"),
            ("0x06000003", "Partial"), ("0x06000004", "Declaration")])
        self.assertEqual(parsed["methods"][1]["reason"], "unsupported opcode callvirt")

    def test_first_type_token_is_the_selected_root(self):
        self.assertEqual(fallback_report.parse_details(DETAILS)["token"], "0x02000002")

    def test_failed_body_keeps_a_failure_reason(self):
        text = ("// type token 0x02000002 Example.Root\n"
                ".method-status 0x06000001 Failed\n"
                "// FAILED token 0x06000001: body unreadable\n")
        parsed = fallback_report.parse_details(text)
        self.assertEqual(parsed["methods"][0]["status"], "Failed")
        self.assertIn("body unreadable", parsed["methods"][0]["reason"])


class TypesFromList(unittest.TestCase):
    def test_nested_owners_collapse_to_the_top_level_type(self):
        self.assertEqual(fallback_report.type_names_from_list(LIST), [
            "Example.Other", "Example.Root", "Other.Deep.Nested"])

    def test_module_pseudo_type_is_skipped(self):
        self.assertNotIn("<Module>", fallback_report.type_names_from_list(LIST))


class Collect(unittest.TestCase):
    def runner_factory(self, outputs):
        commands = []

        def runner(command, timeout):
            command = [str(value) for value in command]
            commands.append(command)
            for needle, output in outputs.items():
                if needle in command:
                    return {"text": output, "stderr": "", "exitCode": 0, "elapsedMs": 1}
            raise AssertionError("unexpected command: " + " ".join(command))

        return commands, runner

    def test_totals_reasons_and_type_rows_are_aggregated(self):
        outputs = {
            "--type": DETAILS,
        }
        commands, runner = self.runner_factory(outputs)
        report = fallback_report.collect("/tmp/lite", "Example.dll",
                                         type_names=["Example.Root"], runner=runner)
        self.assertEqual(report["summary"]["bodies"], 3)
        self.assertEqual(report["summary"]["decompiled"], 1)
        self.assertEqual(report["summary"]["partial"], 2)
        self.assertEqual(report["summary"]["declarations"], 1)
        self.assertEqual(report["summary"]["failed"], 0)
        self.assertAlmostEqual(report["summary"]["rate"], 2 / 3)
        self.assertEqual(report["reasons"], [
            {"reason": "unsupported opcode callvirt", "count": 1, "share": 0.5},
            {"reason": "unsupported signature type needs IL fallback", "count": 1, "share": 0.5},
        ])
        self.assertEqual(report["types"][0]["token"], "0x02000002")
        self.assertEqual(report["types"][0]["name"], "Example.Root")
        self.assertEqual(report["types"][0]["partial"], 2)
        self.assertEqual(report["types"][0]["reasons"], {
            "unsupported opcode callvirt": 1,
            "unsupported signature type needs IL fallback": 1,
        })
        self.assertTrue(any("--no-auto-references" in command for command in commands))

    def test_type_names_can_be_derived_from_list(self):
        commands = []

        def runner(command, timeout):
            commands.append(command)
            if command[1] == "list":
                return {"text": LIST, "stderr": "", "exitCode": 0, "elapsedMs": 1}
            name = command[command.index("--type") + 1]
            text = DETAILS if name == "Example.Root" else "// type token 0x02000009 Empty\n"
            return {"text": text, "stderr": "", "exitCode": 0, "elapsedMs": 1}

        report = fallback_report.collect("lite", "Example.dll", runner=runner)
        self.assertEqual(report["summary"]["scannedTypes"], 3)
        self.assertTrue(any(command[1] == "list" for command in commands))
        self.assertEqual(report["summary"]["bodies"], 3)

    def test_a_broken_type_is_reported_without_hiding_other_types(self):
        def runner(command, timeout):
            if "--type" in command and command[command.index("--type") + 1] != "Example.Root":
                return {"text": "", "stderr": "error: unreadable", "exitCode": 1, "elapsedMs": 1}
            return {"text": DETAILS, "stderr": "", "exitCode": 2, "elapsedMs": 1}

        report = fallback_report.collect("lite", "Example.dll",
                                         type_names=["Example.Root", "Example.Broken"], runner=runner)
        self.assertEqual(len(report["errors"]), 1)
        self.assertIn("Example.Broken", report["errors"][0]["type"])
        self.assertEqual(report["summary"]["partial"], 2)

    def test_reason_offsets_and_tokens_are_not_split_into_many_rows(self):
        self.assertEqual(fallback_report.normalize_reason("unsupported opcode callvirt at IL_0010"),
                         "unsupported opcode callvirt")
        self.assertEqual(fallback_report.normalize_reason("external field needs IL fallback at IL_00AF"),
                         "external field needs IL fallback")


if __name__ == "__main__":
    unittest.main()
