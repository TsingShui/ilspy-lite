#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Optional static population oracle test; rebuild with inventory.py first."""
import json
import os
from pathlib import Path
import subprocess
import unittest

RUST = Path(__file__).resolve().parents[2]
DOTNET = Path(os.environ.get("DOTNET_ROOT", Path.home() / ".dotnet10")) / "dotnet"
ORACLE = RUST / "report/openra/class-inventory/ClassInventory.dll"


class TypePopulation(unittest.TestCase):
    def test_population_includes_empty_generic_nested_and_nonclass_types(self):
        for config in ("release", "debug"):
            result = subprocess.run([str(DOTNET), str(ORACLE),
                str(RUST / f"tests/fixtures/class-{config}.bin"), "--list-types"],
                text=True, capture_output=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stderr)
            population = json.loads(result.stdout)
            types = population["types"]
            self.assertEqual(len(types), len({t["token"] for t in types}))
            by_name = {t["name"]: t for t in types}
            for name in ("ClassCases.Empty", "ClassCases.EmptyGeneric`1",
                         "ClassCases.Outer/Inner/Leaf", "ClassCases.Derived"):
                self.assertEqual(by_name[name]["kind"], "class")
            self.assertEqual(by_name["ClassCases.IMarker"]["kind"], "interface")
            self.assertEqual(by_name["ClassCases.LayoutFields"]["kind"], "value_type")
            self.assertEqual(by_name["<Module>"]["kind"], "module")
            self.assertNotEqual(by_name["ClassCases.Outer/Inner/Leaf"]["parent"], "0x00000000")
            self.assertNotEqual(population["mvid"], "00000000-0000-0000-0000-000000000000")


if __name__ == "__main__":
    unittest.main()
