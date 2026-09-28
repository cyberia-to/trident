#!/usr/bin/env python3
"""Integrity guards only; dummy inputs do not establish compiler acceptance."""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('package_check', Path(__file__).with_name('check.py'))
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)


class Integrity(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.compiler, self.joy, self.inventory = [self.root / name for name in ['compiler', 'joy', 'inventory']]
        for path in [self.compiler, self.joy, self.inventory]:
            path.write_bytes(b'guard-only dummy bytes')
        self.receipt = self.root / 'receipt.json'
        self.data = dict(status='compiler-returned', published_kind='program', compiler=str(self.compiler),
                         compiler_sha256=CHECK.sha(self.compiler), compiler_sha256_end=CHECK.sha(self.compiler),
                         binary_sha256=CHECK.sha(self.joy), binary_sha256_end=CHECK.sha(self.joy))
        self.receipt.write_text(json.dumps(self.data))
        self.expected = CHECK.sha(self.receipt)

    def preflight(self):
        with patch.object(CHECK.subprocess, 'run', side_effect=AssertionError('unexpected tool invocation')):
            return CHECK.preflight(self.receipt, self.expected, self.joy, self.inventory)

    def test_untampered_preflight_is_metadata_only(self):
        self.assertEqual(self.preflight(), self.data)

    def test_tampered_saved_receipt_rejected_before_tools(self):
        self.receipt.write_text(json.dumps(dict(self.data, status='failed')))
        with self.assertRaisesRegex(CHECK.FIXED.Rejected, 'receipt SHA256'):
            self.preflight()

    def test_tampered_compiler_rejected_before_tools(self):
        self.compiler.write_bytes(b'replacement compiler')
        with self.assertRaisesRegex(CHECK.FIXED.Rejected, 'compiler SHA256'):
            self.preflight()

    def test_tampered_joy_rejected_before_tools(self):
        self.joy.write_bytes(b'replacement Joy')
        with self.assertRaisesRegex(CHECK.FIXED.Rejected, 'Joy binary SHA256'):
            self.preflight()

    def test_comment_mutation_changes_exactly_one_ascii_letter(self):
        source = b'module demo\n// preserved comment\npub fn value()->Field{13}\n'
        changed, row = CHECK.comment_mutation(source)
        self.assertEqual(len(source), len(changed))
        self.assertEqual(sum(a != b for a, b in zip(source, changed)), 1)
        self.assertEqual(source[row['offset']] ^ 32, changed[row['offset']])
        self.assertEqual(changed.splitlines()[2], source.splitlines()[2])

    def test_comment_mutation_rejects_missing_comment(self):
        with self.assertRaisesRegex(CHECK.FIXED.Rejected, 'no ASCII comment'):
            CHECK.comment_mutation(b'module demo\n')


if __name__ == '__main__':
    unittest.main(verbosity=2)
