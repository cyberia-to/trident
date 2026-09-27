"""Synthetic receipt-consistency tests; no compiler/self-host acceptance claim."""
import contextlib
import copy
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("fixed_point", HERE / "check-selfhost-fixed-point.py")
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)
SOURCE = b"program native_compiler fn main()->Field{7}\n"


def sha(data):
    return hashlib.sha256(data).hexdigest()


class FixedPointReceipts(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="fixed-point-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        vectors = json.loads((HERE.parents[2] / "joy/cli/tests/compiler_vectors.json").read_text())["files"]
        self.c1 = bytes.fromhex(vectors["compiler"])
        self.c2 = bytes.fromhex(vectors["diagnostic-compiler"])
        self.assertNotEqual(self.c1, self.c2)
        self.job = bytes.fromhex(vectors["job"])
        self.binary, self.checker = self.root / "joy", self.root / "selfhost_inventory"
        self.binary.write_bytes(b"mock Joy identity; never executed")
        self.checker.write_bytes(b"mock inventory identity")
        self.compiler = self.root / "c1.dag"
        self.compiler.write_bytes(self.c1)
        self.inventory = self.root / "inventory.json"
        self.inventory.write_text(json.dumps(dict(schema=1, roots=["native_compiler"],
            source_bytes=len(SOURCE), module_count=1,
            modules={"native_compiler": dict(path="compiler/nox/main.tri", source_bytes=len(SOURCE))})))
        self.first = self.make_step("first", self.compiler)
        self.second = self.make_step("second", self.root / "first/result.dag")
        self.output = self.root / "check.json"
        self.calls = []

    def make_step(self, name, compiler):
        directory = self.root / name
        directory.mkdir()
        (directory / "0.tri").write_bytes(SOURCE)
        (directory / "job.dag").write_bytes(self.job)
        (directory / "result.dag").write_bytes(self.c2)
        limits = dict(source_bytes=len(SOURCE), modules=128, diagnostics=16, sequence_length=65536,
                      validation_visits=1000000, artifact_bytes=16777216, artifact_nodes=196608,
                      artifact_depth=4096, reductions=100000000, arena_nodes=3145728, evaluator_frames=65536)
        options = dict(target=0, input_profile=1, output_profile=1, optimization=0, cfg_flags=[])
        manifest = dict(version=1, entry_module="native_compiler", entry_function="main",
                        modules=[dict(logical_path="native_compiler", file="0.tri",
                                      origin_name="native-compiler", origin_version="1")], options=options, limits=limits)
        (directory / "package.json").write_text(json.dumps(manifest))
        modules = [dict(logical_path="native_compiler", origin_name="native-compiler", origin_version="1",
                        source_bytes=len(SOURCE), particle="ab" * 32, source_particle="cd" * 32)]
        package = dict(compiler_particle=compiler.read_bytes()[8:40].hex(), job_particle=self.job[8:40].hex(),
                       package_particle="ef" * 32, modules=modules, options=options, limits=limits,
                       entry_module="native_compiler", entry_function="main")
        compiled = {key: copy.deepcopy(value) for key, value in package.items()
                    if key not in ("compiler_particle", "job_particle")}
        compiled.update(status="success", diagnostics=[], compiled_particle=self.c2[8:40].hex())
        admission = dict(schema="joy/job-pack/v1", ok=True, artifact=str(directory / "job.dag"), package=package)
        execution = dict(schema="joy/artifact-run/v1", ok=True, artifact=str(directory / "result.dag"),
                         published_particle=self.c2[8:40].hex(), execution=dict(compiler_job=compiled, trace_mode="none", charged_reductions=1,
                         program_particle=package["compiler_particle"], input_particle=package["job_particle"]))
        host = ["--arena-nodes", "3145728", "--budget", "100000000", "--frames", "65536", "--time-ms", "300000"]
        commands = [
            ["cargo", "run", "--release", "--locked", "--offline", "--example", "selfhost_inventory", "--",
             "--root", ".", "--entry", "compiler/nox/main.tri", "--output", str(self.inventory), "--check"],
            [str(self.binary), "pack-job", "--compiler", str(compiler), "--manifest", str(directory / "package.json"),
             "-o", str(directory / "job.dag"), *host],
            [str(self.binary), "run-artifact", str(compiler), "--input", str(directory / "job.dag"), "--emit", "program",
             "-o", str(directory / "result.dag"), *host],
        ]
        receipt = dict(schema="trident/native-closure-probe/v1", status="compiler-returned", published_kind="program",
                       artifact_directory=str(directory), binary=str(self.binary), binary_sha256=sha(self.binary.read_bytes()),
                       binary_sha256_end=sha(self.binary.read_bytes()), compiler=str(compiler),
                       compiler_sha256=sha(compiler.read_bytes()), compiler_sha256_end=sha(compiler.read_bytes()),
                       inventory=str(self.inventory), inventory_sha256=sha(self.inventory.read_bytes()),
                       sources={"native_compiler": dict(path="compiler/nox/main.tri", source_bytes=len(SOURCE),
                       sha256=sha(SOURCE), copy=str(directory / "0.tri"))}, source_bytes=len(SOURCE), module_count=1,
                       manifest=manifest, host_flags=host, admission=admission, execution=execution,
                       job_sha256=sha(self.job), job_dag_entries=int.from_bytes(self.job[40:44], "little"),
                       result_sha256=sha(self.c2),
                       commands=[dict(command=c, exit_code=0, stdout="", stderr="") for c in commands])
        path = self.root / f"{name}.json"
        self.save(path, receipt)
        return path

    def save(self, path, receipt):
        receipt["commands"][-2]["stdout"] = json.dumps(receipt["admission"])
        receipt["commands"][-1]["stdout"] = json.dumps(receipt["execution"])
        Path(receipt["artifact_directory"], "package.json").write_text(json.dumps(receipt["manifest"]))
        path.write_text(json.dumps(receipt))

    def metadata(self, command, **kwargs):
        self.calls.append(command)
        self.assertEqual(command[0], str(self.checker))
        self.assertEqual(command[-1], "--check")
        self.assertNotIn("cargo", command)
        self.assertNotIn("build", command)
        tree = Path(command[command.index("--root") + 1])
        self.assertEqual((tree / "compiler/nox/main.tri").read_bytes(), SOURCE)
        self.assertEqual(command[command.index("--output") + 1], str(self.inventory))
        return subprocess.CompletedProcess(command, 0, "inventory checked\n", "")

    def run_check(self, expected=0, metadata=None):
        args = [str(HERE / "check-selfhost-fixed-point.py"), "--first", str(self.first),
                "--second", str(self.second), "--inventory-checker", str(self.checker), "--output", str(self.output)]
        with patch.object(sys, "argv", args), patch.object(CHECK.subprocess, "run", metadata or self.metadata), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(CHECK.main(), expected)
        return json.loads(self.output.read_text())

    def reject(self, message):
        result = self.run_check(1)
        self.assertEqual(result["status"], "rejected")
        self.assertIn(message, result["error"]["message"])
        self.assertNotIn("fixed_point", result)

    def test_consistent_chain_checks_both_snapshots_without_compiler_execution(self):
        before = self.compiler.read_bytes()
        result = self.run_check()
        self.assertEqual(result["status"], "passed")
        self.assertEqual(len(self.calls), 2)
        self.assertEqual(result["fixed_point"]["artifact_sha256"], sha(self.c2))
        self.assertEqual(result["fixed_point"]["particle"], self.c2[8:40].hex())
        self.assertEqual(result["inventory_checker_sha256_start"], result["inventory_checker_sha256_end"])
        self.assertIn("full self-host acceptance not established", result["scope"])
        self.assertEqual(self.compiler.read_bytes(), before)

    def test_changed_emitted_file_is_rejected(self):
        (self.root / "first/result.dag").write_bytes(self.c2 + b"changed")
        self.reject("emitted ART1 bytes changed")

    def test_source_sha_and_snapshot_inventory_both_bind_actual_bytes(self):
        (self.root / "first/0.tri").write_bytes(SOURCE.replace(b"7", b"8"))
        self.reject("snapshot source SHA256")

    def test_inventory_checker_failure_cannot_be_replaced_by_declared_hashes(self):
        changed = SOURCE.replace(b"7", b"8")
        (self.root / "first/0.tri").write_bytes(changed)
        receipt = json.loads(self.first.read_text())
        receipt["sources"]["native_compiler"]["sha256"] = sha(changed)
        self.save(self.first, receipt)
        result = self.run_check(1, lambda command, **kwargs: subprocess.CompletedProcess(command, 1, "", "inventory stale"))
        self.assertEqual(result["error"]["message"], "snapshot inventory check failed")
        self.assertEqual(result["inventory_checks"][0]["exit_code"], 1)
        self.assertNotIn("fixed_point", result)

    def test_metadata_checker_failure_retains_attempted_command(self):
        def timeout(command, **kwargs):
            raise subprocess.TimeoutExpired(command, 60)
        result = self.run_check(1, timeout)
        self.assertEqual(result["error"]["kind"], "TimeoutExpired")
        self.assertEqual(result["inventory_checks"][0]["command"][0], str(self.checker))
        self.assertIsNone(result["inventory_checks"][0]["exit_code"])

    def test_snapshot_path_cannot_escape_temporary_tree(self):
        receipt = json.loads(self.first.read_text())
        receipt["sources"]["native_compiler"]["path"] = "../escape.tri"
        self.save(self.first, receipt)
        self.reject("canonical relative source path")
        self.assertEqual(self.calls, [])

    def test_second_compiler_must_be_first_emitted_artifact(self):
        receipt = json.loads(self.second.read_text())
        receipt.update(compiler=str(self.compiler), compiler_sha256=sha(self.c1), compiler_sha256_end=sha(self.c1))
        receipt["admission"]["package"]["compiler_particle"] = self.c1[8:40].hex()
        receipt["execution"]["execution"]["program_particle"] = self.c1[8:40].hex()
        receipt["commands"][-2]["command"][3] = str(self.compiler)
        receipt["commands"][-1]["command"][2] = str(self.compiler)
        self.save(self.second, receipt)
        self.reject("second compiler is not the first emitted C2")

    def test_different_c3_cannot_pass_particle_or_byte_comparison(self):
        receipt = json.loads(self.second.read_text())
        (self.root / "second/result.dag").write_bytes(self.c1)
        receipt["result_sha256"] = sha(self.c1)
        receipt["execution"]["published_particle"] = self.c1[8:40].hex()
        receipt["execution"]["execution"]["compiler_job"]["compiled_particle"] = self.c1[8:40].hex()
        self.save(self.second, receipt)
        self.reject("C2 and C3 artifact bytes differ")

    def test_options_and_limits_must_match_between_steps(self):
        original = json.loads(self.second.read_text())
        for key, field, value in [("options", "cfg_flags", ["explicit"]), ("limits", "arena_nodes", 1000000)]:
            with self.subTest(key=key):
                self.output = self.root / f"{key}-check.json"
                receipt = copy.deepcopy(original)
                for owner in [receipt["manifest"], receipt["admission"]["package"], receipt["execution"]["execution"]["compiler_job"]]:
                    owner[key][field] = value
                self.save(self.second, receipt)
                self.reject(f"steps differ: {key}")

    def test_start_end_hashes_and_publication_kind_are_required(self):
        original = json.loads(self.first.read_text())
        for key, value, error in [("binary_sha256_end", "00" * 32, "Joy binary start/end"),
                                  ("compiler_sha256_end", "00" * 32, "compiler start/end"),
                                  ("published_kind", "result", "program publication required"),
                                  ("status", "runtime-rejected", "program publication required")]:
            with self.subTest(key=key):
                self.output = self.root / f"{key}-check.json"
                receipt = copy.deepcopy(original)
                receipt[key] = value
                self.save(self.first, receipt)
                self.reject(error)

    def test_published_particle_must_identify_actual_emitted_artifact(self):
        receipt = json.loads(self.first.read_text())
        receipt["execution"]["published_particle"] = "00" * 32
        self.save(self.first, receipt)
        self.reject("published/compiler-returned ART1 identity")

    def test_successful_gas_and_trace_mode_are_consistent(self):
        original = json.loads(self.first.read_text())
        for field, value, error in [("trace_mode", "rows", "NoTrace"),
                                    ("charged_reductions", 0, "positive and within LIM1"),
                                    ("charged_reductions", 100000001, "positive and within LIM1")]:
            with self.subTest(field=field, value=value):
                self.output = self.root / f"{field}-{value}-check.json"
                receipt = copy.deepcopy(original)
                receipt["execution"]["execution"][field] = value
                self.save(self.first, receipt)
                self.reject(error)

    def test_header_guard_rejects_impossible_declared_dag_size(self):
        receipt = json.loads(self.first.read_text())
        data = self.c2[:40] + (196608).to_bytes(4, "little") + self.c2[44:]
        (self.root / "first/result.dag").write_bytes(data)
        receipt["result_sha256"] = sha(data)
        self.save(self.first, receipt)
        self.reject("NOXDAG minimum framing")

    def test_existing_output_is_preserved_without_any_checker_call(self):
        self.output.write_bytes(b"existing evidence")
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            self.run_check()
        self.assertEqual(self.output.read_bytes(), b"existing evidence")
        self.assertEqual(self.calls, [])


if __name__ == "__main__":
    unittest.main()
