"""Exact C2 retirement transition tested only with tiny owned fixture files."""
import contextlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import resources
import run


class RetirementTests(unittest.TestCase):
    def setUp(self):
        self.stack = contextlib.ExitStack()
        self.addCleanup(self.stack.close)
        self.base = Path(self.stack.enter_context(tempfile.TemporaryDirectory())).resolve()
        self.root = self.base / 'controller'
        self.suite = self.base / 'suite-c2'
        self.work = self.suite / 'whole-c2'
        self.command = self.suite / 'attempts/whole-c2-verify-cost'
        self.root.mkdir()
        self.work.mkdir(parents=True)
        self.command.mkdir(parents=True)
        (self.root / 'native-processes').mkdir()
        self.cost = self.base / 'retained-cost'
        self.cost.write_bytes(b'fixture complete mutation\n')
        self.original_failure = self.base / 'original-failed.json'
        self.original_failure.write_text('{"status":"failed","exit_code":-15}')
        self.output = self.work / 'cost.dag'
        self.output.write_bytes(b'protected\n')
        self.expected = resources.identity(self.cost)
        self.plan = dict(historical_cost='retained-cost', prelude_command=['/exact/fixture-verifier'])
        for obj, key, value in [(run, 'ROOT', self.root), (run, 'BASE', self.base),
                                (run, 'PLAN', self.plan), (run, 'COST_ID', self.expected),
                                (resources, 'ROOT', self.root), (resources, 'ROOTS', {2: self.suite})]:
            self.stack.enter_context(patch.object(obj, key, value))
        self.stack.enter_context(patch.object(run.ownership, 'observe', return_value={'status': 'empty'}))
        self.stack.enter_context(patch.object(resources, 'process_snapshot', return_value=[]))
        s = self.cost.stat()
        self.historical = dict(status='passed-construction-admission', accepted_as_rejection=False,
                               fresh_verification_required=True, expected_fresh_error='semantic terminal: Claim',
                               certificate=dict(stat=dict(device=s.st_dev, inode=s.st_ino, bytes=s.st_size,
                                   mtime_ns=s.st_mtime_ns, ctime_ns=s.st_ctime_ns, links=s.st_nlink, mode=s.st_mode)))
        self.row = dict(name='cost', certificate=self.expected, error='semantic terminal: Claim',
                        protected_output=resources.identity(self.output))
        (self.command / 'stdout').write_bytes(b'')
        (self.command / 'stderr').write_text('error: semantic terminal: Claim\n')
        (self.command / 'resources.jsonl').write_text('{"rss_bytes":1}\n')
        self.receipt = dict(status='passed', exit_code=1, expected_exit=1, argv=self.plan['prelude_command'],
                            inputs_before={str(self.cost): self.expected}, inputs_after={str(self.cost): self.expected},
                            caps=dict(wall=7500, cpu=7500, rss=6 * 1024**3, file=32 * 1024**2),
                            process_binding=dict(pid=10, pgid=10, started='fixture'), final_group=dict(status='empty'),
                            files={p.name: resources.identity(p) for p in self.command.iterdir()})
        self.write(self.root / 'native-processes/whole-c2-verify-cost.json', {'fixture': True})
        self.write(self.root / 'native-processes/whole-c2-verify-cost.retired', {'fixture': True})
        self.freeze()

    def write(self, path, value):
        resources.write(path, value)

    def freeze(self):
        self.write(self.work / 'retained-cost-admission.json', self.historical)
        self.write(self.work / 'receipt.json', dict(rejections=[self.row]))
        self.write(self.command / 'receipt.json', self.receipt)
        self.write(self.work / 'prelude-complete.json', dict(status='passed', generation=2,
                   suite_receipt=resources.identity(self.work / 'receipt.json'), rejection=self.row,
                   admission=resources.identity(self.work / 'retained-cost-admission.json')))

    def test_only_completed_exact_mutation_is_retired(self):
        original = self.original_failure.read_bytes()
        run.retire_cost()
        self.assertFalse(self.cost.exists())
        final = run.load(self.root / 'retained-cost-retirement.json')
        self.assertEqual('passed', final['status'])
        self.assertTrue(final['removed'])
        self.assertEqual(0, final['charged_bytes'])
        self.assertEqual(original, self.original_failure.read_bytes())
        before = run.load(self.root / 'retained-cost-retirement-before.json')
        self.assertFalse(before['removed'])

    def test_failed_fresh_verifier_never_retires(self):
        self.receipt['status'] = 'failed'
        self.freeze()
        with self.assertRaises(ValueError):
            run.retire_cost()
        self.assertTrue(self.cost.exists())

    def test_resource_stop_is_not_a_rejection(self):
        self.receipt['resource_stop'] = 'wall'
        self.freeze()
        with self.assertRaises(ValueError):
            run.retire_cost()
        self.assertTrue(self.cost.exists())

    def test_protected_output_must_remain_exact(self):
        self.output.write_bytes(b'changed')
        with self.assertRaises(ValueError):
            run.retire_cost()
        self.assertTrue(self.cost.exists())

    def test_changed_raw_log_is_rejected(self):
        (self.command / 'stderr').write_text('other error\n')
        with self.assertRaises(ValueError):
            run.retire_cost()
        self.assertTrue(self.cost.exists())

    def test_live_verifier_group_prevents_retirement(self):
        with patch.object(run.ownership, 'observe', return_value={'status': 'owned'}), self.assertRaises(ValueError):
            run.retire_cost()
        self.assertTrue(self.cost.exists())

    def test_changed_file_bytes_prevent_retirement(self):
        self.cost.write_bytes(b'changed')
        with self.assertRaises(ValueError):
            run.retire_cost()
        self.assertTrue(self.cost.exists())
        self.assertFalse(run.load(self.root / 'retained-cost-retirement-failed.json')['removed'])

    def test_same_bytes_replacement_inode_is_rejected(self):
        replacement = self.base / 'replacement'
        replacement.write_bytes(self.cost.read_bytes())
        replacement.replace(self.cost)
        with self.assertRaises(ValueError):
            run.retire_cost()
        self.assertTrue(self.cost.exists())

    def test_hardlinked_temporary_is_rejected(self):
        (self.base / 'second-link').hardlink_to(self.cost)
        with self.assertRaises(ValueError):
            run.retire_cost()
        self.assertTrue(self.cost.exists())

    def test_failure_after_unlink_retains_actual_effect_and_conservative_charge(self):
        original = run.Held.unlink
        def interrupted(held):
            original(held)
            raise OSError('fixture failure after unlink')
        with patch.object(run.Held, 'unlink', interrupted), self.assertRaises(OSError):
            run.retire_cost()
        self.assertFalse(self.cost.exists())
        result = run.load(self.root / 'retained-cost-retirement-failed.json')
        self.assertTrue(result['removed'])
        self.assertEqual(self.expected['bytes'], result['charged_bytes'])
        self.assertFalse((self.root / 'retained-cost-retirement.json').exists())


if __name__ == '__main__':
    unittest.main()
