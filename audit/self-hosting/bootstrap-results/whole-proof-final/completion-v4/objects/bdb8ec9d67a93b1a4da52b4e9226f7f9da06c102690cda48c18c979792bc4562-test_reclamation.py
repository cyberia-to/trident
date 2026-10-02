"""Real filesystem transitions on tiny, separately owned test fixtures only."""
import copy
import datetime
import fcntl
import hashlib
import json
import os
from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import patch

import reclaim
from processes import birth_ns, classify
from reconstruct import reconstruct
from safe_files import Held, identity, read_json, save, state

ROOT = Path(__file__).resolve().parent
TESTS = ROOT / 'test-work'


def data_identity(data):
    return {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}


class TransitionTests(unittest.TestCase):
    def setUp(self):
        TESTS.mkdir(exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir=TESTS)
        self.root = Path(self.temp.name)
        self.original = self.root / 'original'
        self.target = self.root / 'duplicate'
        self.protected = self.root / 'other-partial'
        self.original.write_bytes(bytes(range(256)) * 3)
        self.target.write_bytes(self.original.read_bytes()[:511])
        self.protected.write_bytes(b'untouched historical unique data')
        self.failure = self.root / 'failed.json'
        self.failure.write_bytes(b'{"status":"failed","cleanup_error":"preserved"}\n')
        (self.root / 'retained').mkdir()
        (self.root / 'retained/failed.json').write_bytes(self.failure.read_bytes())
        self.reg = self.root / 'registrations'
        self.reg.mkdir()
        (self.reg / 'one.json').write_bytes(b'{"pid":42,"pgid":42}\n')
        self.lock = self.root / 'lease'
        self.lock.touch()
        self.plan = {'original': {'path': str(self.original), 'state': state(self.original.stat())},
                     'target': {'path': str(self.target), 'state': state(self.target.stat())},
                     'recipe': {'original': identity(self.original), 'prefix': identity(self.target)},
                     'protected': [{'path': str(self.protected), 'state': state(self.protected.stat()), 'identity': identity(self.protected)}],
                     'retained': [{'source': str(self.failure), 'copy': 'retained/failed.json', 'identity': identity(self.failure)}],
                     'registrations': [{'path': str(self.reg / 'one.json'), 'identity': identity(self.reg / 'one.json')}],
                     'registration_directory': str(self.reg),
                     'registration_entries': ['one.json'],
                     'locks': [{'path': str(self.lock), 'state': state(self.lock.stat())}],
                     'owners': [42], 'quiescence_cutoff_ns': 1,
                     'transition_wall_seconds': 10, 'quiescence_min_interval_seconds': 0}
        self.calls = []

    def tearDown(self):
        self.temp.cleanup()

    def observe(self, _private, label, _owners, _cutoff):
        self.calls.append(label)
        now = time.time_ns()
        return {'started_ns': now, 'observed_ns': now, 'no_live_prior_owner': True}

    def run_transition(self, **kwargs):
        return reclaim.transition(self.plan, self.root, self.root / 'action', self.root / 'private',
                                  observer=kwargs.pop('observer', self.observe), **kwargs)

    def rejected(self, **kwargs):
        with self.assertRaises((ValueError, OSError)):
            self.run_transition(**kwargs)
        self.assertTrue(os.path.lexists(self.target))
        self.assertFalse((self.root / 'action/final.json').exists())

    def test_complete_transition_releases_only_closed_duplicate(self):
        original, other = self.original.read_bytes(), self.protected.read_bytes()
        result = self.run_transition()
        self.assertFalse(self.target.exists())
        self.assertEqual(self.original.read_bytes(), original)
        self.assertEqual(self.protected.read_bytes(), other)
        self.assertEqual(result['released_owned_bytes'], 511)
        self.assertEqual(self.calls, ['quiescence-1', 'quiescence-2'])
        for name in ['before-unlink', 'after-unlink', 'verified-held']:
            r = read_json(self.root / 'action' / (name + '.json'))
            self.assertEqual(r['charged_duplicate_bytes'], 511)
            self.assertEqual(r['released_owned_bytes'], 0)
        self.assertEqual(result['before'], result['after'])
        self.assertEqual(read_json(self.failure)['status'], 'failed')

    def test_same_length_wrong_prefix_rejected_by_complete_comparison(self):
        self.target.write_bytes(b'X' + self.target.read_bytes()[1:])
        self.plan['target']['state'] = state(self.target.stat())
        self.rejected()

    def test_changed_original_tail_rejected_by_whole_hash(self):
        self.original.write_bytes(self.original.read_bytes()[:-1] + b'!')
        self.plan['original']['state'] = state(self.original.stat())
        self.rejected()

    def test_target_symlink_rejected(self):
        self.target.unlink()
        self.target.symlink_to(self.original)
        self.rejected()

    def test_target_hardlink_rejected(self):
        os.link(self.target, self.root / 'second-link')
        self.rejected()

    def test_nonregular_target_rejected_without_blocking(self):
        self.target.unlink()
        os.mkfifo(self.target)
        self.rejected()

    def test_symlink_parent_rejected(self):
        alias = self.root / 'alias'
        alias.symlink_to(self.root, target_is_directory=True)
        self.plan['target']['path'] = str(alias / 'duplicate')
        self.rejected()

    def test_changed_timestamp_rejected(self):
        os.utime(self.target, ns=(self.target.stat().st_atime_ns, self.target.stat().st_mtime_ns + 1000))
        self.rejected()

    def test_changed_size_rejected(self):
        with self.target.open('ab') as f:
            f.write(b'!')
        self.rejected()

    def test_replacement_inode_after_durable_journal_rejected(self):
        def hook(stage):
            if stage == 'after-journal':
                p = self.root / 'replacement'
                p.write_bytes(self.target.read_bytes())
                os.replace(p, self.target)
        self.rejected(hook=hook)
        self.assertTrue((self.root / 'action/before-unlink.json').exists())

    def test_mutation_after_durable_journal_rejected(self):
        def hook(stage):
            if stage == 'after-journal':
                with self.target.open('r+b') as f:
                    f.write(b'!')
        self.rejected(hook=hook)

    def test_protected_file_mutation_before_unlink_rejected(self):
        def hook(stage):
            if stage == 'after-journal':
                self.protected.write_bytes(b'changed')
        self.rejected(hook=hook)

    def test_original_failed_evidence_change_rejected(self):
        self.failure.write_bytes(b'{"status":"passed"}')
        self.rejected()

    def test_changed_source_binding_prevents_unlink(self):
        p = self.root / 'reviewed-driver'
        p.write_bytes(b'original reviewed source')
        bindings = {str(p): identity(p)}
        p.write_bytes(b'changed source')
        self.rejected(bindings=bindings)

    def test_binding_provenance_retained_in_transition(self):
        p = self.root / 'reviewed-driver'
        p.write_bytes(b'original reviewed source')
        bindings = {str(p): identity(p)}
        result = self.run_transition(bindings=bindings)
        self.assertEqual(result['bindings'], bindings)
        self.assertEqual(read_json(self.root / 'action/before-unlink.json')['bindings'], bindings)

    def test_source_changed_after_unlink_does_not_pass(self):
        p = self.root / 'reviewed-driver'
        p.write_bytes(b'original reviewed source')
        bindings = {str(p): identity(p)}
        def hook(stage):
            if stage == 'after-unlink':
                p.write_bytes(b'changed source')
        with self.assertRaises(ValueError):
            self.run_transition(bindings=bindings, hook=hook)
        r = read_json(self.root / 'action/failed.json')
        self.assertTrue(r['unlink_performed'])
        self.assertEqual(r['released_owned_bytes'], 0)

    def test_retained_failure_copy_change_rejected(self):
        (self.root / 'retained/failed.json').write_bytes(b'changed')
        self.rejected()

    def test_extra_native_registration_rejected(self):
        (self.reg / 'two.json').write_text('{}')
        self.rejected()

    def test_extra_native_retirement_rejected(self):
        (self.reg / 'one.retired').write_text('123')
        self.rejected()

    def test_busy_lease_rejected(self):
        with self.lock.open('rb') as lock:
            fcntl.flock(lock.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
            self.rejected()

    def test_second_observation_failure_prevents_unlink(self):
        def observer(*args):
            if args[1] == 'quiescence-2':
                raise ValueError('old owner is active')
            return self.observe(*args)
        self.rejected(observer=observer)

    def test_second_observation_must_be_later(self):
        def observer(*args):
            return {'observed_ns': time.time_ns(), 'started_ns': 1, 'no_live_prior_owner': True}
        self.rejected(observer=observer)

    def test_observation_requires_positive_quiescence_verdict(self):
        def observer(*args):
            return dict(self.observe(*args), no_live_prior_owner=False)
        self.rejected(observer=observer)

    def test_post_unlink_failure_retains_actual_transition_and_charge(self):
        def hook(stage):
            if stage == 'after-unlink':
                raise ValueError('simulated interruption')
        with self.assertRaises(ValueError):
            self.run_transition(hook=hook)
        self.assertFalse(self.target.exists())
        r = read_json(self.root / 'action/failed.json')
        self.assertTrue(r['unlink_performed'])
        self.assertEqual(r['released_owned_bytes'], 0)
        self.assertEqual(r['charged_duplicate_bytes'], 511)
        self.assertTrue((self.root / 'action/after-unlink.json').exists())

    def test_post_unlink_changed_original_detected(self):
        def hook(stage):
            if stage == 'after-unlink':
                self.original.write_bytes(b'changed')
        with self.assertRaises(ValueError):
            self.run_transition(hook=hook)
        self.assertTrue(read_json(self.root / 'action/failed.json')['unlink_performed'])

    def test_durable_records_never_overwrite(self):
        path = self.root / 'record.json'
        save(path, {'original': True})
        before = path.read_bytes()
        with self.assertRaises(FileExistsError):
            save(path, {'replacement': True})
        self.assertEqual(path.read_bytes(), before)

    def test_repeat_action_never_overwrites_original_transition(self):
        self.run_transition()
        before = (self.root / 'action/final.json').read_bytes()
        with self.assertRaises(FileExistsError):
            self.run_transition()
        self.assertEqual((self.root / 'action/final.json').read_bytes(), before)

    def test_missing_or_wrong_review_does_not_start_action(self):
        sandbox = self.root / 'whole-prefix-reclamation-v4'
        sandbox.mkdir()
        with patch.object(reclaim, 'ROOT', sandbox):
            with self.assertRaises(FileNotFoundError):
                reclaim.authorize('0' * 64)
            save(sandbox / 'review.json', {'status': 'unreviewed'})
            with self.assertRaises(ValueError):
                reclaim.authorize('0' * 64)
        self.assertFalse((sandbox / 'action-1').exists())
        self.assertTrue(self.target.exists())

    def test_reconstruction_exact_and_no_overwrite(self):
        dest = self.root / 'reconstructed-prefix'
        self.assertEqual(reconstruct(self.plan, dest), self.plan['recipe']['prefix'])
        self.assertEqual(dest.read_bytes(), self.target.read_bytes())
        with self.assertRaises(FileExistsError):
            reconstruct(self.plan, dest)

    def test_reconstruction_checks_original_before_creating_output(self):
        dest = self.root / 'reconstructed-prefix'
        self.original.write_bytes(self.original.read_bytes()[:-1] + b'!')
        self.plan['original']['state'] = state(self.original.stat())
        with self.assertRaises(ValueError):
            reconstruct(self.plan, dest)
        self.assertFalse(dest.exists())


class BirthTests(unittest.TestCase):
    def setUp(self):
        self.cutoff = birth_ns('Fri Oct  2 12:00:00 2026')
        self.now = self.cutoff + 60_000_000_000

    def row(self, pid=42, parent=1, group=42, second=10):
        return f'{pid} {parent} {group} Fri Oct  2 12:00:{second:02d} 2026\n'.encode()

    def test_old_and_ambiguous_reuse_blocked(self):
        for second in [0, 1, 2]:
            with self.subTest(second=second), self.assertRaises(ValueError):
                classify(self.row(second=second), [42], self.cutoff, self.now)

    def test_later_reused_pid_and_group_allowed(self):
        r = classify(self.row(), [42], self.cutoff, self.now)
        self.assertEqual(len(r['reused']), 1)

    def test_old_leaderless_group_and_child_blocked(self):
        for row in [self.row(pid=99, group=42, second=0), self.row(pid=99, parent=42, group=99, second=0)]:
            with self.assertRaises(ValueError):
                classify(row, [42], self.cutoff, self.now)

    def test_malformed_empty_duplicate_and_future_rejected(self):
        for data in [b'', b'bad', self.row() * 2, self.row(second=59)]:
            now = self.cutoff + 30_000_000_000
            with self.subTest(data=data), self.assertRaises(ValueError):
                classify(data, [42], self.cutoff, now)

    def test_nonmatching_old_process_is_not_an_owner(self):
        self.assertTrue(classify(self.row(pid=5, group=5, second=0), [42], self.cutoff, self.now)['no_live_prior_owner'])

    def test_inconsistent_weekday_rejected(self):
        with self.assertRaises(ValueError):
            birth_ns('Thu Oct  2 12:00:00 2026')


if __name__ == '__main__':
    unittest.main()
