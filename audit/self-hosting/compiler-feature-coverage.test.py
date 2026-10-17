#!/usr/bin/env python3
"""Fail-closed map guards; synthetic logs test parsing, never compiler acceptance."""
import copy
import gzip
import hashlib
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('coverage_check', HERE / 'compiler-feature-coverage.py')
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)
ROOT = HERE.parents[1]
MAPPING = CHECK.read(HERE / 'compiler-feature-coverage.json')
INVENTORY = ROOT / MAPPING['inventory']['path']


class CoverageGuards(unittest.TestCase):
    def rejected(self, mapping, message, inventory=INVENTORY):
        with self.assertRaisesRegex(CHECK.Invalid, message):
            CHECK.validate(ROOT, mapping, inventory)

    def test_reviewed_map_binds_real_sources_and_historical_case_records(self):
        result = CHECK.validate(ROOT, MAPPING, INVENTORY)
        self.assertEqual(result['features'], 51)
        self.assertEqual(result['selected_installed_cases'], 145)
        self.assertEqual(result['rust_evidence']['status'], 'not supplied')
        self.assertIn('not checked by this mapping', result['acceptance'])

    def test_changed_inventory_requires_new_review(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'inventory.json'
            path.write_bytes(INVENTORY.read_bytes() + b'\n')
            self.rejected(MAPPING, 'inventory changed', path)

    def test_missing_feature_cannot_pass(self):
        changed = copy.deepcopy(MAPPING)
        del changed['features']['stmt.for']
        self.rejected(changed, 'feature keys')

    def test_changed_closure_source_cannot_pass(self):
        changed = copy.deepcopy(MAPPING)
        changed['sources']['native_compiler']['sha256'] = '0' * 64
        self.rejected(changed, 'changed path')

    def test_changed_test_source_cannot_pass(self):
        changed = copy.deepcopy(MAPPING)
        next(iter(changed['rust_tests'].values()))['sha256'] = '0' * 64
        self.rejected(changed, 'changed file')

    def test_changed_receipt_cannot_pass(self):
        changed = copy.deepcopy(MAPPING)
        changed['installed_runners']['full']['receipt_sha256'] = '0' * 64
        self.rejected(changed, 'changed receipt')

    def test_rejection_case_cannot_stand_in_for_executed_positive(self):
        changed = copy.deepcopy(MAPPING)
        changed['groups']['locals']['installed_positive'][0] = 'full:unknown'
        self.rejected(changed, 'not positive')

    def test_unknown_case_cannot_pass(self):
        changed = copy.deepcopy(MAPPING)
        changed['groups']['locals']['installed_positive'][0] = 'full:invented-case'
        self.rejected(changed, 'absent or ambiguous')

    def test_missing_intrinsic_mapping_cannot_pass(self):
        changed = copy.deepcopy(MAPPING)
        del changed['intrinsics']['vm.core.convert.split']
        self.rejected(changed, 'intrinsic declaration set')

    def test_new_known_only_intrinsic_caller_cannot_pass(self):
        for call in ['field.inv', 'another.inv', 'inv']:
            inventory = CHECK.read(INVENTORY)
            inventory['modules']['native_compiler']['calls'][call] = 1
            with self.assertRaisesRegex(CHECK.Invalid, 'known-only intrinsic'):
                CHECK.known_only_calls(inventory, 'vm.core.field.inv',
                                      MAPPING['intrinsics']['vm.core.field.inv'])

    def test_no_test_log_cannot_satisfy_strict_execution_requirement(self):
        with self.assertRaisesRegex(CHECK.Invalid, 'not complete and green'):
            CHECK.validate(ROOT, MAPPING, INVENTORY, require_rust=True)

    def test_log_parser_requires_completed_unfiltered_suite_and_all_selected_tests(self):
        # Synthetic log ONLY; this does not report actual Rust/C1 success.
        tests = {'one': {'binary': 'native_compiler', 'test': 'group::one'}}
        counts = {'native_compiler': 1}
        line = 'Running tests/native_compiler.rs (target/example)\n'
        line += 'test group::one ... ok\n'
        summary = 'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\n'
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'synthetic.log'
            for suffix, expected in [('', 'incomplete or failed'),
                                     (summary, 'selected suites passed'),
                                     (summary.replace('0 filtered', '1 filtered'), 'incomplete or failed'),
                                     (summary.replace('1 passed', '2 passed'), 'incomplete or failed')]:
                path.write_text(line + suffix)
                self.assertEqual(CHECK.rust_evidence(path, tests, counts)['status'], expected)
            path.write_text(line.replace('group::one', 'group::another') + summary)
            self.assertEqual(CHECK.rust_evidence(path, tests, counts)['missing'], ['one'])


class SplitGateGuards(unittest.TestCase):
    """Synthetic Cargo metadata/logs validate accounting, never guest acceptance."""
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.directory = self.root / 'evidence'
        self.directory.mkdir()
        self.files = ['Cargo.toml', 'Cargo.lock', 'tests/a.rs', 'tests/b.rs', 'lib/std/compiler/nox/lexer.tri']
        for name in self.files:
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text('fixture\n')
        inventory = self.directory / 'inventory.json'
        inventory.write_text('{}')
        source = dict(path=self.files[-1], sha256=CHECK.sha(self.root / self.files[-1]), bytes=8)
        self.mapping = dict(source_revision='a' * 40, inventory={'sha256': CHECK.sha(inventory)}, sources={'lexer': source})
        metadata = dict(workspace_default_members=['fixture'], packages=[dict(id='fixture',
            manifest_path=str(self.root / 'Cargo.toml'), targets=[dict(kind=['test'], name=name,
                src_path=str(self.root / f'tests/{name}.rs'), test=True, doctest=False) for name in ('a', 'b')])])
        captured = self.archive('metadata.json.gz', json.dumps(metadata).encode())
        captured.update(argv=['cargo', 'metadata', '--format-version', '1', '--no-deps', '--locked', '--offline'],
                        cwd=str(self.root), cargo_toml_sha256=CHECK.sha(self.root / 'Cargo.toml'),
                        cargo_lock_sha256=CHECK.sha(self.root / 'Cargo.lock'))
        commands = [self.command('first', 'primary', 'a'), self.command('second', 'primary', 'b'),
                    self.command('reviewed-rerun', 'supplemental', 'a')]
        one = dict(passed=1, failed=0, ignored=0, filtered_out=0)
        two, three = CHECK.sum_counts([one, one]), CHECK.sum_counts([one, one, one])
        sources = [dict(logical_path='lexer', **source)]
        identity = json.dumps({'lexer': {'sha256': source['sha256'], 'bytes': 8}}, sort_keys=True, separators=(',', ':')).encode()
        self.gate = dict(schema='trident/split-rust-gate/v1', status='passed', committed_source_revision='a' * 40,
                         committed_sources_verified=True,
                         source_inventory_path='inventory.json', source_inventory_sha256=CHECK.sha(inventory),
                         source_map_sha256=hashlib.sha256(identity).hexdigest(), sources=sources,
                         cargo_metadata=captured, commands=commands, counts=dict(primary=two, supplemental=one, observed=three, unique=two),
                         checks=dict(warnings=0, failures=0, source_unchanged=True),
                         coverage=dict(metadata_targets=['test:a', 'test:b'], covered_targets=['test:a', 'test:b'],
                                       missing_targets=[], unique_registered_tests=2, observed_test_executions=3))
        combined = self.archive('combined.log.gz', b''.join(self.raw(c) for c in commands))
        combined.update(concatenation_only=True, ordered_commands=[c['id'] for c in commands],
                        ordered_raw_logs=[c['raw_log'] for c in commands])
        self.gate['combined_log'] = combined

    def archive(self, name, raw):
        path = self.directory / name
        path.write_bytes(gzip.compress(raw, mtime=0))
        return dict(path=name, encoding='gzip', raw_sha256=hashlib.sha256(raw).hexdigest(), stored_sha256=CHECK.sha(path))

    def raw(self, command):
        return gzip.decompress((self.directory / command['raw_log']).read_bytes())

    def command(self, identity, kind, target):
        raw = (f'Running tests/{target}.rs (target/{identity})\ntest one ... ok\n'
               'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\n').encode()
        archive = self.archive(identity + '.log.gz', raw)
        counts = dict(passed=1, failed=0, ignored=0, filtered_out=0)
        return dict(id=identity, kind=kind, argv=['cargo', 'test', '--release', '--locked', '--offline',
                    '--test', target, '--', '--test-threads=4'], env={'CARGO_TARGET_DIR': '../target'}, cwd=str(self.root),
                    targets=['test:' + target], exit_code=0, raw_log=archive['path'], log_encoding='gzip',
                    raw_sha256=archive['raw_sha256'], stored_sha256=archive['stored_sha256'],
                    counts=counts, suite_results=[dict(target='test:' + target, **counts)],
                    source_hashes={path: CHECK.sha(self.root / path) for path in self.files})

    def verify(self):
        return CHECK.split_gate(self.root, self.directory, self.gate, self.mapping)

    def rejected(self, message):
        with self.assertRaisesRegex(CHECK.Invalid, message):
            self.verify()

    def test_supplemental_execution_is_observed_but_not_counted_twice(self):
        counts, suites, primary = self.verify()
        self.assertEqual(counts['passed'], 2)
        self.assertEqual(suites, 2)
        self.assertEqual(primary.count(b'test one ... ok'), 2)
        self.assertEqual(self.gate['counts']['observed']['passed'], 3)

    def test_committed_source_verification_must_be_explicitly_true(self):
        for value in (False, None, 1, 'true'):
            self.gate['committed_sources_verified'] = value
            self.rejected('committed gate sources not verified')
        del self.gate['committed_sources_verified']
        self.rejected('committed gate sources not verified')

    def test_missing_primary_target_cannot_pass(self):
        del self.gate['commands'][1]
        self.rejected('incomplete primary target')

    def test_omitting_target_from_metadata_and_commands_cannot_hide_it(self):
        row = self.gate['cargo_metadata']
        data = json.loads(gzip.decompress((self.directory / row['path']).read_bytes()))
        data['packages'][0]['targets'].pop()
        row.update(self.archive(row['path'], json.dumps(data).encode()))
        self.gate['coverage']['metadata_targets'].pop()
        self.rejected('integration target set changed')

    def test_failed_subcommand_cannot_be_masked_by_green_totals(self):
        self.gate['commands'][1]['exit_code'] = 1
        self.rejected('failed Rust subcommand')

    def test_reusing_a_log_or_primary_target_is_rejected(self):
        self.gate['commands'][2]['raw_log'] = self.gate['commands'][0]['raw_log']
        self.rejected('reused Rust command')
        self.gate['commands'][2]['raw_log'] = 'reviewed-rerun.log.gz'
        self.gate['commands'][2]['kind'] = 'primary'
        self.rejected('duplicate primary')

    def test_changed_raw_log_or_raw_digest_is_rejected(self):
        row = self.gate['commands'][0]
        path = self.directory / row['raw_log']
        path.write_bytes(gzip.compress(self.raw(row) + b'extra\n', mtime=0))
        self.rejected('changed raw_log')
        row['stored_sha256'] = CHECK.sha(path)
        self.rejected('decompressed gate log differs')

    def test_double_counted_unique_total_is_rejected(self):
        self.gate['counts']['unique'] = self.gate['counts']['observed']
        self.rejected('double-count')

    def test_changed_current_source_is_rejected(self):
        (self.root / 'tests/a.rs').write_text('changed\n')
        self.rejected('gate test source changed')

    def test_reviewed_test_source_change_requires_matching_supplemental_target(self):
        version = self.archive('test-before-review.rs.gz', (self.root / 'tests/a.rs').read_bytes())
        version.update(source_path='tests/a.rs', superseded_by_command='reviewed-rerun')
        self.gate['source_versions'] = [version]
        (self.root / 'tests/a.rs').write_text('changed\n')
        self.gate['commands'][2]['source_hashes']['tests/a.rs'] = CHECK.sha(self.root / 'tests/a.rs')
        self.verify()
        self.gate['commands'][1]['source_hashes']['tests/a.rs'] = CHECK.sha(self.root / 'tests/a.rs')
        self.rejected('source changed without matching')

    def test_missing_per_command_source_binding_is_rejected(self):
        del self.gate['commands'][0]['source_hashes']['tests/a.rs']
        self.rejected('source bindings incomplete')

    def test_concat_manifest_cannot_hide_missing_bytes(self):
        row = self.gate['combined_log']
        row.update(self.archive(row['path'], b'not the original logs'))
        self.rejected('not exact command concatenation')

    def test_command_test_filter_is_not_complete_target_coverage(self):
        self.gate['commands'][0]['argv'].insert(5, 'one')
        self.rejected('unsupported Rust target selector')

    def test_libtest_output_and_thread_options_do_not_filter_tests(self):
        command = self.gate['commands'][2]
        command['argv'][-1:] = ['--nocapture', '--test-threads=2']
        self.verify()
        command['argv'].append('--skip=one')
        self.rejected('unexpected Rust gate command/filter')

    def test_incomplete_failed_filtered_or_repeated_suite_is_rejected(self):
        command = self.gate['commands'][0]
        raw = self.raw(command)
        for altered in (raw.split(b'test result:')[0], raw.replace(b'0 filtered', b'1 filtered'),
                        raw.replace(b'test result: ok', b'test result: FAILED'), raw + raw,
                        b'warning: unwanted warning\n' + raw):
            with self.assertRaises(CHECK.Invalid):
                CHECK.log_suites(altered, {'test:a': 'tests/a.rs'})

    def test_diagnostic_interleaving_preserves_terminal_suite_counts(self):
        raw = self.raw(self.gate['commands'][0]).replace(b'test one', b'diagnostic: test one')
        rows = CHECK.log_suites(raw, {'test:a': 'tests/a.rs'})
        self.assertEqual(rows[0]['passed'], 1)


if __name__ == '__main__':
    unittest.main(verbosity=2)
