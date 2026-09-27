#!/usr/bin/env python3
"""Fixture/identity guards; synthetic records do not establish C2 acceptance."""
import argparse
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('source_scale', Path(__file__).with_name('check.py'))
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)


class SourceScaleGuards(unittest.TestCase):
    def test_exact_padding_and_historical_invalid_sources(self):
        values = {name: (source, code) for name, source, code in CHECK.cases()}
        for size in [4096, 65536]:
            source, code = values[f'valid{size}']
            self.assertEqual(len(source), size)
            self.assertEqual(code, 0)
            self.assertTrue(source.startswith(b'program sample //'))
            self.assertTrue(source.endswith(b'\nfn main()->Field{13}'))
        self.assertEqual(values['exact65536'], (b'\xff' + bytes(65535), 1))
        self.assertEqual(values['excess65537'], (b'\xff' + bytes(65536), 7))
        source, code = values['diagnostic-after-comment']
        self.assertGreater(source.index(b'missing'), 4096)
        self.assertEqual(code, 5)

    def test_actual_c2_identity_guard_precedes_any_tool(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            compiler, joy, receipt = [root / name for name in ['result.dag', 'joy', 'receipt.json']]
            particle = '01' * 32
            compiler.write_bytes(b'NOXDAG01' + bytes.fromhex(particle) + b'guard-only')
            joy.write_bytes(b'guard-only')
            document = dict(status='compiler-returned', published_kind='program', artifact_directory=str(root),
                result_sha256=CHECK.sha(compiler), binary_sha256=CHECK.sha(joy), binary_sha256_end=CHECK.sha(joy),
                host_flags=CHECK.HOST, execution=dict(ok=True, published_particle=particle,
                execution=dict(compiler_job=dict(status='success', compiled_particle=particle))))
            receipt.write_text(json.dumps(document))
            args = argparse.Namespace(compiler=compiler, compiler_sha256=CHECK.sha(compiler), joy=joy,
                                      joy_sha256=CHECK.sha(joy), producer_receipt=receipt,
                                      producer_receipt_sha256=CHECK.sha(receipt))
            with patch.object(CHECK.subprocess, 'run', side_effect=AssertionError('unexpected compiler/build')):
                self.assertEqual(CHECK.bind(args), document)
                compiler.write_bytes(b'replacement compiler')
                with self.assertRaisesRegex(ValueError, 'not actual C2'):
                    CHECK.bind(args)

    def test_changed_producer_receipt_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'receipt.json'
            path.write_text('{}')
            args = argparse.Namespace(producer_receipt=path, producer_receipt_sha256='0' * 64)
            with self.assertRaisesRegex(ValueError, 'producer receipt changed'):
                CHECK.bind(args)

    def test_forced_output_requires_actual_guest_rejection(self):
        CHECK.require_guest_rejection(dict(exit_code=1, stderr='error: guest compilation failed: invalid binding'))
        for row in [dict(exit_code=1, stderr='host timeout'),
                    dict(exit_code=0, stderr='guest compilation failed: unexpected')]:
            with self.assertRaisesRegex(ValueError, 'other than guest compilation'):
                CHECK.require_guest_rejection(row)

    def test_missing_tool_at_end_preserves_failed_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            report = dict(status='passed', error={'original': 'failure evidence'})
            CHECK.record_end_hash(report, 'compiler_sha256_end', Path(directory) / 'missing')
            self.assertEqual(report['status'], 'failed')
            self.assertIsNone(report['compiler_sha256_end'])
            self.assertIn('compiler_sha256_end', report['identity_errors'])
            self.assertEqual(report['error'], {'original': 'failure evidence'})

    def test_atom_reader_rejects_extra_or_wrong_shape(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'noun.dag'
            particle = bytes(range(32))
            data = b'NOXDAG01' + particle + (1).to_bytes(4, 'little') + particle + b'\x08' + (13).to_bytes(8, 'little')
            path.write_bytes(data)
            self.assertEqual(CHECK.atomic_output(path), 13)
            for invalid in [data + b'\0', data[:76] + b'\x40' + data[77:], b'wrong']:
                path.write_bytes(invalid)
                with self.assertRaises(ValueError):
                    CHECK.atomic_output(path)


if __name__ == '__main__':
    unittest.main(verbosity=2)
