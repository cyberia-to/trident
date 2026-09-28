"""Behavioral guards using the actual historical rehearsal kit; no acceptance fabrication."""
import gzip
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import tarfile
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('prepare_source', Path(__file__).with_name('prepare-selfhost-source.py'))
P = importlib.util.module_from_spec(spec)
spec.loader.exec_module(P)


class Preparation(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.compiler = gzip.decompress((ROOT / 'audit/self-hosting/lexer-bootstrap/c2.dag.gz').read_bytes())
        cls.inventory = (ROOT / 'audit/self-hosting/lexer-bootstrap/inventory.json').read_bytes()
        cls.metadata = {}
        with tarfile.open(Path(__file__).with_name('fixtures') / 'selfhost-kit-metadata.tar.gz') as tar:
            for member in tar:
                cls.metadata[member.name] = tar.extractfile(member).read()
        cls.map = json.loads(cls.metadata['fixed-point.json'])['fixed_point']['source_sha256_set']

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='selfbuild guard ')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.kit, self.source, self.output = [self.root / n for n in ['kit', 'source', 'output']]
        self.kit.mkdir()
        self.source.mkdir()
        for name, data in dict(self.metadata, **{'compiler.dag': self.compiler, 'inventory.json': self.inventory}).items():
            (self.kit / name).write_bytes(data)
        for row in self.map.values():
            target = self.source / row['path']
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / row['path'], target)

    def pin(self):
        return hashlib.sha256((self.kit / 'kit.json').read_bytes()).hexdigest()

    def prepare(self, **kwargs):
        return P.prepare(self.kit, self.source, self.output, self.pin(), rehearsal=True, **kwargs)

    def rejected(self, pattern):
        with self.assertRaisesRegex((ValueError, OSError), pattern):
            self.prepare()
        self.assertFalse(self.output.exists())

    def update_member(self, name, data):
        (self.kit / name).write_bytes(data)
        manifest = json.loads((self.kit / 'kit.json').read_bytes())
        manifest['files'][name] = P.identity(data)
        (self.kit / 'kit.json').write_text(json.dumps(manifest))

    def change_fixed(self, mutate):
        fixed = json.loads((self.kit / 'fixed-point.json').read_bytes())
        mutate(fixed['fixed_point'])
        self.update_member('fixed-point.json', json.dumps(fixed).encode())

    def test_actual_rehearsal_copies_exact_sources_and_compiler(self):
        report = self.prepare()
        self.assertEqual((report['status'], report['kit_status']), ('prepared', 'rehearsal'))
        self.assertEqual(len(report['sources']), 94)
        manifest = json.loads((self.output / 'package.json').read_bytes())
        self.assertEqual(manifest['options'], P.OPTIONS)
        self.assertEqual(manifest['limits'], P.LIMITS)
        self.assertEqual((self.output / 'compiler.dag').read_bytes(), self.compiler)
        for entry in manifest['modules']:
            row = self.map[entry['logical_path']]
            self.assertEqual((self.output / entry['file']).read_bytes(), (self.source / row['path']).read_bytes())

    def test_default_rejects_actual_unaccepted_kit(self):
        with self.assertRaisesRegex(ValueError, 'accepted kit required'):
            P.prepare(self.kit, self.source, self.output, self.pin())
        self.assertFalse(self.output.exists())

    def test_externally_pinned_manifest_precedes_json_parse(self):
        original = self.pin()
        (self.kit / 'kit.json').write_bytes(b'{malformed')
        with self.assertRaisesRegex(ValueError, 'pinned kit manifest'):
            P.prepare(self.kit, self.source, self.output, original, True)
        self.assertFalse(self.output.exists())

    def test_same_length_source_change_rejects_before_output(self):
        path = self.source / 'compiler/nox/main.tri'
        data = path.read_bytes()
        path.write_bytes(bytes([data[0] ^ 1]) + data[1:])
        self.rejected('source bytes changed')

    def test_missing_source_rejects_before_output(self):
        (self.source / 'compiler/nox/main.tri').unlink()
        with self.assertRaises(FileNotFoundError):
            self.prepare()
        self.assertFalse(self.output.exists())

    def test_changed_c2_rejected_even_with_updated_manifest_pin(self):
        data = self.compiler[:-1] + bytes([self.compiler[-1] ^ 1])
        self.update_member('compiler.dag', data)
        self.rejected('frozen C2 bytes')

    def test_changed_inventory_rejected_with_updated_manifest_pin(self):
        self.update_member('inventory.json', self.inventory + b' ')
        self.rejected('frozen inventory')

    def test_origin_and_missing_map_member_cannot_change_fixed_package(self):
        for mutate in [lambda f: f['source_sha256_set']['native_compiler'].update(origin_version='2'),
                       lambda f: f['source_sha256_set'].pop('native_compiler')]:
            with self.subTest(mutate=mutate):
                (self.kit / 'fixed-point.json').write_bytes(self.metadata['fixed-point.json'])
                self.change_fixed(mutate)
                self.rejected('frozen source map')

    def test_profile_limits_and_bool_scalar_cannot_change_frozen_job(self):
        for mutate in [lambda f: f['options'].update(output_profile=0),
                       lambda f: f['options'].update(input_profile=True),
                       lambda f: f['limits'].update(reductions=20000000001)]:
            with self.subTest(mutate=mutate):
                (self.kit / 'fixed-point.json').write_bytes(self.metadata['fixed-point.json'])
                self.change_fixed(mutate)
                self.rejected('frozen options and limits')

    def test_unlisted_file_and_nonportable_paths_reject(self):
        (self.kit / 'extra').write_bytes(b'')
        self.rejected('unlisted kit file')
        (self.kit / 'extra').unlink()
        # Portable path guard independently checks aliases without relying on disk case behavior.
        for name in ['../main.tri', '/main.tri', 'lib\\main.tri', 'NUL', 'COM1.txt', 'name.']:
            with self.subTest(name=name), self.assertRaises(ValueError):
                P.relative(name)

    def test_source_map_path_escape_rejects_with_updated_kit_pin(self):
        for bad in ['../main.tri', '/main.tri', 'lib\\main.tri', 'COMPILER/nox/main.tri']:
            with self.subTest(path=bad):
                (self.kit / 'fixed-point.json').write_bytes(self.metadata['fixed-point.json'])
                self.change_fixed(lambda fixed: fixed['source_sha256_set']['native_compiler'].update(path=bad))
                self.rejected('frozen source map')

    def test_kit_file_count_accepts_exact_cap_and_rejects_one_above(self):
        count = len(list(self.kit.iterdir()))
        for index in range(P.MAX_FILES - count):
            self.update_member(f'extra{index}', b'')
        self.prepare()
        self.output = self.root / 'second output'
        self.update_member('one-more', b'')
        self.rejected('kit file count')

    def test_declared_kit_size_rejects_before_opening_oversized_payload(self):
        manifest = json.loads((self.kit / 'kit.json').read_bytes())
        manifest['files']['compiler.dag']['bytes'] = P.MAX_FILE + 1
        (self.kit / 'kit.json').write_text(json.dumps(manifest))
        self.rejected('kit file size')

    def test_duplicate_json_key_and_nonfinite_number_reject(self):
        for raw in [b'{"a":1,"a":2}', b'{"a":NaN}']:
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                P.load(raw)

    def test_existing_output_file_and_directory_remain_unchanged(self):
        self.output.write_bytes(b'preserved')
        with self.assertRaisesRegex(ValueError, 'fresh'):
            self.prepare()
        self.assertEqual(self.output.read_bytes(), b'preserved')
        self.output.unlink()
        self.output.mkdir()
        (self.output / 'marker').write_bytes(b'preserved')
        with self.assertRaisesRegex(ValueError, 'fresh'):
            self.prepare()
        self.assertEqual(list(self.output.iterdir()), [self.output / 'marker'])

    def test_output_overlap_does_not_mutate_inputs(self):
        before = set(self.source.rglob('*'))
        with self.assertRaisesRegex(ValueError, 'overlap'):
            P.prepare(self.kit, self.source, self.source / 'new', self.pin(), True)
        self.assertEqual(set(self.source.rglob('*')), before)

    def test_case_alias_of_input_parent_cannot_receive_output(self):
        alias = self.root / 'SOURCE'
        if not alias.exists() or not alias.samefile(self.source):
            self.skipTest('filesystem has case-sensitive directory names')
        before = set(self.source.rglob('*'))
        with self.assertRaisesRegex(ValueError, 'overlap'):
            P.prepare(self.kit, self.source, alias / 'new', self.pin(), True)
        self.assertEqual(set(self.source.rglob('*')), before)

    def test_source_and_parent_symlinks_reject(self):
        def symlink(path, target, **kwargs):
            try:
                path.symlink_to(target, **kwargs)
            except OSError as error:
                if getattr(error, 'winerror', None) == 1314:
                    self.skipTest('Windows symlink privilege is unavailable (WinError 1314)')
                raise
        source_file = self.source / 'compiler/nox/main.tri'
        saved = source_file.read_bytes()
        source_file.unlink()
        symlink(source_file, ROOT / 'compiler/nox/main.tri')
        self.rejected('symlink')
        source_file.unlink()
        source_file.write_bytes(saved)
        link = self.root / 'linked'
        symlink(link, self.source, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, 'symlink'):
            P.prepare(self.kit, self.source, link / 'output', self.pin(), True)
        self.assertFalse((self.source / 'output').exists())

    def test_regular_read_accepts_exact_cap_and_rejects_one_above_before_read(self):
        path = self.root / 'bounded'
        path.write_bytes(b'abc')
        self.assertEqual(P.read(path, 3), b'abc')
        with self.assertRaisesRegex(ValueError, 'bounded ordinary file'):
            P.read(path, 2)

    def test_write_failure_retains_failed_receipt_and_no_prepared_claim(self):
        original = P.write
        def fail_source(path, data):
            if path.name == '0.tri':
                raise OSError('injected write failure')
            original(path, data)
        with mock.patch.object(P, 'write', side_effect=fail_source):
            with self.assertRaisesRegex(OSError, 'injected'):
                self.prepare()
        report = json.loads((self.output / 'receipt.json').read_bytes())
        self.assertEqual(report['status'], 'failed')
        self.assertFalse((self.output / 'package.json').exists())

    def test_relocation_and_spaces_preserve_manifest_and_copied_bytes(self):
        first = self.prepare()
        relocated = self.root / 'source пробел'
        self.source.rename(relocated)
        other = self.root / 'other output'
        second = P.prepare(self.kit, relocated, other, self.pin(), True)
        self.assertEqual(first['files'], second['files'])
        self.assertEqual((self.output / 'package.json').read_bytes(), (other / 'package.json').read_bytes())


if __name__ == '__main__':
    unittest.main()
