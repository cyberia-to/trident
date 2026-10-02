"""Actual metadata subprocesses with empty PATH; no guest/corpus pass claim."""
import argparse
import contextlib
import hashlib
import importlib.util
import io
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
spec = importlib.util.spec_from_file_location('generated_profile', HERE / 'check-generated-compiler-profile.py')
G = importlib.util.module_from_spec(spec)
spec.loader.exec_module(G)
GIT = G.metadata_git(None)


class MetadataGit(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix='metadata-git-')
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        self.repo = self.root / 'repository'
        self.repo.mkdir()
        self.git(['init', '-q'])
        hooks = self.root / 'empty-hooks'
        hooks.mkdir()
        self.git(['-c', f'core.hooksPath={hooks}', '-c', 'commit.gpgsign=false',
                  '-c', 'user.name=Metadata test', '-c', 'user.email=metadata@example.invalid',
                  'commit', '--allow-empty', '-qm', 'metadata fixture'])
        self.head = self.git(['rev-parse', 'HEAD']).stdout.strip()
        self.output = self.root / 'receipt.json'
        self.compiler = self.root / 'compiler.dag'
        self.compiler.write_bytes(b'metadata-only pinned input; no ART1 execution')

    def git(self, arguments):
        return subprocess.run([str(GIT), *arguments], cwd=self.repo, text=True,
                              capture_output=True, check=True)

    def acceptance(self):
        args = argparse.Namespace(joy=Path(sys.executable), compiler=self.compiler, output=self.output)
        selection = G.CompilerSelection(args, argparse.ArgumentParser())
        self.addCleanup(selection.close)
        return G.Acceptance(selection.binary, self.output, self.repo, selection, GIT)

    def test_real_metadata_runs_with_empty_path_and_preserves_child_environment(self):
        a = self.acceptance()
        with patch.dict(os.environ, PATH=''):
            a.metadata()
            child = a.run(['-c', 'import json,os;print(json.dumps(dict(path=os.environ["PATH"])))'],
                          executable=sys.executable)
            self.assertEqual(child['path'], '')
            self.assertEqual(os.environ['PATH'], '')
        self.assertEqual(a.report['revision'], self.head)
        self.assertEqual(a.report['working_tree'], '')
        self.assertEqual([row['command'][0] for row in a.report['commands'][:2]], [str(GIT)] * 2)
        self.assertEqual(a.report['metadata_git']['sha256'], hashlib.sha256(GIT.read_bytes()).hexdigest())

    def invoke(self, arguments, environment, expected=1):
        argv = ['profile', '--joy', sys.executable, '--compiler', str(self.compiler),
                '--output', str(self.output), *arguments]

        def metadata_only(a):
            a.metadata()
            raise RuntimeError('intentional stop before guest execution')

        with patch.dict(os.environ, environment), patch.object(sys, 'argv', argv), \
                patch.object(G, 'exercise', metadata_only), contextlib.redirect_stdout(io.StringIO()), \
                contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as stopped:
            G.main()
        self.assertEqual(stopped.exception.code, expected)

    def test_environment_selects_actual_git_with_empty_path(self):
        self.invoke([], dict(PATH='', TRIDENT_AUDIT_GIT=str(GIT)))
        receipt = json.loads(self.output.read_text())
        self.assertEqual(receipt['metadata_git']['path'], str(GIT))
        self.assertEqual([row['exit_code'] for row in receipt['commands']], [0, 0])
        self.assertEqual(receipt['observations'], [])
        self.assertIn('intentional stop', receipt['failure'])

    def test_explicit_git_overrides_invalid_environment_without_path_search(self):
        self.invoke(['--git', str(GIT)], dict(PATH='', TRIDENT_AUDIT_GIT='relative-git'))
        receipt = json.loads(self.output.read_text())
        self.assertEqual(receipt['metadata_git']['path'], str(GIT))
        self.assertEqual([row['exit_code'] for row in receipt['commands']], [0, 0])

    def test_relative_environment_and_explicit_paths_reject_before_receipt_creation(self):
        for arguments, environment in [([], dict(PATH='', TRIDENT_AUDIT_GIT='git')),
                                        (['--git', 'git'], dict(PATH='', TRIDENT_AUDIT_GIT=str(GIT)))]:
            with self.subTest(arguments=arguments):
                self.invoke(arguments, environment, expected=2)
                self.assertFalse(self.output.exists())

    def test_missing_git_and_non_file_paths_fail_closed(self):
        with patch.dict(os.environ, PATH=''), self.assertRaisesRegex(AssertionError, 'requires --git'):
            G.metadata_git(None)
        with self.assertRaisesRegex(AssertionError, 'executable file'):
            G.metadata_git(self.root)
        with self.assertRaises(FileNotFoundError):
            G.metadata_git(self.root / 'absent')

    def test_normal_path_fallback_remains_available(self):
        self.assertEqual(G.metadata_git(None), GIT)

    def test_changed_metadata_executable_is_rejected_before_launch(self):
        a = self.acceptance()
        selected = self.root / 'changed-git'
        selected.write_bytes(b'changed executable')
        a.git = selected
        with self.assertRaisesRegex(AssertionError, 'executable changed'):
            a.metadata()
        self.assertEqual(a.report['commands'], [])


if __name__ == '__main__':
    unittest.main()
