#!/usr/bin/env python3
"""Fail-closed map guards; synthetic logs test parsing, never compiler acceptance."""
import copy
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
        self.assertIn('remain open', result['acceptance'])

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


if __name__ == '__main__':
    unittest.main(verbosity=2)
