"""Adversarial checks use the actual archive and rehash altered semantic records."""
import copy
import gzip
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest

import check_delivery as delivery
from check_relations import check_relations

ROOT = Path(__file__).resolve().parent


class DeliveryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest = json.loads((ROOT / 'archive.json').read_bytes())
        cls.index = json.loads((ROOT / 'files.json').read_bytes())
        cls.payload = delivery.read_archive(ROOT / 'successful-corpus.tar.gz', cls.index, cls.manifest)
        cls.prepared = json.loads((ROOT.parent / 'whole-proof-validation/corpus-v2/prepared-files.json').read_bytes())

    def setUp(self):
        self.payload = dict(type(self).payload)
        self.index = copy.deepcopy(type(self).index)
        self.manifest = copy.deepcopy(type(self).manifest)

    def replace_json(self, name, change):
        value = json.loads(self.payload[name])
        change(value)
        self.payload[name] = (json.dumps(value, indent=2) + '\n').encode()
        self.index[name].update(delivery.identity(self.payload[name]))
        if name.startswith(('corpus/c2/', 'corpus/c3/')) and name not in (
                'corpus/c2/receipt.json', 'corpus/c3/receipt.json'):
            prefix, relative = name.split('/', 2)[1:]
            file_name = f'corpus/{prefix}/files.json'
            files = json.loads(self.payload[file_name])
            files[relative] = delivery.identity(self.payload[name])
            self.payload[file_name] = (json.dumps(files, indent=2) + '\n').encode()
            self.index[file_name].update(delivery.identity(self.payload[file_name]))
            receipt_name = f'corpus/{prefix}/receipt.json'
            receipt = json.loads(self.payload[receipt_name])
            receipt['files'].update(delivery.identity(self.payload[file_name]))
            if relative.startswith('corpora/') and relative.endswith('.json'):
                receipt['corpora'][Path(relative).stem].update(delivery.identity(self.payload[name]))
            self.payload[receipt_name] = (json.dumps(receipt, indent=2) + '\n').encode()
            self.index[receipt_name].update(delivery.identity(self.payload[receipt_name]))
        self.manifest['complete_original_bytes'] = sum(len(data) for key, data in self.payload.items()
                                                        if key.startswith('corpus/'))

    def rejects(self, message):
        with self.assertRaisesRegex(ValueError, message):
            check_relations(self.payload, self.index, self.manifest, self.prepared)

    def test_actual_complete_delivery(self):
        result = delivery.check()
        self.assertEqual([row['observations'] for row in result['generations']], [547, 547])

    def test_wrong_generation_rejected_after_rehash(self):
        self.replace_json('corpus/c2/receipt.json', lambda row: row.update(proof_generation=2))
        self.rejects('corpus generation')

    def test_wrong_count_rejected_after_rehash(self):
        self.replace_json('corpus/c3/receipt.json', lambda row: row.update(observations=546))
        self.rejects('547 corpus cardinality')

    def test_false_passed_report_with_missing_case_rejected(self):
        self.replace_json('corpus/c2/corpora/c2-check-generated-compiler-profile.json',
                          lambda row: row['observations'].pop())
        self.rejects('corpus counts')

    def test_wrong_compiler_rejected_after_rehash(self):
        self.replace_json('corpus/c3/corpora/c3-run-native-compiler.json',
                          lambda row: row.update(compiler_sha256_end='0' * 64))
        self.rejects('provided compiler digest')

    def test_failed_preservation_observation_rejected(self):
        def change(row):
            next(item for item in row['observations'] if 'previous_output_preserved' in item)[
                'previous_output_preserved'] = False
        self.replace_json('corpus/c2/corpora/c2-check-generated-compiler-profile.json', change)
        self.rejects('failed observation flag')

    def test_wrong_child_command_rejected(self):
        def change(row):
            row['commands'][0]['command'][-1] = 'different-result.json'
        self.replace_json('corpus/c2/receipt.json', change)
        self.rejects('child exact argv')

    def test_nonempty_runtime_path_rejected(self):
        def change(row):
            row['runtime_environment']['PATH'] = '/usr/bin'
        self.replace_json('corpus/c2/receipt.json', change)
        self.rejects('runtime environment')

    def test_incorrect_fresh_result_rejected(self):
        self.replace_json('fresh/c1/receipt.json', lambda row: row.update(status='failed'))
        self.rejects('producer/fresh verification status')

    def test_wrong_orchestration_source_rejected(self):
        self.replace_json('corpus/orchestration/receipt.json', lambda row: row.update(driver_sha256='0' * 64))
        self.rejects('orchestration source identity')

    def test_wrong_generation_input_rejected_even_identical_bytes(self):
        def change(row):
            for key in ('inputs_before', 'inputs_after'):
                old = next(path for path in row[key] if path.endswith('/compiler.dag'))
                row[key][old.replace('c1-fresh', 'c2-fresh')] = row[key].pop(old)
        self.replace_json('corpus/c2/receipt.json', change)
        self.rejects('generation-specific six inputs')

    def check_small_archive(self, members, expected_members, error):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'test.tar.gz'
            with path.open('wb') as raw:
                with gzip.GzipFile(filename='', fileobj=raw, mode='wb', mtime=0) as zipped:
                    with tarfile.open(fileobj=zipped, mode='w') as archive:
                        for name, kind, data in members:
                            member = tarfile.TarInfo(name)
                            member.type = kind
                            member.size = len(data)
                            archive.addfile(member, io.BytesIO(data))
            index = {name: delivery.identity(data) for name, data in expected_members.items()}
            manifest = {'files': len(index), 'logical_bytes': sum(len(data) for data in expected_members.values()),
                        'archive': delivery.identity(path.read_bytes())}
            with self.assertRaisesRegex(ValueError, error):
                delivery.read_archive(path, index, manifest)

    def test_duplicate_archive_member_rejected(self):
        self.check_small_archive([('a', tarfile.REGTYPE, b'a')] * 2, {'a': b'a'}, 'duplicate')

    def test_archive_link_rejected(self):
        self.check_small_archive([('a', tarfile.SYMTYPE, b'')], {'a': b''}, 'unsafe')

    def test_archive_traversal_rejected(self):
        self.check_small_archive([('../a', tarfile.REGTYPE, b'a')], {'../a': b'a'}, 'unsafe')

    def test_missing_archive_member_rejected(self):
        self.check_small_archive([], {'a': b'a'}, 'membership')

    def test_rehashed_archive_corruption_rejected(self):
        self.check_small_archive([('a', tarfile.REGTYPE, b'b')], {'a': b'a'}, 'member digest')


if __name__ == '__main__':
    unittest.main()
