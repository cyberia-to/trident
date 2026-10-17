"""Routing guards only; these tests do not claim compiler-corpus acceptance."""
import contextlib
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
sys.path.insert(0, str(HERE))
SPEC = importlib.util.spec_from_file_location("native_compiler_runner", HERE / "run-native-compiler.py")
RUNNER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RUNNER)


class CompilerSelection(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="compiler-routing-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.joy = self.root / "joy"
        self.joy.write_bytes(b"mock installed Joy")
        self.compiler = self.root / "provided.dag"
        # Mock bytes suffice: production delegates canonical/profile admission
        # to Joy, and the rejection test verifies that no fallback follows it.
        self.compiler.write_bytes(b"NOXDAG01" + bytes(range(32)) + bytes(4))
        self.sha = hashlib.sha256(self.compiler.read_bytes()).hexdigest()
        self.particle = self.compiler.read_bytes()[8:40].hex()
        self.output = self.root / "receipt.json"
        self.commands = []

    def invoke(self, action, provided=True, output=None, compiler=None):
        args = [str(HERE / "run-native-compiler.py"), "--joy", str(self.joy),
                "--output", str(output or self.output)]
        if provided:
            args += ["--compiler", str(compiler or self.compiler)]

        def run(command, **kwargs):
            self.commands.append(command)
            return action(command)

        with patch.object(sys, "argv", args), patch.object(RUNNER.subprocess, "run", run):
            RUNNER.main()

    def result(self, command, value=None, error=None):
        return subprocess.CompletedProcess(command, int(error is not None),
                                           json.dumps(value) if value is not None else "", error or "")

    def read_receipt(self):
        return json.loads(self.output.read_text())

    def test_provided_routes_pack_and_execution_without_build(self):
        def action(command):
            self.assertNotIn("build", command)
            if command[1] == "pack-job":
                self.assertEqual(command[command.index("--compiler") + 1], str(self.compiler))
                return self.result(command, {"package": {"compiler_particle": self.particle}})
            self.assertEqual(command[1:3], ["run-artifact", str(self.compiler)])
            return self.result(command, error="short probe stop")

        with self.assertRaisesRegex(AssertionError, "short probe stop"):
            self.invoke(action)
        receipt = self.read_receipt()
        self.assertEqual([c[1] for c in self.commands], ["pack-job", "run-artifact"])
        self.assertEqual(receipt["compiler_mode"], "provided")
        self.assertEqual(receipt["compiler_path"], str(self.compiler))
        self.assertEqual(receipt["compiler_sha256_start"], self.sha)
        self.assertEqual(receipt["compiler_sha256_end"], self.sha)
        self.assertEqual(receipt["status"], "command_failed")

    def test_profile_rejection_has_no_seed_fallback(self):
        with self.assertRaisesRegex(AssertionError, "requires compiler profile"):
            self.invoke(lambda c: self.result(c, error="pack-job requires compiler profile(1,1)"))
        self.assertEqual([c[1] for c in self.commands], ["pack-job"])
        self.assertEqual(self.compiler.read_bytes()[8:40].hex(), self.particle)
        self.assertEqual(self.read_receipt()["compiler_sha256_end"], self.sha)

    def test_seed_mode_retains_single_source_build(self):
        with self.assertRaisesRegex(AssertionError, "seed probe stop"):
            self.invoke(lambda c: self.result(c, error="seed probe stop"), provided=False)
        self.assertEqual([c[1] for c in self.commands], ["build"])
        self.assertEqual(self.read_receipt()["compiler_mode"], "seed-build")

    def test_missing_compiler_fails_before_commands(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as error:
            self.invoke(lambda c: self.fail(c), compiler=self.root / "missing.dag")
        self.assertEqual(error.exception.code, 2)
        self.assertEqual(self.commands, [])

    def test_receipt_cannot_alias_compiler(self):
        alias = self.root / "hardlink.dag"
        alias.hardlink_to(self.compiler)
        for output in [self.compiler, alias]:
            with self.subTest(output=output), contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                self.invoke(lambda c: self.fail(c), output=output)
        self.assertEqual(hashlib.sha256(self.compiler.read_bytes()).hexdigest(), self.sha)
        self.assertEqual(self.commands, [])

    def test_receipt_cannot_alias_joy_in_either_mode(self):
        alias = self.root / "joy-hardlink"
        alias.hardlink_to(self.joy)
        for output in [self.joy, alias]:
            for provided in [False, True]:
                with self.subTest(output=output, provided=provided), contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                    self.invoke(lambda c: self.fail(c), output=output, provided=provided)
        self.assertEqual(self.joy.read_bytes(), b"mock installed Joy")
        self.assertEqual(self.commands, [])

    def test_existing_receipt_is_preserved_in_either_mode(self):
        self.output.write_bytes(b"prior evidence")
        for provided in [False, True]:
            with self.subTest(provided=provided), contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                self.invoke(lambda c: self.fail(c), provided=provided)
        self.assertEqual(self.output.read_bytes(), b"prior evidence")
        self.assertEqual(self.commands, [])

    def test_mutated_compiler_records_both_identities_and_stops(self):
        def mutate(command):
            self.compiler.write_bytes(b"changed")
            return self.result(command, {"package": {"compiler_particle": self.particle}})

        with self.assertRaisesRegex(AssertionError, "compiler changed during command"):
            self.invoke(mutate)
        receipt = self.read_receipt()
        self.assertEqual(receipt["status"], "compiler_changed")
        self.assertEqual(receipt["compiler_sha256_start"], self.sha)
        self.assertNotEqual(receipt["compiler_sha256_start"], receipt["compiler_sha256_end"])
        self.assertEqual(len(self.commands), 1)

    def test_pack_and_execution_must_match_compiler_particle(self):
        for mismatch in ["pack-job", "run-artifact"]:
            with self.subTest(mismatch=mismatch):
                self.commands.clear()
                self.output = self.root / f"receipt-{mismatch}.json"

                def action(command):
                    particle = "wrong" if command[1] == mismatch else self.particle
                    if command[1] == "pack-job":
                        return self.result(command, {"package": {"compiler_particle": particle}})
                    return self.result(command, {"execution": {"program_particle": particle}})

                with self.assertRaisesRegex(AssertionError, "compiler identity"):
                    self.invoke(action)
                self.assertNotIn("build", [c[1] for c in self.commands])
                self.assertNotEqual(self.read_receipt()["status"], "passed")


if __name__ == "__main__":
    unittest.main()
