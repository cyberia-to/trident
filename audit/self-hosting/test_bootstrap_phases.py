"""Adversarial phase-receipt wiring fixtures; these never claim guest acceptance."""
import copy
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import bootstrap_phase_checks as P

B = P.B
SPEC = importlib.util.spec_from_file_location("bootstrap_phases", HERE / "bootstrap-phases.py")
PHASES = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PHASES)
ORIGIN = dict(run_id="123", run_attempt="1", head_sha="a" * 40)


class PhaseBoundaries(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)

    def identity(self, root, name, value):
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(value if isinstance(value, bytes) else json.dumps(value).encode())
        return dict(path=name, sha256=B.sha(path), bytes=path.stat().st_size)

    def seal(self, root, report):
        report["files"] = self.identity(root, "files.json", B.evidence_files(root))
        self.identity(root, "receipt.json", report)
        return root, report

    def common(self, target, repeat, phase):
        system, arch = B.TARGETS[target]
        return dict(schema=P.SCHEMA, phase=phase, repeat=repeat,
                    status="produced" if phase == "producer" else "passed",
                    target=target, pins={name: "a" * 40 for name in B.REPOS},
                    ci_origin=copy.deepcopy(ORIGIN), rust_version="1.95.0",
                    host={"rust": f"host: {target}\nrelease: 1.95.0", "system": system,
                          "machine": "arm64" if arch == "arm64" else "x86_64", "libc": "glibc" if system == "Linux" else ""},
                    profile=copy.deepcopy(B.PROFILE), implementation=P.implementation(), commands=[])

    def producer(self, target="aarch64-apple-darwin", repeat=1):
        root = self.root / f"{target}-{repeat}-producer"
        prefix = f"repeat-{repeat}/"
        artifacts = {f"c{g}": self.identity(root, prefix + f"c{g}.dag", b"receipt fixture only") for g in (1, 2, 3)}
        inventory = self.identity(root, prefix + "inventory.json", {"modules": {str(i): {} for i in range(94)}})
        joy = self.identity(root, prefix + ("joy.exe" if "windows" in target else "joy"), b"fixture executable")
        tools = {"joy": joy["sha256"], "inventory": "b" * 64}
        flags = [part for key, value in B.PROFILE.items() for part in ("--" + key, str(value))] + ["--frames", "65536"]
        steps = {}
        for generation in (2, 3):
            steps[f"step{generation}"] = self.identity(root, prefix + f"c{generation}-step.json", dict(
                status="compiler-returned", published_kind="program", result_sha256=artifacts[f"c{generation}"]["sha256"],
                compiler_sha256=artifacts[f"c{generation-1}"]["sha256"],
                compiler_sha256_end=artifacts[f"c{generation-1}"]["sha256"],
                binary_sha256=tools["joy"], binary_sha256_end=tools["joy"],
                inventory_sha256=inventory["sha256"], host_flags=flags))
        comparison = dict(artifact_sha256=artifacts["c2"]["sha256"], source_sha256_set=["fixture"], options={}, limits={})
        fixed = self.identity(root, prefix + "fixed-point.json", dict(
            status="passed", fixed_point=comparison,
            job_checker_sha256_start=tools["joy"], job_checker_sha256_end=tools["joy"],
            inventory_checker_sha256_start=tools["inventory"], inventory_checker_sha256_end=tools["inventory"],
            steps=[dict(receipt_sha256=steps[f"step{g}"]["sha256"]) for g in (2, 3)]))
        report = self.common(target, repeat, "producer")
        report.update(joy=joy, repetitions=[dict(number=repeat, status="produced", corpora={}, tools=tools,
                      inventory=inventory, fixed_point=fixed, comparison=comparison, **artifacts, **steps)])
        return self.seal(root, report)

    def consumer(self, producer, generation):
        proot, original = producer
        target, repeat = original["target"], original["repeat"]
        root = self.root / f"{target}-{repeat}-c{generation}"
        selected = original["repetitions"][0][f"c{generation}"]
        compiler = self.identity(root, f"inputs/c{generation}.dag", (proot / selected["path"]).read_bytes())
        joy = self.identity(root, "inputs/" + ("joy.exe" if "windows" in target else "joy"), (proot / original["joy"]["path"]).read_bytes())
        receipt = self.identity(root, "inputs/producer.json", (proot / "receipt.json").read_bytes())
        manifest = self.identity(root, "inputs/producer-files.json", (proot / "files.json").read_bytes())
        prefix = "C:/native/" + root.name if "windows" in target else "/native/" + root.name
        compiler_path, joy_path = prefix + "/" + compiler["path"], prefix + "/" + joy["path"]
        report = self.common(target, repeat, "corpus")
        report.update(generation=generation, compiler=compiler, joy=joy,
                      producer_receipt=receipt, producer_manifest=manifest,
                      producer=dict(receipt_sha256=receipt["sha256"], files_sha256=manifest["sha256"],
                                    generation=generation, compiler=copy.deepcopy(selected), joy=copy.deepcopy(original["joy"])),
                      fixture_repositories={name: original["pins"][name] for name in ("trident", "joy")},
                      execution=dict(compiler=compiler_path, joy=joy_path), corpora={})
        for script, count in B.CORPORA.items():
            label = f"c{generation}-{script.removesuffix('.py')}"
            name = f"corpora/{label}.json"
            child = dict(status="passed", compiler_mode="provided", compiler_path=compiler_path,
                         compiler_sha256_start=compiler["sha256"], compiler_sha256_end=compiler["sha256"],
                         binary_sha256_start=joy["sha256"], binary_sha256_end=joy["sha256"],
                         observations=[{}] * count,
                         commands=[dict(command=[joy_path, "pack-job", "--compiler", compiler_path], exit_code=0)])
            report["corpora"][label] = self.identity(root, name, child)
            report["commands"].append(dict(status="completed", exit_code=0, command=[
                "python", "/harness/bootstrap-runner.py", "--corpus-child", "/fixtures/trident/audit/self-hosting/" + script,
                "--compiler", compiler_path, "--joy", joy_path, "--output", prefix + "/" + name]))
        return self.seal(root, report)

    def matrix(self):
        reports = []
        for target in B.TARGETS:
            for repeat in (1, 2):
                producer = self.producer(target, repeat)
                reports.extend([producer, self.consumer(producer, 2), self.consumer(producer, 3)])
        return reports

    def test_complete_wiring_requires_twelve_distinct_producers_and_both_generations(self):
        reports = self.matrix()
        self.assertEqual(P.compare_matrix(reports, ORIGIN), reports[0][1]["repetitions"][0]["c2"]["sha256"])
        for changed in (reports[:-1], reports + reports[:1], reports[:-1] + reports[:1]):
            with self.subTest(length=len(changed)), self.assertRaises(ValueError):
                P.compare_matrix(changed, ORIGIN)

    def test_relabelled_c2_corpus_cannot_supply_c3_despite_equal_compiler_bytes(self):
        root, report = self.consumer(self.producer(), 2)
        report["generation"] = 3
        with self.assertRaises(ValueError):
            P.corpus(root, report)
        report["producer"]["generation"] = 3
        with self.assertRaises(ValueError):
            P.corpus(root, report)

    def test_consumer_requires_the_original_producer_receipt_and_manifest(self):
        root, report = self.consumer(self.producer(), 2)
        for key in ("receipt_sha256", "files_sha256"):
            original = report["producer"][key]
            report["producer"][key] = "b" * 64
            with self.subTest(key=key), self.assertRaises(ValueError):
                P.corpus(root, report)
            report["producer"][key] = original

    def test_changed_exported_joy_or_compiler_fails_even_with_refreshed_consumer_manifest(self):
        root, report = self.consumer(self.producer(), 2)
        for key in ("joy", "compiler"):
            original = copy.deepcopy(report[key])
            content = (root / original["path"]).read_bytes()
            report[key] = self.identity(root, original["path"], b"substituted native bytes")
            self.seal(root, report)
            with self.subTest(key=key), self.assertRaises(ValueError):
                P.corpus(root, report)
            report[key] = self.identity(root, original["path"], content)
        self.seal(root, report)
        P.corpus(root, report)

    def test_failed_or_partial_producer_cannot_become_a_completed_phase(self):
        root, report = self.producer()
        for status in ("passed", "running", "failed", "prepared"):
            report["status"] = status
            with self.subTest(status=status), self.assertRaises(ValueError):
                P.producer(root, report)
        report["status"] = "produced"
        report["repetitions"][0]["corpora"] = {"fake": {}}
        with self.assertRaises(ValueError):
            P.producer(root, report)

    def test_changed_raw_files_cannot_keep_original_manifest_green(self):
        root, report = self.producer()
        (root / "repeat-1/c3.dag").write_bytes(b"corrupt")
        with self.assertRaises(ValueError):
            P.producer(root, report)

    def test_corpus_requires_all_six_actual_successful_wrapper_commands(self):
        root, report = self.consumer(self.producer(), 2)
        original = copy.deepcopy(report["commands"])
        mutations = [original[:-1], original + original[:1],
                     [original[0] | {"exit_code": 1}] + original[1:],
                     [original[0] | {"status": "running"}] + original[1:]]
        for commands in mutations:
            report["commands"] = commands
            with self.subTest(commands=len(commands)), self.assertRaises(ValueError):
                P.corpus(root, report)

    def test_corpus_rejects_wrapper_compiler_substitution_duplicate_flag_and_tool_rebuild(self):
        root, report = self.consumer(self.producer(), 2)
        original = copy.deepcopy(report["commands"])
        for tail in (["--compiler", "/other/c3.dag"], ["--joy", "/other/joy"]):
            report["commands"] = copy.deepcopy(original)
            report["commands"][0]["command"].extend(tail)
            with self.assertRaises(ValueError):
                P.corpus(root, report)
        report["commands"] = original + [dict(status="completed", exit_code=0, command=["cargo", "build", "--release"])]
        with self.assertRaises(ValueError):
            P.corpus(root, report)

    def test_corpus_requires_declared_pinned_sibling_fixtures(self):
        root, report = self.consumer(self.producer(), 2)
        report["fixture_repositories"]["joy"] = "b" * 40
        with self.assertRaises(ValueError):
            P.corpus(root, report)

    def test_native_host_profile_and_phase_code_changes_reject(self):
        reports = self.matrix()
        root, report = reports[-1]
        for key, value in (("host", {"rust": "host: other\nrelease: 1.95.0"}),
                           ("profile", report["profile"] | {"time-ms": 1}),
                           ("implementation", report["implementation"] | {"bootstrap-phases.py": "b" * 64})):
            original = report[key]
            report[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                P.compare_matrix(reports, ORIGIN)
            report[key] = original

    def test_mixed_run_attempt_head_and_missing_origin_fail(self):
        reports = self.matrix()
        report = reports[-1][1]
        for origin in (None, ORIGIN | {"run_id": "124"}, ORIGIN | {"run_attempt": "2"}, ORIGIN | {"head_sha": "b" * 40}):
            report["ci_origin"] = origin
            with self.subTest(origin=origin), self.assertRaises(ValueError):
                P.compare_matrix(reports, ORIGIN)
            with self.assertRaises(ValueError):
                P.compare_matrix(reports)

    def test_windows_recorded_paths_replay_on_other_native_hosts(self):
        root, report = self.consumer(self.producer("x86_64-pc-windows-msvc"), 3)
        for key in ("compiler", "joy"):
            report["execution"][key] = report["execution"][key].replace("/", "\\")
        for row in report["commands"]:
            row["command"] = [x.replace("/", "\\") if x.startswith("C:/") else x for x in row["command"]]
        P.corpus(root, report)

    def test_child_receipt_cannot_switch_generation_or_drop_observations(self):
        root, report = self.consumer(self.producer(), 2)
        label = next(iter(report["corpora"]))
        identity = report["corpora"][label]
        original = B.load(root / identity["path"])
        for changed in (original | {"compiler_path": original["compiler_path"].replace("c2.dag", "c3.dag")},
                        original | {"observations": original["observations"][:-1]},
                        original | {"compiler_mode": "host"}):
            report["corpora"][label] = self.identity(root, identity["path"], changed)
            self.seal(root, report)
            with self.assertRaises(ValueError):
                P.corpus(root, report)

    def test_missing_phase_cli_fails_and_preserves_original_reason(self):
        downloaded = self.root / "downloaded"
        downloaded.mkdir()
        output = self.root / "aggregate"
        result = subprocess.run([sys.executable, str(HERE / "bootstrap-phases.py"), "--phase", "matrix",
                                 "--matrix", str(downloaded), "--output", str(output)], capture_output=True)
        self.assertEqual(result.returncode, 1, result.stderr)
        report = B.load(output / "receipt.json")
        self.assertEqual(report["status"], "failed")
        self.assertNotIn("compiler_sha256", report)
        self.assertIn("thirty-six", report["error"])

    def reject_overlap(self, input_root, output, work, phase="corpus"):
        before = {p.relative_to(input_root).as_posix(): p.read_bytes()
                  for p in input_root.rglob("*") if p.is_file()}
        argv = [sys.executable, str(HERE / "bootstrap-phases.py"), "--phase", phase, "--output", str(output)]
        if phase == "matrix":
            argv.extend(["--matrix", str(input_root)])
        else:
            argv.extend(["--producer", str(input_root), "--work", str(work), "--generation", "2",
                         "--target", "aarch64-apple-darwin", "--repeat", "1",
                         "--pins-json", json.dumps({name: "a" * 40 for name in B.REPOS})])
        result = subprocess.run(argv, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"separate", result.stderr)
        after = {p.relative_to(input_root).as_posix(): p.read_bytes()
                 for p in input_root.rglob("*") if p.is_file()}
        self.assertEqual(before, after)
        self.assertFalse(output.exists(), "overlap must reject before creating output")
        if work is not None:
            self.assertFalse(work.exists(), "overlap must reject before creating work")

    def test_overlapping_input_paths_reject_before_any_evidence_or_work_is_created(self):
        for index, phase in enumerate(("corpus", "corpus", "matrix")):
            original = self.root / str(index) / "original"
            original.mkdir(parents=True)
            (original / "receipt.json").write_bytes(b"original retained bytes")
            output = original / "new-output" if index != 1 else self.root / "output-1"
            work = original / "new-work" if index == 1 else self.root / f"work-{index}"
            self.reject_overlap(original, output, work if phase == "corpus" else None, phase)

    def test_symlink_parent_cannot_bypass_input_preservation_preflight(self):
        original = self.root / "original"
        original.mkdir()
        (original / "receipt.json").write_bytes(b"original retained bytes")
        alias = self.root / "alias"
        try:
            os.symlink(original, alias, target_is_directory=True)
        except OSError as error:
            self.skipTest(f"host does not permit directory symlinks: {error}")
        self.reject_overlap(original, alias / "output", self.root / "work")

    def test_producer_routes_whole_self_build_and_exports_exact_native_joy(self):
        args = argparse.Namespace(work=self.root / "work", producer=None, generation=None,
                                  target="x86_64-pc-windows-msvc" if os.name == "nt" else "aarch64-apple-darwin", repeat=2)
        audit = B.Audit(self.root / "evidence", {}, schema=P.SCHEMA)
        expected = b"native producer executable fixture"

        def self_build(audit, work, number, target, pins, **kwargs):
            binary = work / "target" / target / "release" / ("joy.exe" if os.name == "nt" else "joy")
            binary.parent.mkdir(parents=True)
            binary.write_bytes(expected)
            (audit.output / f"repeat-{number}").mkdir()
            return dict(tools={"joy": B.sha(binary)})

        source_pins = {name: "a" * 40 for name in B.REPOS}
        with patch.object(B, "repetition", side_effect=self_build) as build, patch.object(B, "generation_corpora") as corpora:
            PHASES.produce(audit, args, source_pins)
        self.assertEqual(build.call_count, 1)
        self.assertEqual(build.call_args.args[1:], (args.work.resolve() / "repeat-2", 2, args.target, source_pins))
        self.assertEqual(build.call_args.kwargs, dict(producer_only=True, prepared=PHASES.helpers_match))
        corpora.assert_not_called()
        self.assertEqual(audit.report["status"], "produced")
        self.assertEqual(B.retained(audit.output, audit.report["joy"]).read_bytes(), expected)

    def test_consumer_routes_both_generations_through_copied_inputs_and_only_fixture_checkout(self):
        target = "x86_64-pc-windows-msvc" if os.name == "nt" else "aarch64-apple-darwin"
        original_root, original = self.producer(target)
        producer_bytes = (original_root / "receipt.json").read_bytes()
        for generation in (2, 3):
            args = argparse.Namespace(producer=original_root, generation=generation,
                                      work=self.root / f"work-{generation}")
            args.work.mkdir()
            metadata = self.common(target, 1, "corpus")
            for key in ("schema", "status", "commands"):
                del metadata[key]
            audit = B.Audit(self.root / f"consumer-{generation}", metadata, schema=P.SCHEMA)
            sources = args.work / "sources"
            calls = []

            def run_corpora(audit, evidence, helpers, compiler, joy, selected, report):
                calls.append((evidence, helpers, compiler, joy, selected))
                self.assertEqual(compiler.name, f"c{generation}.dag")
                self.assertEqual(compiler.read_bytes(), (original_root / original["repetitions"][0][f"c{generation}"]["path"]).read_bytes())
                self.assertEqual(B.sha(joy), original["joy"]["sha256"])
                if os.name != "nt":
                    self.assertTrue(joy.stat().st_mode & 0o100)

            with patch.object(B, "checkout", return_value=sources) as checkout, \
                 patch.object(PHASES, "helpers_match") as helpers, patch.object(B, "clean") as clean, \
                 patch.object(B, "prepare", side_effect=AssertionError("consumer may not build a seed")), \
                 patch.object(B, "repetition", side_effect=AssertionError("consumer may not self-build")), \
                 patch.object(B, "generation_corpora", side_effect=run_corpora):
                PHASES.consume(audit, args, original["pins"])
            checkout.assert_called_once_with(audit, args.work.resolve(), original["pins"], ("trident", "joy"))
            helpers.assert_called_once_with(sources)
            self.assertEqual([call.args[1].name for call in clean.call_args_list], ["trident", "joy"])
            self.assertEqual(len(calls), 1)
            self.assertEqual(calls[0], (audit.output / "corpora", sources / "trident/audit/self-hosting",
                                      audit.output / "inputs" / f"c{generation}.dag",
                                      audit.output / "inputs" / ("joy.exe" if os.name == "nt" else "joy"), generation))
            self.assertEqual(audit.env["CARGO_NET_OFFLINE"], "true")
            self.assertEqual((original_root / "receipt.json").read_bytes(), producer_bytes)
            self.assertEqual(audit.report["producer"]["generation"], generation)


if __name__ == "__main__":
    unittest.main()
