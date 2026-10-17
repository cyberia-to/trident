"""Mock routing tests; no guest compilation or C2 acceptance claim."""
import argparse
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
from native_compiler_selection import CompilerSelection


def load(name):
    spec = importlib.util.spec_from_file_location(name, HERE / (name + '.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


IMPORTS = load('check-guest-constant-linking')
GENERATED = load('check-generated-compiler-profile')


class CompilerRouting(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='compiler-selection-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.joy, self.compiler = self.root / 'joy', self.root / 'compiler.dag'
        self.joy.write_bytes(b'pinned mock Joy')
        self.git = self.root / 'git'
        self.git.write_bytes(b'pinned mock Git')
        self.git.chmod(0o700)
        self.vectors = json.loads((HERE.parents[2] / 'joy/cli/tests/compiler_vectors.json').read_text())['files']
        self.compiler.write_bytes(bytes.fromhex(self.vectors['compiler']))
        self.original = self.compiler.read_bytes()
        self.output = self.root / 'receipt.json'
        self.commands = []

    def invoke(self, module, action, provided=True, provider=None, output=None, compiler=None):
        args = [str(HERE / (module.__name__ + '.py')), '--joy', str(self.joy),
                '--output', str(output or self.output)]
        if provided:
            args += ['--compiler', str(compiler or self.compiler)]
        if module is GENERATED:
            args += ['--git', str(self.git)]

        def dispatch(command, **kwargs):
            self.commands.append(command)
            return action(command)

        with patch.object(sys, 'argv', args), patch.object(module.subprocess, 'run', dispatch), contextlib.redirect_stdout(io.StringIO()):
            if module is IMPORTS:
                module.main(provider or self.one_case)
            else:
                module.main()

    @staticmethod
    def one_case():
        yield dict(case='routing-only', sources={'sample': 'program sample fn main()->Field{14}'}, value=14)

    def result(self, command, value=None, error=None):
        return subprocess.CompletedProcess(command, int(error is not None),
                                           json.dumps(value) if value is not None else '', error or '')

    def complete_case(self, command):
        kind = command[1]
        output = Path(command[command.index('-o') + 1])
        if kind == 'build':
            key = 'compiler' if command[command.index('--artifact-profile') + 1] == 'compiler-job' else 'generated'
            output.write_bytes(bytes.fromhex(self.vectors[key]))
            return self.result(command)
        if kind == 'pack-job':
            compiler = Path(command[command.index('--compiler') + 1])
            output.write_bytes(bytes.fromhex(self.vectors['job']))
            return self.result(command, {'package': {'compiler_particle': compiler.read_bytes()[8:40].hex(),
                'job_particle': output.read_bytes()[8:40].hex()}})
        artifact = Path(command[2])
        input_file = Path(command[command.index('--input') + 1])
        if '--emit' in command:
            output.write_bytes(bytes.fromhex(self.vectors['generated']))
            return self.result(command, {'execution': {'program_particle': artifact.read_bytes()[8:40].hex(),
                'input_particle': input_file.read_bytes()[8:40].hex(), 'compiler_job': {'status': 'success'}}})
        output.write_bytes(bytes.fromhex(self.vectors['fourteen']))
        return self.result(command, {'execution': {'program_particle': artifact.read_bytes()[8:40].hex(),
            'input_particle': input_file.read_bytes()[8:40].hex()}})

    def test_supplied_compiler_keeps_independent_raw_oracle_and_complete_output_comparison(self):
        self.invoke(IMPORTS, self.complete_case)
        receipt = json.loads(self.output.read_text())
        self.assertEqual(receipt['status'], 'passed')
        self.assertEqual(receipt['compiler_mode'], 'provided')
        self.assertEqual(receipt['compiler_sha256_start'], receipt['compiler_sha256_end'])
        self.assertTrue(receipt['observations'][0]['complete_seed_output_equal'])
        builds = [row for row in receipt['commands'] if row['command'][1] == 'build']
        self.assertEqual(len(builds), 1)
        self.assertTrue(builds[0]['reference_only'])
        self.assertEqual(builds[0]['command'][builds[0]['command'].index('--artifact-profile') + 1], 'raw')
        self.assertEqual(self.commands[0][1], 'pack-job')
        self.assertEqual(self.commands[1][1:3], ['run-artifact', str(self.compiler)])
        self.assertEqual(self.compiler.read_bytes(), self.original)

    def test_default_import_mode_keeps_one_seed_build_and_independent_oracle(self):
        self.invoke(IMPORTS, self.complete_case, provided=False)
        receipt = json.loads(self.output.read_text())
        self.assertEqual(receipt['compiler_mode'], 'seed-build')
        builds = [row for row in receipt['commands'] if row['command'][1] == 'build']
        self.assertEqual([row['reference_only'] for row in builds], [False, True])
        self.assertTrue(receipt['observations'][0]['complete_seed_output_equal'])

    def test_all_shared_case_providers_route_first_job_to_supplied_compiler(self):
        for name in ['check-guest-constant-linking', 'check-guest-function-imports',
                     'check-guest-type-imports', 'check-guest-intrinsics']:
            with self.subTest(provider=name):
                self.commands.clear()
                self.output = self.root / (name + '.json')
                provider = load(name).cases
                with self.assertRaisesRegex(AssertionError, 'requires compiler profile'):
                    self.invoke(IMPORTS, lambda c: self.result(c, error='pack-job requires compiler profile(1,1)'), provider=provider)
                self.assertEqual([c[1] for c in self.commands], ['pack-job'])
                command = self.commands[0]
                self.assertEqual(command[command.index('--compiler') + 1], str(self.compiler))
                self.assertEqual(json.loads(self.output.read_text())['status'], 'failed')

    def test_missing_compiler_and_existing_or_alias_receipts_fail_before_commands(self):
        alias = self.root / 'compiler-link.dag'
        alias.hardlink_to(self.compiler)
        existing = self.root / 'existing.json'
        existing.write_bytes(b'prior evidence')
        for module in [IMPORTS, GENERATED]:
            for kwargs in [dict(compiler=self.root / 'missing.dag'), dict(output=self.compiler),
                           dict(output=alias), dict(output=self.joy), dict(output=existing)]:
                with self.subTest(module=module.__name__, arguments=kwargs), contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                    self.invoke(module, lambda c: self.fail(c), **kwargs)
        self.assertEqual(self.commands, [])
        self.assertEqual(existing.read_bytes(), b'prior evidence')
        self.assertEqual(self.compiler.read_bytes(), self.original)

    def test_mutation_during_first_pack_stops_without_fallback(self):
        def mutate(command):
            self.compiler.write_bytes(b'changed')
            return self.result(command)
        with self.assertRaisesRegex(AssertionError, 'compiler changed'):
            self.invoke(IMPORTS, mutate)
        receipt = json.loads(self.output.read_text())
        self.assertEqual(receipt['status'], 'failed')
        self.assertNotEqual(receipt['compiler_sha256_start'], receipt['compiler_sha256_end'])
        self.assertEqual([c[1] for c in self.commands], ['pack-job'])

    def test_pack_and_execution_particles_bind_actual_artifact(self):
        for bad, owner, key in [('pack-job', 'package', 'compiler_particle'),
                                ('pack-job', 'package', 'job_particle'),
                                ('run-artifact', 'execution', 'program_particle'),
                                ('run-artifact', 'execution', 'input_particle')]:
            self.output = self.root / (key + '.json')
            self.commands.clear()
            def action(command):
                result = self.complete_case(command)
                if command[1] == bad:
                    value = json.loads(result.stdout)
                    value[owner][key] = '00' * 32
                    result.stdout = json.dumps(value)
                return result
            with self.subTest(command=bad), self.assertRaisesRegex(AssertionError, 'identity'):
                self.invoke(IMPORTS, action)
            self.assertNotIn('build', [c[1] for c in self.commands])

    def test_provided_mode_forbids_seed_build_and_oracle_input_replacement(self):
        args = argparse.Namespace(joy=self.joy, compiler=self.compiler, output=self.output)
        selection = CompilerSelection(args, argparse.ArgumentParser())
        self.addCleanup(selection.close)
        raw = ['build', self.root / 'source.tri', '--artifact-profile', 'raw', '-o', self.root / 'oracle.dag']
        with self.assertRaisesRegex(AssertionError, 'cannot build a compiler'):
            selection.before(raw)
        selection.before(raw, reference_only=True)
        for change in [raw[:-1] + [self.compiler], raw[:3] + ['compiler-job'] + raw[4:]]:
            with self.assertRaises(AssertionError):
                selection.before(change, reference_only=True)

    def test_concurrent_receipt_creation_is_exclusive_and_preserves_compiler(self):
        original_open = Path.open
        def race(path, mode='r', *args, **kwargs):
            if path == self.output and mode == 'x':
                path.symlink_to(self.compiler)
            return original_open(path, mode, *args, **kwargs)
        with patch.object(Path, 'open', race), contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            self.invoke(IMPORTS, lambda command: self.fail(command))
        self.assertEqual(self.commands, [])
        self.assertEqual(self.compiler.read_bytes(), self.original)

    def test_receipt_updates_keep_owned_inode_after_path_is_replaced(self):
        args = argparse.Namespace(joy=self.joy, compiler=self.compiler, output=self.output)
        selection = CompilerSelection(args, argparse.ArgumentParser())
        self.addCleanup(selection.close)
        self.output.unlink()
        self.output.symlink_to(self.compiler)
        selection.write_report({'status': 'failed', 'reason': 'routing test'})
        self.assertEqual(self.compiler.read_bytes(), self.original)

    def test_semantic_mismatch_records_failure_after_successful_commands(self):
        def action(command):
            result = self.complete_case(command)
            if command[1] == 'run-artifact' and '--emit' not in command:
                Path(command[command.index('-o') + 1]).write_bytes(bytes.fromhex(self.vectors['zero']))
            return result
        with self.assertRaises(AssertionError):
            self.invoke(IMPORTS, action)
        receipt = json.loads(self.output.read_text())
        self.assertEqual(receipt['status'], 'failed')
        self.assertEqual(receipt['failure']['kind'], 'AssertionError')
        self.assertTrue(all(row['exit_code'] == 0 for row in receipt['commands']))
        self.assertEqual(receipt['compiler_sha256_start'], receipt['compiler_sha256_end'])

    def test_final_binary_mutation_records_failure_outside_command_execution(self):
        original_read = Path.read_bytes
        def read(path):
            value = original_read(path)
            if path.name == 'program.dag' and len(self.commands) == 5:
                self.joy.write_bytes(b'changed after final command')
            return value
        with patch.object(Path, 'read_bytes', read), self.assertRaisesRegex(AssertionError, 'installed Joy changed'):
            self.invoke(IMPORTS, self.complete_case)
        receipt = json.loads(self.output.read_text())
        self.assertEqual(receipt['status'], 'failed')
        self.assertIn('installed Joy changed', receipt['failure']['message'])
        self.assertEqual(len(receipt['commands']), 5)
        self.assertTrue(all(row['exit_code'] == 0 for row in receipt['commands']))
        self.assertNotEqual(receipt['binary_sha256_start'], receipt['binary_sha256_end'])

    def test_generated_profile_routes_first_compile_without_seed_build(self):
        def action(command):
            if command[0] == str(self.git):
                return subprocess.CompletedProcess(command, 0, 'mock metadata\n', '')
            if command[1] == 'pack-job':
                return self.complete_case(command)
            self.assertEqual(command[1:3], ['run-artifact', str(self.compiler)])
            return self.result(command, error='intentional prefix stop')
        with self.assertRaises(SystemExit) as stopped:
            self.invoke(GENERATED, action)
        self.assertEqual(stopped.exception.code, 1)
        self.assertNotIn('build', [c[1] for c in self.commands])
        receipt = json.loads(self.output.read_text())
        self.assertEqual(receipt['compiler_mode'], 'provided')
        self.assertEqual(receipt['status'], 'failed')
        self.assertEqual([c[1] for c in self.commands if c[0] != str(self.git)], ['pack-job', 'run-artifact'])

    def test_generated_profile_rejects_raw_input_without_fallback(self):
        self.compiler.write_bytes(bytes.fromhex(self.vectors['generated']))
        with self.assertRaises(SystemExit):
            self.invoke(GENERATED, lambda c: subprocess.CompletedProcess(c, 0, 'metadata\n', ''))
        self.assertEqual([c[0] for c in self.commands], [str(self.git), str(self.git)])
        receipt = json.loads(self.output.read_text())
        self.assertIn('ART1 profile', receipt['failure'])
        self.assertEqual(receipt['status'], 'failed')

    def test_generated_default_mode_retains_one_compiler_build(self):
        def action(command):
            if command[0] == str(self.git):
                return subprocess.CompletedProcess(command, 0, 'metadata\n', '')
            self.assertEqual(command[1], 'build')
            self.assertEqual(command[command.index('--artifact-profile') + 1], 'compiler-job')
            return self.result(command, error='intentional seed stop')
        with self.assertRaises(SystemExit):
            self.invoke(GENERATED, action, provided=False)
        self.assertEqual([c[1] for c in self.commands if c[0] != str(self.git)], ['build'])
        self.assertEqual(json.loads(self.output.read_text())['compiler_mode'], 'seed-build')


if __name__ == '__main__':
    unittest.main()
