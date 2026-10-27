#!/usr/bin/env python3
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
"""Exercise corpus acquisition through its public CLI without running inputs."""
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import zipfile

HERE = Path(__file__).resolve().parent
FIXTURE = HERE.parents[1] / "tests/fixtures/release.bin"

MSCORLIB = "mscorlib, Version=4.0.0.0, Culture=neutral, PublicKeyToken=b77a5c561934e089, ContentType=0x0"
SYSTEM = "System, Version=4.0.0.0, Culture=neutral, PublicKeyToken=b77a5c561934e089, ContentType=0x0"
APP = "App, Version=1.0.0.0, Culture=neutral, PublicKeyToken=null, ContentType=0x0"


def load_corpus():
    """Import corpus.py by path so selection rules are testable without the CLI."""
    spec = importlib.util.spec_from_file_location("corpus_under_test", HERE / "corpus.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class CorpusE2E(unittest.TestCase):
    def test_unsafe_archive_is_explicitly_incomplete_without_traversal(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            checkout = root / "checkout"
            checkout.mkdir()
            with zipfile.ZipFile(checkout / "unsafe.nupkg", "w") as archive:
                archive.writestr("../escape.dll", FIXTURE.read_bytes())
            subprocess.run(["git", "init", "-q", str(checkout)], check=True)
            subprocess.run(["git", "-C", str(checkout), "add", "."], check=True)
            subprocess.run(["git", "-C", str(checkout), "-c", "user.name=Corpus Test",
                            "-c", "user.email=corpus@example.invalid", "-c", "core.hooksPath=/dev/null",
                            "commit", "-qm", "Owned unsafe-archive fixture"], check=True)
            commit = subprocess.check_output(["git", "-C", str(checkout), "rev-parse", "HEAD"], text=True).strip()
            result = subprocess.run([sys.executable, str(HERE / "corpus.py"), "collect",
                                     "--checkout", str(checkout), "--commit", commit,
                                     "--output", str(root / "report")], capture_output=True, text=True)
            self.assertEqual(result.returncode, 1, result.stderr)
            manifest = json.loads((root / "report/manifest.json").read_text())
            self.assertFalse(manifest["complete"])
            self.assertIn("unsafe", manifest["errors"][0]["error"])
            self.assertFalse(list(root.rglob("escape.dll")))

    def test_archive_cannot_follow_an_existing_extraction_directory_symlink(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            checkout = root / "checkout"
            checkout.mkdir()
            package = checkout / "package.nupkg"
            with zipfile.ZipFile(package, "w") as archive:
                archive.writestr("tools/sample.dll", FIXTURE.read_bytes())
            subprocess.run(["git", "init", "-q", str(checkout)], check=True)
            subprocess.run(["git", "-C", str(checkout), "add", "."], check=True)
            subprocess.run(["git", "-C", str(checkout), "-c", "user.name=Corpus Test",
                            "-c", "user.email=corpus@example.invalid", "-c", "core.hooksPath=/dev/null",
                            "commit", "-qm", "Owned extraction fixture"], check=True)
            commit = subprocess.check_output(["git", "-C", str(checkout), "rev-parse", "HEAD"], text=True).strip()
            output = root / "report"
            digest = hashlib.sha256(package.read_bytes()).hexdigest()
            extraction = output / "extracted" / digest
            extraction.mkdir(parents=True)
            escape = root / "escape"
            escape.mkdir()
            (extraction / "tools").symlink_to(escape, target_is_directory=True)
            result = subprocess.run([sys.executable, str(HERE / "corpus.py"), "collect",
                                     "--checkout", str(checkout), "--commit", commit,
                                     "--output", str(output)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 1, result.stderr)
            self.assertFalse((escape / "sample.dll").exists())
            manifest = json.loads((output / "manifest.json").read_text())
            self.assertFalse(manifest["complete"])
            self.assertIn("symlink", manifest["errors"][0]["error"])

    def test_baseline_rejects_omitted_bodyless_methods(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            snapshot = root / "methods.json"
            snapshot.write_text(json.dumps({"methods": [
                {"token": "0x06000001", "body_kind": "declaration", "owner": "Owned", "name": "Extern"}]}))
            inventory = root / "inventory.json"
            inventory.write_text(json.dumps({"complete": True, "images": [
                {"id": 0, "kind": "managed", "assembly": "Owned", "path": str(FIXTURE),
                 "origins": ["owned"], "inventory_path": str(snapshot)}]}))
            # The external process reports success while omitting the known declaration.
            cli = root / "empty-scan"
            cli.write_text("#!/bin/sh\nexit 0\n")
            cli.chmod(0o755)
            result = subprocess.run([sys.executable, str(HERE / "corpus.py"), "baseline",
                                     "--inventory", str(inventory), "--cli", str(cli),
                                     "--output", str(root / "baseline")], capture_output=True, text=True)
            self.assertEqual(result.returncode, 1, result.stderr)
            report = json.loads((root / "baseline/baseline.json").read_text())
            self.assertFalse(report["complete"])
            self.assertIn("0x06000001", json.dumps(report["images"][0]["errors"]))

    def test_baseline_runs_production_cli_and_accounts_for_every_fixture_body(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "manifest.json"
            manifest.write_text(json.dumps({"schema": 1, "complete": True, "files": [
                {"origin": "owned", "path": str(FIXTURE), "pe_candidate": True}]}))
            subprocess.run([sys.executable, str(HERE / "corpus.py"), "inventory",
                            "--manifest", str(manifest), "--output", str(root / "inventory")],
                           capture_output=True, text=True, check=True)
            cli = HERE.parents[1] / "target/release/ilspy-lite"
            self.assertTrue(cli.is_file(), "existing release CLI is required")
            result = subprocess.run([sys.executable, str(HERE / "corpus.py"), "baseline",
                                     "--inventory", str(root / "inventory/inventory.json"),
                                     "--cli", str(cli), "--output", str(root / "baseline")],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 2, result.stderr)
            report = json.loads((root / "baseline/baseline.json").read_text())
            image = report["images"][0]
            self.assertTrue(image["scan_complete"], image["errors"])
            self.assertEqual(image["bodies"], 59)
            self.assertEqual(sum(image["counts"].values()), 59)
            self.assertFalse(image["gate_pass"])
            self.assertEqual(len(json.loads(Path(image["results_path"]).read_text())), 59)

    def test_inventory_distinguishes_reference_markers_from_implementation_images(self):
        contract = HERE.parents[1] / "tests/fixtures/binding-contract-release.bin"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "manifest.json"
            manifest.write_text(json.dumps({"schema": 1, "complete": True, "files": [
                {"origin": "contract", "path": str(contract), "pe_candidate": True},
                {"origin": "implementation", "path": str(FIXTURE), "pe_candidate": True}]}))
            result = subprocess.run([sys.executable, str(HERE / "corpus.py"), "inventory",
                                     "--manifest", str(manifest), "--output", str(root / "inventory")],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            images = json.loads((root / "inventory/inventory.json").read_text())["images"]
            self.assertEqual([i["reference_marker"] for i in images], [True, False])
            self.assertTrue(all("zero_code_methods" in i for i in images))

    def test_inventory_recognizes_a_core_local_reference_marker_but_not_a_local_impostor(self):
        fixtures = HERE.parents[1] / "tests/fixtures"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "manifest.json"
            manifest.write_text(json.dumps({"schema": 1, "complete": True, "files": [
                {"origin": name, "path": str(fixtures / name), "pe_candidate": True}
                for name in ("core-reference.bin", "core-fake-marker.bin", "core-types.bin", "core-generic-marker.bin")]}))
            result = subprocess.run([sys.executable, str(HERE / "corpus.py"), "inventory",
                                     "--manifest", str(manifest), "--output", str(root / "inventory")],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            images = json.loads((root / "inventory/inventory.json").read_text())["images"]
            self.assertEqual([i["reference_marker"] for i in images], [True, False, False, False])

    def test_scope_skips_reference_contracts_without_rescanning_inputs(self):
        contract = HERE.parents[1] / "tests/fixtures/binding-contract-release.bin"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "manifest.json"
            manifest.write_text(json.dumps({"schema": 1, "complete": True, "files": [
                {"origin": "contract", "path": str(contract), "pe_candidate": True},
                {"origin": "implementation", "path": str(FIXTURE), "pe_candidate": True}]}))
            subprocess.run([sys.executable, str(HERE / "corpus.py"), "inventory",
                            "--manifest", str(manifest), "--output", str(root / "inventory")],
                           capture_output=True, text=True, check=True)
            cli = HERE.parents[1] / "target/release/ilspy-lite"
            result = subprocess.run([sys.executable, str(HERE / "corpus.py"), "baseline",
                                     "--inventory", str(root / "inventory/inventory.json"),
                                     "--cli", str(cli), "--output", str(root / "baseline")],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 2, result.stderr)
            # Only saved reports remain accessible to the scope command.
            saved = json.loads((root / "inventory/inventory.json").read_text())
            for image in saved["images"]:
                image["path"] = str(root / "no-input-access" / str(image["id"]))
            (root / "inventory/inventory.json").write_text(json.dumps(saved))
            result = subprocess.run([sys.executable, str(HERE / "implementation_scope.py"),
                                     "--inventory", str(root / "inventory/inventory.json"),
                                     "--baseline", str(root / "baseline/baseline.json"),
                                     "--output", str(root / "scope")], capture_output=True, text=True)
            self.assertEqual(result.returncode, 2, result.stderr)
            scoped = json.loads((root / "scope/implementation-baseline.json").read_text())
            self.assertTrue(scoped["complete"], scoped["errors"])
            self.assertEqual(scoped["summary"]["bodies"], 59)
            self.assertEqual(len(scoped["images"]), 1)
            self.assertEqual(scoped["images"][0]["origins"], ["implementation"])
            self.assertEqual(len(scoped["skipped"]), 1)
            self.assertEqual(scoped["skipped"][0]["origins"], ["contract"])
            self.assertEqual(scoped["skipped"][0]["reason"], "verified_reference_contract")
            original = json.loads((root / "baseline/baseline.json").read_text())
            self.assertEqual(scoped["images"][0]["counts"], original["images"][1]["counts"])
            reconciliation = scoped["reconciliation"]
            self.assertEqual(reconciliation["original_bodies"],
                             reconciliation["implementation_bodies"] + reconciliation["skipped_bodies"])
            self.assertEqual(reconciliation["newly_recovered_by_scope_change"], 0)
            target_inventory = json.loads((root / "scope/implementation-inventory.json").read_text())
            self.assertEqual([i["origins"] for i in target_inventory["images"]], [["implementation"]])
            self.assertEqual(target_inventory["summary"]["cil_methods"], 59)
            self.assertEqual(target_inventory["excluded_reference_ids"], [0])

    def test_scope_does_not_skip_an_unmarked_empty_image_by_name_or_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "manifest.json"
            manifest.write_text(json.dumps({"complete": True,
                                           "commit": "9668d49b9f71b40045e5417b55d20e330be95921"}))
            image = {"id": 0, "kind": "managed", "assembly": "mscorlib", "mvid": "owned",
                     "sha256": "0" * 64, "origins": ["dotnet/legacy/mscorlib.dll"],
                     "reference_marker": False, "zero_code_methods": 2,
                     "method_rows": 2, "counts": {"cil": 2}}
            inventory = root / "inventory.json"
            inventory.write_text(json.dumps({"complete": True, "manifest": str(manifest), "images": [image]}))
            result = {"id": 0, "assembly": image["assembly"], "origins": image["origins"],
                      "bodies": 2, "counts": {"Partial": 2}, "scan_complete": True,
                      "reasons": {"empty method body": 2}, "results_path": "saved-results.json",
                      "stamp": {"image_sha256": image["sha256"]}}
            baseline = root / "baseline.json"
            baseline.write_text(json.dumps({"complete": True, "images": [result], "cli": {},
                                            "summary": {"bodies": 2, "counts": {"Partial": 2}}}))
            process = subprocess.run([sys.executable, str(HERE / "implementation_scope.py"),
                                      "--inventory", str(inventory), "--baseline", str(baseline),
                                      "--output", str(root / "scope")], capture_output=True, text=True)
            self.assertEqual(process.returncode, 2, process.stderr)
            scoped = json.loads((root / "scope/implementation-baseline.json").read_text())
            self.assertTrue(scoped["complete"], scoped["errors"])
            self.assertEqual(scoped["skipped"], [])
            self.assertEqual(scoped["summary"]["bodies"], 2)
            self.assertEqual(scoped["summary"]["counts"], {"Partial": 2})

    def test_gate_uses_integer_counts_and_incomplete_scans_cannot_pass(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            report = root / "baseline.json"
            report.write_text(json.dumps({"complete": True, "images": [
                {"id": 0, "bodies": 100, "counts": {"Decompiled": 99, "Partial": 1}, "scan_complete": True},
                {"id": 1, "bodies": 99, "counts": {"Decompiled": 98, "Partial": 1}, "scan_complete": True},
                {"id": 2, "bodies": 100, "counts": {"Decompiled": 100}, "scan_complete": False},
                {"id": 3, "bodies": 100, "counts": {"Decompiled": 99}, "scan_complete": True}]}))
            result = subprocess.run([sys.executable, str(HERE / "corpus.py"), "gate",
                                     "--baseline", str(report)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 1, result.stderr)
            decisions = json.loads(result.stdout)
            self.assertEqual([r["pass"] for r in decisions["images"]], [True, False, False, False])
            self.assertEqual(decisions["images"][1]["unrecovered"], 1)

    def test_gate_uses_the_weighted_one_percent_boundary(self):
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "baseline.json"
            images = [
                {"id": 0, "bodies": 10, "counts": {"Decompiled": 9, "Partial": 1}, "scan_complete": True},
                {"id": 1, "bodies": 190, "counts": {"Decompiled": 188, "Failed": 2}, "scan_complete": True},
            ]
            def run():
                report.write_text(json.dumps({"complete": True, "images": images}))
                result = subprocess.run([sys.executable, str(HERE / "corpus.py"), "gate",
                                         "--baseline", str(report)], capture_output=True, text=True)
                return result, json.loads(result.stdout)
            result, decision = run()
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertFalse(decision["pass"])
            self.assertEqual((decision["bodies"], decision["unrecovered"]), (200, 3))
            images[1]["counts"] = {"Decompiled": 189, "Failed": 1}
            result, decision = run()
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertTrue(decision["pass"])
            self.assertEqual((decision["bodies"], decision["unrecovered"]), (200, 2))
            self.assertFalse(decision["images"][0]["pass"])
            self.assertEqual(decision["threshold_percent"], 1)
            images[:] = [{"id": 0, "bodies": 200000,
                          "counts": {"Decompiled": 197999, "Partial": 2001}, "scan_complete": True}]
            result, decision = run()
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertFalse(decision["pass"])
            self.assertEqual(f"{100 * decision['unrecovered'] / decision['bodies']:.2f}", "1.00")

    def test_gate_rejects_five_percent_and_preserves_incomplete_cases(self):
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "baseline.json"
            images = [
                {"id": 0, "bodies": 10, "counts": {"Decompiled": 8, "Partial": 2}, "scan_complete": True},
                {"id": 1, "bodies": 90, "counts": {"Decompiled": 87, "Failed": 3}, "scan_complete": True},
            ]
            def run():
                report.write_text(json.dumps({"complete": True, "images": images}))
                result = subprocess.run([sys.executable, str(HERE / "corpus.py"), "gate",
                                         "--baseline", str(report)], capture_output=True, text=True)
                return result, json.loads(result.stdout)
            result, decision = run()
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertFalse(decision["pass"])
            self.assertEqual((decision["bodies"], decision["unrecovered"]), (100, 5))
            self.assertFalse(decision["images"][0]["pass"])
            images[1]["counts"] = {"Decompiled": 86, "Failed": 4}
            result, decision = run()
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertFalse(decision["pass"])
            self.assertEqual(decision["unrecovered"], 6)
            images[1]["counts"] = {"Decompiled": 86, "Failed": 3}
            result, decision = run()
            self.assertEqual(result.returncode, 1, result.stderr)
            self.assertFalse(decision["complete"])
            self.assertEqual(decision["unrecovered"], 6)
            images[:] = [{"id": 0, "bodies": 200000,
                          "counts": {"Decompiled": 189999, "Partial": 10001}, "scan_complete": True}]
            result, decision = run()
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertFalse(decision["pass"])
            self.assertEqual(f"{100 * decision['unrecovered'] / decision['bodies']:.2f}", "5.00")
            images[:] = [{"id": 0, "bodies": 0, "counts": {}, "scan_complete": True}]
            result, decision = run()
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertFalse(decision["pass"])
            self.assertFalse(decision["applicable"])

    def test_baseline_exit_uses_aggregate_not_each_image(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            images = []
            for i, count in enumerate((10, 90)):
                methods = [{"token": f"0x{0x06000000 | n:08X}", "body_kind": "cil",
                            "owner": "Owned", "name": f"M{n}"} for n in range(1, count + 1)]
                snapshot = root / f"methods-{i}.json"
                snapshot.write_text(json.dumps({"methods": methods}))
                images.append({"id": i, "kind": "managed", "assembly": f"Owned{i}",
                               "path": str(root / f"input-{i}.bin"), "origins": [f"owned{i}"],
                               "inventory_path": str(snapshot)})
            inventory = root / "inventory.json"
            inventory.write_text(json.dumps({"complete": True, "images": images}))
            cli = root / "owned-fake-cli"
            cli.write_text("#!/usr/bin/env python3\nimport json,sys\n"
                           "i=int(sys.argv[2].split('-')[-1].split('.')[0])\n"
                           "n,p=((10,2),(90,3))[i]\n"
                           "for x in range(1,n+1):\n"
                           " print(json.dumps({'token':f'0x{0x06000000|x:08X}',"
                           "'status':'Partial' if x<=p else 'Decompiled','reason':'owned'}))\n")
            cli.chmod(0o755)
            result = subprocess.run([sys.executable, str(HERE / "corpus.py"), "baseline",
                                     "--inventory", str(inventory), "--cli", str(cli),
                                     "--output", str(root / "baseline"), "--workers", "2"],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 2, result.stderr)
            baseline = json.loads((root / "baseline/baseline.json").read_text())
            self.assertTrue(baseline["complete"])
            self.assertEqual(baseline["summary"]["bodies"], 100)
            self.assertEqual(baseline["summary"]["counts"]["Partial"], 5)
            self.assertEqual(baseline["summary"]["aggregate_threshold_percent"], 1)
            self.assertFalse(baseline["summary"]["aggregate_gate_pass"])
            self.assertFalse(baseline["images"][0]["gate_pass"])
            cli.write_text(cli.read_text().replace("((10,2),(90,3))", "((10,1),(90,0))"))
            passing = subprocess.run([sys.executable, str(HERE / "corpus.py"), "baseline",
                                      "--inventory", str(inventory), "--cli", str(cli),
                                      "--output", str(root / "one-percent"), "--workers", "2"],
                                     capture_output=True, text=True)
            self.assertEqual(passing.returncode, 0, passing.stderr)
            exact = json.loads((root / "one-percent/baseline.json").read_text())
            self.assertEqual(exact["summary"]["unrecovered"], 1)
            self.assertTrue(exact["summary"]["aggregate_gate_pass"])
            self.assertEqual(exact["summary"]["aggregate_threshold_percent"], 1)
            self.assertFalse(exact["images"][0]["gate_pass"])
            inventory.write_text(json.dumps({"complete": True, "images": []}))
            empty = subprocess.run([sys.executable, str(HERE / "corpus.py"), "baseline",
                                    "--inventory", str(inventory), "--cli", str(cli),
                                    "--output", str(root / "empty"), "--workers", "2"],
                                   capture_output=True, text=True)
            self.assertEqual(empty.returncode, 2, empty.stderr)
            empty_report = json.loads((root / "empty/baseline.json").read_text())
            self.assertTrue(empty_report["complete"])
            self.assertFalse(empty_report["summary"]["aggregate_gate_applicable"])

    def test_implementation_scope_exit_uses_the_weighted_gate(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "manifest.json"
            manifest.write_text(json.dumps({"complete": True}))
            inventory_images, baseline_images = [], []
            for i, (bodies, recovered, reference) in enumerate(((10, 8, False), (90, 87, False), (100, 0, True))):
                identity = {"id": i, "assembly": f"Owned{i}", "origins": [f"owned{i}"], "sha256": str(i) * 64}
                inventory_images.append({**identity, "kind": "managed", "reference_marker": reference,
                                         "zero_code_methods": 0, "counts": {"cil": bodies},
                                         "inventory_path": f"owned-{i}.json"})
                baseline_images.append({**identity, "bodies": bodies,
                                        "counts": {"Decompiled": recovered, "Partial": bodies - recovered},
                                        "scan_complete": True, "reasons": {"owned": bodies - recovered},
                                        "results_path": f"results-{i}.json", "stamp": {"image_sha256": identity["sha256"]}})
            inventory = root / "inventory.json"
            inventory.write_text(json.dumps({"complete": True, "manifest": str(manifest), "images": inventory_images}))
            baseline = root / "baseline.json"
            baseline.write_text(json.dumps({"complete": True, "images": baseline_images, "cli": {},
                                            "summary": {"bodies": 200, "counts": {"Decompiled": 95, "Partial": 105}}}))
            result = subprocess.run([sys.executable, str(HERE / "implementation_scope.py"),
                                     "--inventory", str(inventory), "--baseline", str(baseline),
                                     "--output", str(root / "scope")], capture_output=True, text=True)
            self.assertEqual(result.returncode, 2, result.stderr)
            scoped = json.loads((root / "scope/implementation-baseline.json").read_text())
            self.assertTrue(scoped["complete"])
            self.assertEqual(scoped["summary"]["bodies"], 100)
            self.assertEqual(scoped["summary"]["counts"], {"Decompiled": 95, "Partial": 5})
            self.assertEqual(scoped["summary"]["aggregate_threshold_percent"], 1)
            self.assertFalse(scoped["summary"]["aggregate_gate_pass"])
            self.assertEqual(len(scoped["skipped"]), 1)
            self.assertEqual(scoped["reconciliation"]["newly_recovered_by_scope_change"], 0)
            baseline_images[1]["counts"]["Decompiled"] -= 1
            baseline.write_text(json.dumps({"complete": True, "images": baseline_images, "cli": {},
                                            "summary": {"bodies": 200, "counts": {"Decompiled": 94, "Partial": 105}}}))
            result = subprocess.run([sys.executable, str(HERE / "implementation_scope.py"),
                                     "--inventory", str(inventory), "--baseline", str(baseline),
                                     "--output", str(root / "incomplete")], capture_output=True, text=True)
            self.assertEqual(result.returncode, 1, result.stderr)
            incomplete = json.loads((root / "incomplete/implementation-baseline.json").read_text())
            self.assertFalse(incomplete["complete"])
            self.assertEqual(incomplete["summary"]["unrecovered"], 6)
            self.assertEqual(incomplete["summary"]["incomplete_images"], 1)
            self.assertIn("unrecovered: 6", (root / "incomplete/SCOPE.md").read_text())
            baseline_images[0]["counts"] = {"Decompiled": 9, "Partial": 1}
            baseline_images[1]["counts"] = {"Decompiled": 90, "Partial": 0}
            baseline.write_text(json.dumps({"complete": True, "images": baseline_images, "cli": {},
                                            "summary": {"bodies": 200, "counts": {"Decompiled": 99, "Partial": 101}}}))
            passing = subprocess.run([sys.executable, str(HERE / "implementation_scope.py"),
                                      "--inventory", str(inventory), "--baseline", str(baseline),
                                      "--output", str(root / "one-percent")], capture_output=True, text=True)
            self.assertEqual(passing.returncode, 0, passing.stderr)
            exact = json.loads((root / "one-percent/implementation-baseline.json").read_text())
            self.assertTrue(exact["complete"])
            self.assertTrue(exact["summary"]["aggregate_gate_pass"])
            self.assertEqual(exact["summary"]["aggregate_threshold_percent"], 1)
            self.assertEqual(exact["summary"]["unrecovered"], 1)
            self.assertIn("Aggregate <= 1% gate: pass", (root / "one-percent/SCOPE.md").read_text())

    def test_inventory_preserves_raw_windows_metadata_without_projection(self):
        fixture = HERE.parents[2] / "ILSpy-tests/Mono.Cecil-net45/Resources/assemblies/winrtcomp.winmd"
        self.assertTrue(fixture.is_file(), "fixed submodule fixture is required")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "manifest.json"
            manifest.write_text(json.dumps({"schema": 1, "complete": True, "files": [
                {"origin": "winmd", "path": str(fixture), "pe_candidate": True}]}))
            subprocess.run([sys.executable, str(HERE / "corpus.py"), "inventory",
                            "--manifest", str(manifest), "--output", str(root / "inventory")],
                           capture_output=True, text=True, check=False)
            inventory = json.loads((root / "inventory/inventory.json").read_text())
            image = inventory["images"][0]
            details = json.loads(Path(image["inventory_path"]).read_text())
            self.assertEqual(details["reader_options"], "None")
            self.assertEqual(details["method_rows"], 9)
            self.assertIn("WindowsRuntime", details["metadata_version"])

    def test_independent_inventory_reads_all_fixture_methods_without_execution(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "manifest.json"
            manifest.write_text(json.dumps({"schema": 1, "complete": True, "files": [
                {"origin": "owned", "path": str(FIXTURE), "pe_candidate": True}]}))
            result = subprocess.run([sys.executable, str(HERE / "corpus.py"), "inventory",
                                     "--manifest", str(manifest), "--output", str(root / "inventory")],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            inventory = json.loads((root / "inventory/inventory.json").read_text())
            self.assertTrue(inventory["complete"], inventory["errors"])
            image = inventory["images"][0]
            self.assertEqual(image["kind"], "managed")
            details = json.loads(Path(image["inventory_path"]).read_text())
            methods = details["methods"]
            self.assertEqual(len(methods), 59)
            self.assertEqual(len({m["token"] for m in methods}), 59)
            self.assertTrue(any(m["name"] == "MoveNext" for m in methods))
            self.assertTrue(any(m["name"] == ".ctor" for m in methods))
            self.assertTrue(all(m["body_kind"] == "cil" for m in methods))
            self.assertFalse(inventory["input_executed"])

    def test_collect_retains_direct_and_archived_inputs_with_provenance(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            checkout = root / "checkout"
            checkout.mkdir()
            data = FIXTURE.read_bytes()
            (checkout / "sample.winmd").write_bytes(data)
            (checkout / "readme.txt").write_text("owned test data\n")
            with zipfile.ZipFile(checkout / "package.nupkg", "w") as archive:
                archive.writestr("tools/sample.netmodule", data)
                archive.writestr("content/readme.txt", "not a PE")
            subprocess.run(["git", "init", "-q", str(checkout)], check=True)
            subprocess.run(["git", "-C", str(checkout), "add", "."], check=True)
            subprocess.run(["git", "-C", str(checkout), "-c", "user.name=Corpus Test",
                            "-c", "user.email=corpus@example.invalid", "-c", "core.hooksPath=/dev/null",
                            "commit", "-qm", "Owned fixture corpus"], check=True)
            commit = subprocess.check_output(["git", "-C", str(checkout), "rev-parse", "HEAD"], text=True).strip()
            result = subprocess.run([sys.executable, str(HERE / "corpus.py"), "collect",
                                     "--checkout", str(checkout), "--commit", commit,
                                     "--output", str(root / "report")], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            manifest = json.loads((root / "report/manifest.json").read_text())
            self.assertTrue(manifest["complete"], manifest["errors"])
            files = manifest["files"]
            self.assertEqual(len(files), 5)
            pe = [f for f in files if f["pe_candidate"]]
            self.assertEqual(len(pe), 2)
            self.assertEqual({f["origin"] for f in pe},
                             {"sample.winmd", "package.nupkg!tools/sample.netmodule"})
            self.assertEqual({f["sha256"] for f in pe}, {hashlib.sha256(data).hexdigest()})
            self.assertTrue(all(Path(f["path"]).read_bytes() == data for f in pe))
            self.assertEqual(manifest["commit"], commit)

    def test_ambiguity_tie_break_prefers_the_scanned_images_directory_family(self):
        corpus = load_corpus()
        family = "/corpus/mcs/5.23/lib/mono/4.5/mscorlib.dll"
        reference = "/corpus/dotnet/netcore-2.2/mscorlib.dll"
        legacy = "/corpus/dotnet/legacy/mscorlib.dll"
        own = "/corpus/mcs/5.23/lib/mono/4.5/Owned.exe"
        owned = {MSCORLIB: [reference, legacy, family]}
        decisions = corpus.select_references(owned, [MSCORLIB], own, "exact", "tie-break")
        self.assertEqual(len(decisions), 1)
        decision = decisions[0]
        self.assertEqual(decision["outcome"], "selected")
        self.assertEqual(decision["chosen"], family)
        self.assertEqual(decision["basis"], "directory-family")
        self.assertEqual(decision["candidates"], sorted([reference, legacy, family]))

    def test_ambiguity_tie_break_prefers_non_legacy_then_path_order(self):
        corpus = load_corpus()
        legacy = "/corpus/dotnet/legacy/mscorlib.dll"
        reference = "/corpus/dotnet/netcore-2.2/mscorlib.dll"
        own = "/corpus/elsewhere/Owned.exe"
        owned = {MSCORLIB: [legacy, reference]}
        preferred = corpus.select_references(owned, [MSCORLIB], own, "exact", "tie-break")[0]
        self.assertEqual((preferred["chosen"], preferred["basis"]), (reference, "non-legacy"))
        first = "/corpus/b/mscorlib.dll"
        second = "/corpus/a/mscorlib.dll"
        ordered = corpus.select_references({MSCORLIB: [first, second]}, [MSCORLIB], own,
                                           "exact", "tie-break")[0]
        self.assertEqual((ordered["chosen"], ordered["basis"]), (second, "path-order"))
        skipped = corpus.select_references({MSCORLIB: [first, second]}, [MSCORLIB], own,
                                           "exact", "skip")[0]
        self.assertEqual((skipped["outcome"], skipped["chosen"]), ("ambiguous", None))
        self.assertEqual(skipped["candidates"], sorted([first, second]))

    def test_tie_break_passes_exactly_one_file_per_identity_and_never_the_image_itself(self):
        corpus = load_corpus()
        own = "/corpus/app/Owned.exe"
        kept = "/corpus/app/mscorlib.dll"
        other = "/corpus/other/mscorlib.dll"
        owned = {MSCORLIB: [kept, other], APP: [own]}
        plan = corpus.plan_references(owned, {own: [MSCORLIB, APP]}, own, "exact", "tie-break", 1)
        self.assertEqual(plan["passed"], [kept])
        self.assertEqual(len(plan["passed"]), len(set(plan["passed"])))
        self.assertEqual([d["identity"] for d in plan["declared"]], sorted([APP, MSCORLIB]))
        self.assertEqual([d["outcome"] for d in plan["declared"]], ["unresolved", "selected"])
        self.assertEqual(corpus.selection_counts(plan["declared"])["ambiguous"], 0)

    def test_one_hop_closure_adds_only_the_references_of_selected_files(self):
        corpus = load_corpus()
        own = "/corpus/app/Owned.exe"
        direct = "/corpus/app/Direct.dll"
        transitive = "/corpus/lib/Transitive.dll"
        second_hop = "/corpus/lib/SecondHop.dll"
        deeper = "Deeper, Version=1.0.0.0, Culture=neutral, PublicKeyToken=null, ContentType=0x0"
        owned = {SYSTEM: [direct], MSCORLIB: [transitive], APP: [own], deeper: [second_hop]}
        declared = {own: [SYSTEM], direct: [MSCORLIB, APP], transitive: [deeper]}
        off = corpus.plan_references(owned, declared, own, "exact", "tie-break", 0)
        self.assertEqual(off["passed"], [direct])
        self.assertEqual(off["hop1"], [])
        hop = corpus.plan_references(owned, declared, own, "exact", "tie-break", 1)
        self.assertEqual(hop["passed"], [direct, transitive])
        self.assertEqual([d["identity"] for d in hop["hop1"]], sorted([APP, MSCORLIB]))
        self.assertEqual([d["outcome"] for d in hop["hop1"]], ["unresolved", "selected"])
        self.assertNotIn(second_hop, hop["passed"])
        self.assertNotIn(own, hop["passed"])

    def test_plan_prints_the_selection_without_invoking_the_scan(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            app = root / "app/Owned.exe"
            direct = root / "app/Direct.dll"
            transitive = root / "lib/Transitive.dll"
            log = root / "stub.log"
            stub = root / "stub-cli"
            answers = [json.dumps(record) for record in (
                {"identity": APP, "path": str(app), "references": [SYSTEM]},
                {"identity": SYSTEM, "path": str(direct), "references": [MSCORLIB]},
                {"identity": MSCORLIB, "path": str(transitive), "references": []})]
            stub.write_text("#!/bin/sh\n"
                            "echo \"$1\" >> " + str(log) + "\n"
                            "[ \"$1\" = identities ] || exit 3\n"
                            + "".join("printf '%s\\n' '" + answer + "'\n" for answer in answers))
            stub.chmod(0o755)
            inventory = root / "inventory.json"
            inventory.write_text(json.dumps({"complete": True, "images": [
                {"id": 0, "kind": "managed", "assembly": "Owned", "path": str(app),
                 "origins": ["owned"], "counts": {"cil": 3}}]}))
            result = subprocess.run([sys.executable, str(HERE / "corpus.py"), "plan",
                                     "--inventory", str(inventory), "--cli", str(stub)],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            planned = json.loads(result.stdout)
            self.assertEqual(planned["images"][0]["selected"], 1)
            self.assertEqual((planned["images"][0]["hop"], planned["images"][0]["hop1_selected"]), (0, 0))
            self.assertEqual(log.read_text().split(), ["identities"])
            hopped = subprocess.run([sys.executable, str(HERE / "corpus.py"), "plan",
                                     "--inventory", str(inventory), "--cli", str(stub),
                                     "--reference-hop", "1", "--output", str(root / "plan")],
                                    capture_output=True, text=True)
            self.assertEqual(hopped.returncode, 0, hopped.stderr)
            row = json.loads(hopped.stdout)["images"][0]
            self.assertEqual((row["hop"], row["hop1_selected"], row["passed"]), (1, 1, 2))
            detail = json.loads((root / "plan/plan.json").read_text())["images"][0]
            self.assertEqual(detail["selection"]["hop1"][0]["chosen"], str(transitive))
            self.assertEqual(log.read_text().split(), ["identities", "identities"])

    def test_baseline_stamp_records_the_reference_selection_knobs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "manifest.json"
            manifest.write_text(json.dumps({"schema": 1, "complete": True, "files": [
                {"origin": "owned", "path": str(FIXTURE), "pe_candidate": True}]}))
            subprocess.run([sys.executable, str(HERE / "corpus.py"), "inventory",
                            "--manifest", str(manifest), "--output", str(root / "inventory")],
                           capture_output=True, text=True, check=True)
            cli = HERE.parents[1] / "target/release/ilspy-lite"
            self.assertTrue(cli.is_file(), "existing release CLI is required")
            result = subprocess.run([sys.executable, str(HERE / "corpus.py"), "baseline",
                                     "--inventory", str(root / "inventory/inventory.json"),
                                     "--cli", str(cli), "--output", str(root / "baseline"),
                                     "--references", "declared", "--reference-hop", "1",
                                     "--reference-ambiguity", "skip"],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 2, result.stderr)
            report = json.loads((root / "baseline/baseline.json").read_text())
            self.assertEqual(report["references"], {"mode": "declared", "policy": "exact",
                                                    "ambiguity": "skip", "hop": 1})
            image = report["images"][0]
            stamp = image["stamp"]["references"]
            self.assertEqual((stamp["mode"], stamp["ambiguity"], stamp["hop"]), ("declared", "skip", 1))
            for key in ("selected", "hop1", "selection"):
                self.assertIn(key, stamp)
            self.assertEqual(stamp["selected"], [d["chosen"] for d in stamp["selection"]["declared"]
                                                 if d["chosen"]])
            self.assertEqual(image["references"]["declared"], image["references"]["declared_identities"])


if __name__ == "__main__":
    unittest.main()
