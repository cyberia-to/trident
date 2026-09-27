"""Failure-boundary tests for SH6 orchestration; no synthetic guest acceptance."""
import copy
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).with_name("bootstrap-runner.py")
SPEC = importlib.util.spec_from_file_location("bootstrap_runner", SCRIPT)
RUNNER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RUNNER)


class BootstrapBoundaries(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.addCleanup(self.temporary.cleanup)

    def identity(self, name, value):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(value, bytes):
            path.write_bytes(value)
        else:
            path.write_text(json.dumps(value), encoding="utf-8")
        return dict(path=name, sha256=RUNNER.sha(path), bytes=path.stat().st_size)

    def corpus(self, compiler="c", binary="j", count=1):
        return dict(status="passed", compiler_mode="provided", compiler_path="compiler.dag",
                    compiler_sha256_start=compiler, compiler_sha256_end=compiler,
                    binary_sha256_start=binary, binary_sha256_end=binary,
                    observations=[{}] * count, commands=[dict(command=["joy", "run"], exit_code=1)])

    def fixture(self, numbers=(1, 2), target="aarch64-apple-darwin"):
        """Receipt wiring fixture only; tests do not represent compiler execution."""
        rows = []
        for number in numbers:
            prefix = f"repeat-{number}/"
            artifact = self.identity(prefix + "compiler.dag", b"fixture compiler")
            inventory = self.identity(prefix + "inventory.json", {"fixture": True})
            self.identity(prefix + "input/job.dag", b"fixture JOB1")
            flags = [part for key, value in RUNNER.PROFILE.items() for part in ("--" + key, str(value))]
            steps = []
            for generation in (2, 3):
                steps.append(self.identity(prefix + f"step{generation}.json", dict(
                    status="compiler-returned", published_kind="program", result_sha256=artifact["sha256"],
                    compiler_sha256=artifact["sha256"], compiler_sha256_end=artifact["sha256"],
                    binary_sha256="j", binary_sha256_end="j", inventory_sha256=inventory["sha256"],
                    host_flags=flags + ["--frames", "65536"])))
            comparison = dict(artifact_sha256=artifact["sha256"], source_sha256_set=["s"], options={}, limits={})
            fixed = self.identity(prefix + "fixed.json", dict(status="passed", fixed_point=comparison,
                                 job_checker_sha256_start="j", job_checker_sha256_end="j",
                                 inventory_checker_sha256_start="i", inventory_checker_sha256_end="i",
                                 steps=[dict(receipt_sha256=step["sha256"]) for step in steps]))
            corpora = {}
            for generation in (2, 3):
                for script, count in RUNNER.CORPORA.items():
                    name = f"c{generation}-{script.removesuffix('.py')}"
                    corpora[name] = self.identity(prefix + name + ".json", self.corpus(artifact["sha256"], count=count))
            rows.append(dict(number=number, status="passed", tools={"joy": "j", "inventory": "i"},
                             inventory=inventory, c1=artifact, c2=artifact, c3=artifact,
                             step2=steps[0], step3=steps[1], corpora=corpora, fixed_point=fixed, comparison=comparison))
        report = dict(schema=RUNNER.SCHEMA, status="passed", ci_origin=None, selected_repetitions=list(numbers),
                    target=target, host={"rust": f"host: {target}\nrelease: 1.95.0"},
                    profile=dict(RUNNER.PROFILE), repetitions=rows,
                    pins={name: "a" * 40 for name in RUNNER.REPOS}, rust_version="1.95.0", runner_sha256="r")
        self.refresh_files(report)
        return report

    def refresh_files(self, report):
        report["files"] = self.identity("files.json", RUNNER.evidence_files(self.root))

    def matrix_fixture(self, origin=None):
        reports, parent = [], self.root
        try:
            for target in RUNNER.TARGETS:
                for number in (1, 2):
                    self.root = parent / f"{target}-repeat-{number}"
                    report = self.fixture((number,), target)
                    report["ci_origin"] = copy.deepcopy(origin)
                    reports.append((self.root, report))
        finally:
            self.root = parent
        return reports

    def test_single_repeat_and_local_default_keep_distinct_numbered_evidence(self):
        for numbers in ((1,), (2,), (1, 2)):
            report = self.fixture(numbers)
            self.assertEqual(RUNNER.compare([(self.root, report)]), report["repetitions"][0]["c2"]["sha256"])
        for numbers in ([], [True], [0], [3], [2, 1], [1, 1], [1, 2, 3]):
            with self.subTest(numbers=numbers), self.assertRaisesRegex(ValueError, "selected repetitions"):
                RUNNER.selected({"selected_repetitions": numbers})

    def test_selected_repeat_cannot_relabel_first_repeat_or_reuse_its_evidence(self):
        report = self.fixture((1,))
        report["selected_repetitions"] = [2]
        with self.assertRaisesRegex(ValueError, "numbered repetitions"):
            RUNNER.compare([(self.root, report)])
        report["repetitions"][0]["number"] = 2
        with self.assertRaisesRegex(ValueError, "directory reused"):
            RUNNER.compare([(self.root, report)])

    def test_legacy_two_repeat_receipt_cannot_claim_current_schema_acceptance(self):
        report = self.fixture()
        report["schema"] = "trident/clean-bootstrap/v1"
        with self.assertRaisesRegex(ValueError, "current schema"):
            RUNNER.compare([(self.root, report)])

    def test_matrix_requires_exact_twelve_single_repeat_jobs(self):
        reports = self.matrix_fixture()
        expected = reports[0][1]["repetitions"][0]["c2"]["sha256"]
        self.assertEqual(RUNNER.compare_matrix(reports), expected)
        for changed in (reports[:-1], reports + reports[:1], reports[:-1] + reports[:1]):
            with self.assertRaisesRegex(ValueError, "platform.*evidence"):
                RUNNER.compare_matrix(changed)
        report = reports[-1][1]
        report["selected_repetitions"] = [1, 2]
        with self.assertRaisesRegex(ValueError, "one selected repetition"):
            RUNNER.compare_matrix(reports)

    def test_matrix_rejects_mixed_ci_runs_attempts_heads_and_missing_origin(self):
        origin = dict(run_id="123", run_attempt="2", head_sha="a" * 40)
        reports = self.matrix_fixture(origin)
        RUNNER.compare_matrix(reports, origin)
        last = reports[-1][1]
        for changed in (None, origin | {"run_id": "124"}, origin | {"run_attempt": "1"},
                        origin | {"head_sha": "b" * 40}):
            last["ci_origin"] = changed
            with self.subTest(changed=changed), self.assertRaises(ValueError):
                RUNNER.compare_matrix(reports, origin)
            with self.assertRaises(ValueError):
                RUNNER.compare_matrix(reports)
        last["ci_origin"] = origin
        with self.assertRaisesRegex(ValueError, "CI run origin"):
            RUNNER.compare_matrix(reports, origin | {"run_attempt": "3"})

    def test_matrix_rejects_changed_pins_profile_source_options_and_limits(self):
        reports = self.matrix_fixture()
        root, report = reports[-1]
        for key, value in (("pins", report["pins"] | {"joy": "b" * 40}),
                           ("profile", report["profile"] | {"time-ms": 3_600_000}),
                           ("runner_sha256", "other")):
            old = report[key]
            report[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                RUNNER.compare_matrix(reports)
            report[key] = old
        parent, self.root = self.root, root
        try:
            row = report["repetitions"][0]
            fixed = RUNNER.load(root / row["fixed_point"]["path"])
            for key, value in (("source_sha256_set", ["other"]), ("options", {"changed": 1}), ("limits", {"changed": 1})):
                changed = copy.deepcopy(fixed)
                changed["fixed_point"][key] = value
                row["comparison"] = changed["fixed_point"]
                row["fixed_point"] = self.identity(row["fixed_point"]["path"], changed)
                self.refresh_files(report)
                with self.subTest(key=key), self.assertRaisesRegex(ValueError, "source/options/limits differ"):
                    RUNNER.compare_matrix(reports)
        finally:
            self.root = parent

    def test_ci_origin_requires_exact_ids_and_binds_pinned_head(self):
        with patch.dict(RUNNER.os.environ, {}, clear=True):
            self.assertIsNone(RUNNER.ci_origin())
        env = dict(GITHUB_ACTIONS="true", GITHUB_RUN_ID="123", GITHUB_RUN_ATTEMPT="2", BOOTSTRAP_HEAD_SHA="a" * 40)
        with patch.dict(RUNNER.os.environ, env, clear=True):
            self.assertEqual(RUNNER.ci_origin(), dict(run_id="123", run_attempt="2", head_sha="a" * 40))
        for key, value in (("GITHUB_RUN_ID", ""), ("GITHUB_RUN_ATTEMPT", "0"), ("BOOTSTRAP_HEAD_SHA", "short")):
            with patch.dict(RUNNER.os.environ, env | {key: value}, clear=True), self.assertRaises(ValueError):
                RUNNER.ci_origin()
        report = self.fixture((2,))
        report["ci_origin"] = dict(run_id="123", run_attempt="2", head_sha="b" * 40)
        with self.assertRaisesRegex(ValueError, "CI head differs"):
            RUNNER.compare([(self.root, report)])

    def test_prepare_only_routes_explicit_second_repeat_without_build_fallback(self):
        output, work = self.root / "evidence", self.root / "work"
        args = [str(SCRIPT), "--target", "aarch64-apple-darwin", "--prepare-only", "--repeat", "2",
                "--pins-json", json.dumps({name: "a" * 40 for name in RUNNER.REPOS}),
                "--work", str(work), "--output", str(output)]
        with patch.object(sys, "argv", args), patch.dict(RUNNER.os.environ, {}, clear=True), \
             patch.object(RUNNER.Audit, "run", return_value="host: aarch64-apple-darwin\nrelease: 1.95.0"), \
             patch.object(RUNNER, "native"), patch.object(RUNNER, "repetition") as run:
            self.assertEqual(RUNNER.main(), 0)
        run.assert_called_once()
        self.assertEqual(run.call_args.args[1:3], (work.resolve() / "repeat-2", 2))
        receipt = RUNNER.load(output / "receipt.json")
        self.assertEqual(receipt["selected_repetitions"], [2])
        self.assertEqual(receipt["status"], "prepared")
        self.assertNotIn("compiler_sha256", receipt)

    def test_repeat_cli_rejects_invalid_number_without_creating_output(self):
        output = self.root / "evidence"
        result = subprocess.run([sys.executable, str(SCRIPT), "--repeat", "3", "--output", str(output)],
                                capture_output=True, check=False)
        self.assertEqual(result.returncode, 2)
        self.assertFalse(output.exists())

    def test_mismatched_ci_head_fails_before_work_or_build_and_retains_receipt(self):
        output, work = self.root / "evidence", self.root / "work"
        args = [str(SCRIPT), "--target", "aarch64-apple-darwin", "--repeat", "2",
                "--pins-json", json.dumps({name: "a" * 40 for name in RUNNER.REPOS}),
                "--work", str(work), "--output", str(output)]
        env = dict(GITHUB_ACTIONS="true", GITHUB_RUN_ID="123", GITHUB_RUN_ATTEMPT="2", BOOTSTRAP_HEAD_SHA="b" * 40)
        with patch.object(sys, "argv", args), patch.dict(RUNNER.os.environ, env, clear=True), \
             patch.object(RUNNER.Audit, "run") as command:
            self.assertEqual(RUNNER.main(), 1)
        command.assert_not_called()
        self.assertFalse(work.exists())
        receipt = RUNNER.load(output / "receipt.json")
        self.assertEqual(receipt["status"], "failed")
        self.assertIn("CI head differs", receipt["error"])

    def test_whole_compiler_routes_two_hour_ceiling_with_bounded_outer_timeout(self):
        audit = RUNNER.Audit(self.root / "evidence", {"repetitions": []})
        sources = self.root / "sources"
        helper = sources / "trident/audit/self-hosting/bootstrap-runner.py"
        helper.parent.mkdir(parents=True)
        helper.write_bytes(SCRIPT.read_bytes())
        evidence = audit.output / "repeat-2"
        evidence.mkdir()
        joy, checker, inventory, c1 = [evidence / name for name in ("joy", "checker", "inventory.json", "c1.dag")]
        for path in (joy, checker, inventory, c1):
            path.write_bytes(b"routing fixture only")
        with patch.object(RUNNER, "prepare", return_value=(sources, joy, checker, inventory, c1)), \
             patch.object(audit, "run", side_effect=RuntimeError("stop before guest execution")) as command:
            with self.assertRaisesRegex(RuntimeError, "stop before guest"):
                RUNNER.repetition(audit, self.root / "work", 2, "aarch64-apple-darwin", {})
        argv, cwd, timeout = command.call_args.args
        self.assertEqual(timeout, 7500)
        self.assertEqual(argv[argv.index("--time-ms") + 1], "7200000")
        self.assertEqual(argv[argv.index("--compiler") + 1], c1)
        self.assertEqual(cwd, sources / "trident")
        self.assertNotIn("status", audit.report["repetitions"][0])

    def test_workflow_uses_exact_cross_product_and_attempt_scoped_artifacts(self):
        workflow = SCRIPT.parents[2] / ".github/workflows/selfhost-bootstrap.yml"
        text = workflow.read_text(encoding="utf-8")
        matrix = text.split("      matrix:\n", 1)[1].split("    defaults:\n", 1)[0]
        self.assertNotIn("include:", matrix)
        self.assertIn("repeat: [1, 2]", matrix)
        platforms = re.findall(r"- target: (\S+)\n +runner: (\S+)\n +architecture: (\S+)", matrix)
        self.assertEqual(len(platforms), 6)
        self.assertEqual({t for t, _, _ in platforms}, set(RUNNER.TARGETS))
        for target, _, architecture in platforms:
            self.assertEqual(architecture, RUNNER.TARGETS[target][1])
        self.assertIn('--repeat "$env:BOOTSTRAP_REPEAT"', text)
        self.assertIn("name: bootstrap-${{ matrix.platform.target }}-repeat-${{ matrix.repeat }}-attempt-${{ github.run_attempt }}", text)
        self.assertIn("pattern: bootstrap-*-repeat-*-attempt-${{ github.run_attempt }}", text)
        self.assertIn("name: bootstrap-matrix-comparison-attempt-${{ github.run_attempt }}", text)
        self.assertIn("BOOTSTRAP_HEAD_SHA: ${{ github.event.pull_request.head.sha || github.sha }}", text)
        self.assertNotIn("overwrite: true", text)

    def test_pins_require_complete_exact_commit_identities(self):
        values = {name: "a" * 40 for name in RUNNER.REPOS}
        self.assertEqual(RUNNER.pins(json.dumps(values)), values)
        for change in ({"joy": "release/0.4"}, {"joy": "a" * 39}, {"extra": "a" * 40}):
            with self.assertRaises(ValueError):
                RUNNER.pins(json.dumps(values | change))
        del values["joy"]
        with self.assertRaises(ValueError):
            RUNNER.pins(json.dumps(values))

    @patch.object(RUNNER.platform, "system", return_value="Darwin")
    @patch.object(RUNNER.platform, "machine", return_value="arm64")
    def test_native_guard_rejects_cross_host_and_unpinned_rust(self, *_):
        RUNNER.native("aarch64-apple-darwin", "host: aarch64-apple-darwin\nrelease: 1.95.0", "1.95.0")
        for target, rust in [("x86_64-apple-darwin", "host: x86_64-apple-darwin\nrelease: 1.95.0"),
                             ("aarch64-apple-darwin", "host: x86_64-apple-darwin\nrelease: 1.95.0"),
                             ("aarch64-apple-darwin", "host: aarch64-apple-darwin\nrelease: 1.96.0")]:
            with self.assertRaises(ValueError):
                RUNNER.native(target, rust, "1.95.0")

    def test_negative_guest_cases_do_not_fail_a_passed_corpus(self):
        RUNNER.corpus_identity(self.corpus(), "c", "j", 1)

    def test_host_fallback_and_changed_tool_fail_closed(self):
        for change in ({"compiler_mode": "built"}, {"compiler_sha256_end": "other"},
                       {"binary_sha256_end": "other"}, {"observations": []}, {"status": "failed"}):
            with self.assertRaises(ValueError):
                RUNNER.corpus_identity(self.corpus() | change, "c", "j", 1)
        report = self.corpus()
        report["commands"] = [dict(command=["joy", "build", "source.tri", "--artifact-profile", "raw"])]
        with self.assertRaises(ValueError):
            RUNNER.corpus_identity(report, "c", "j", 1)
        report["commands"][0]["reference_only"] = True
        RUNNER.corpus_identity(report, "c", "j", 1)
        report["commands"][0]["command"][-1] = "compiler-job"
        with self.assertRaises(ValueError):
            RUNNER.corpus_identity(report, "c", "j", 1)

    def test_retained_artifact_escape_or_mutation_is_rejected(self):
        identity = self.identity("inside", b"original")
        RUNNER.retained(self.root, identity)
        (self.root / "inside").write_bytes(b"modified")
        with self.assertRaises(ValueError):
            RUNNER.retained(self.root, identity)
        with self.assertRaises(ValueError):
            RUNNER.retained(self.root, identity | {"path": "../outside"})

    def test_aggregate_binds_exact_retained_bytes_and_all_corpora(self):
        report = self.fixture()
        self.assertEqual(RUNNER.compare([(self.root, report)]), report["repetitions"][0]["c2"]["sha256"])
        for change in ({"status": "prepared"}, {"repetitions": report["repetitions"][:1]},
                       {"host": {"rust": "host: wrong\nrelease: 1.95.0"}}):
            with self.assertRaises(ValueError):
                RUNNER.compare([(self.root, report | change)])
        (self.root / "repeat-1/compiler.dag").write_bytes(b"different artifact")
        with self.assertRaises(ValueError):
            RUNNER.compare([(self.root, report)])

    def test_aggregate_rechecks_corpus_tool_binding(self):
        report = self.fixture()
        for row in report["repetitions"]:
            name = next(iter(row["corpora"]))
            identity = row["corpora"][name]
            content = RUNNER.load(self.root / identity["path"])
            content["binary_sha256_end"] = "changed"
            row["corpora"][name] = self.identity(identity["path"], content)
        self.refresh_files(report)
        with self.assertRaisesRegex(ValueError, "Joy changed"):
            RUNNER.compare([(self.root, report)])

    def test_aggregate_rejects_unbound_step_and_inconsistent_pins(self):
        report = self.fixture()
        changed = copy.deepcopy(report)
        changed["pins"]["joy"] = "b" * 40
        with self.assertRaisesRegex(ValueError, "inputs differ"):
            RUNNER.compare([(self.root, report), (self.root, changed)])
        row = report["repetitions"][0]
        step = RUNNER.load(self.root / row["step2"]["path"])
        step["compiler_sha256"] = "different seed"
        row["step2"] = self.identity(row["step2"]["path"], step)
        self.refresh_files(report)
        with self.assertRaisesRegex(ValueError, "chain binding"):
            RUNNER.compare([(self.root, report)])

    def test_duplicate_repeat_or_reused_directory_is_rejected(self):
        report = self.fixture()
        report["repetitions"][1] = copy.deepcopy(report["repetitions"][0])
        with self.assertRaisesRegex(ValueError, "numbered repetitions"):
            RUNNER.compare([(self.root, report)])
        report["repetitions"][1]["number"] = 2
        with self.assertRaisesRegex(ValueError, "directory reused"):
            RUNNER.compare([(self.root, report)])

    def test_deleted_or_changed_raw_job_cannot_leave_aggregate_green(self):
        report = self.fixture()
        job = self.root / "repeat-1/input/job.dag"
        job.write_bytes(b"changed JOB1")
        with self.assertRaisesRegex(ValueError, "raw evidence"):
            RUNNER.compare([(self.root, report)])
        job.unlink()
        with self.assertRaisesRegex(ValueError, "raw evidence"):
            RUNNER.compare([(self.root, report)])

    def test_matrix_with_missing_platform_fails_and_preserves_receipt(self):
        downloaded = self.root / "downloaded"
        downloaded.mkdir()
        output = self.root / "matrix"
        result = subprocess.run([sys.executable, str(SCRIPT), "--matrix", str(downloaded), "--output", str(output)],
                                capture_output=True, check=False)
        self.assertEqual(result.returncode, 1)
        receipt = RUNNER.load(output / "receipt.json")
        self.assertEqual(receipt["status"], "failed")
        self.assertNotIn("compiler_sha256", receipt)
        self.assertIn("platform evidence", receipt["error"])

    def test_failed_command_keeps_logs_and_durable_exit_status(self):
        audit = RUNNER.Audit(self.root / "evidence", {})
        with self.assertRaises(ValueError):
            audit.run([sys.executable, "-c", "import sys; print('output'); print('failure', file=sys.stderr); sys.exit(3)"], self.root)
        receipt = RUNNER.load(audit.output / "receipt.json")
        self.assertEqual(receipt["commands"][0]["exit_code"], 3)
        self.assertIn("failure", (audit.output / "commands/0.stderr").read_text())

    def test_command_timeout_keeps_interruption_evidence(self):
        audit = RUNNER.Audit(self.root / "evidence", {})
        with self.assertRaises(subprocess.TimeoutExpired):
            audit.run([sys.executable, "-c", "import time; time.sleep(60)"], self.root, timeout=0.05)
        self.assertEqual(RUNNER.load(audit.output / "receipt.json")["commands"][0]["status"], "interrupted")

    def test_unavailable_executable_is_recorded_as_launch_failure(self):
        audit = RUNNER.Audit(self.root / "evidence", {})
        with self.assertRaises(OSError):
            audit.run([self.root / "nonexistent-executable"], self.root)
        row = RUNNER.load(audit.output / "receipt.json")["commands"][0]
        self.assertEqual(row["status"], "launch-failed")
        self.assertIsNone(row["exit_code"])

    def test_child_adapter_retains_original_corpus_temporary_files(self):
        script = self.root / "run-native-compiler.py"
        script.write_text("import tempfile\nfrom pathlib import Path\nwith tempfile.TemporaryDirectory() as d:\n"
                          "    Path(d, 'retained').write_bytes(b'evidence')\n", encoding="utf-8")
        retained = self.root / "retained"
        retained.mkdir()
        result = subprocess.run([sys.executable, str(SCRIPT), "--corpus-child", str(script), "--retain", str(retained),
                                 "--joy", "joy", "--compiler", "compiler", "--output", str(self.root / "output")],
                                capture_output=True, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([p.read_bytes() for p in retained.glob("*/retained")], [b"evidence"])

    def test_optimized_python_cannot_disable_corpus_assertions(self):
        with patch.dict(RUNNER.os.environ, {"PYTHONOPTIMIZE": "1"}):
            audit = RUNNER.Audit(self.root / "evidence", {})
        self.assertNotIn("PYTHONOPTIMIZE", audit.env)
        result = subprocess.run([sys.executable, "-O", str(SCRIPT), "--corpus-child", "run-native-compiler.py"],
                                capture_output=True, check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"unoptimized Python", result.stderr)


if __name__ == "__main__":
    unittest.main()
