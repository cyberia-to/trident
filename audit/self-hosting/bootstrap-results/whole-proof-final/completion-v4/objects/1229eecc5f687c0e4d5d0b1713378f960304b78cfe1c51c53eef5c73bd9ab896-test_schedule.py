"""Adversarial admission and process-ownership regressions; no proof workloads."""
import copy
import errno
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

import ownership
import resources
import run


def row(pid, group=50, born='original', state='S'):
    return dict(pid=pid, pgid=group, started=born, state=state, ppid=1, rss_kib=10)


class OwnershipTests(unittest.TestCase):
    def binding(self):
        return dict(pid=50, pgid=50, started='original', members={'50': 'original'})

    def test_reused_identifier_never_becomes_owned(self):
        self.assertEqual('unbound', ownership.observe(self.binding(), [row(50, born='later')])['status'])

    def test_known_descendant_keeps_leaderless_group_bound(self):
        record = self.binding()
        self.assertEqual('owned', ownership.observe(record, [row(50), row(51)])['status'])
        self.assertEqual('owned', ownership.observe(record, [row(51)])['status'])

    def test_unknown_leaderless_group_is_not_adopted(self):
        self.assertEqual('unbound', ownership.observe(self.binding(), [row(51)])['status'])

    def test_empty_is_observed(self):
        self.assertEqual('empty', ownership.observe(self.binding(), [row(60, group=60)])['status'])

    def test_unbound_group_never_signalled(self):
        with patch.object(ownership, 'snapshot', return_value=[row(50, born='later')]), \
                patch.object(ownership.os, 'killpg') as signal:
            result = ownership.stop_group(self.binding(), grace=0, final_wait=0)
        signal.assert_not_called()
        self.assertEqual('failed', result['status'])

    def test_permission_error_is_retained_and_not_success(self):
        with patch.object(ownership, 'snapshot', return_value=[row(50, state='Z')]), \
                patch.object(ownership.os, 'killpg', side_effect=PermissionError(errno.EPERM, 'denied')):
            result = ownership.stop_group(self.binding(), grace=0, final_wait=0)
        self.assertEqual('failed', result['status'])
        self.assertEqual([errno.EPERM, errno.EPERM], [r['errno'] for r in result['events'] if 'errno' in r])

    def test_empty_after_permission_error_is_separate_observation(self):
        with patch.object(ownership, 'snapshot', side_effect=[[row(50)], []]), \
                patch.object(ownership.os, 'killpg', side_effect=PermissionError(errno.EPERM, 'denied')):
            result = ownership.stop_group(self.binding(), grace=0, final_wait=0)
        self.assertEqual('passed', result['status'])
        self.assertEqual(errno.EPERM, next(r['errno'] for r in result['events'] if 'errno' in r))

    def test_one_cleanup_failure_does_not_skip_other_groups(self):
        records = {1: self.binding(), 2: dict(pid=80, pgid=80, started='other')}
        children = {1: SimpleNamespace(pid=50), 2: SimpleNamespace(pid=80)}
        with tempfile.TemporaryDirectory() as temp, patch.object(run, 'ROOT', Path(temp)), \
                patch.object(resources, 'ROOT', Path(temp)), \
                patch.object(ownership, 'stop_child', side_effect=[PermissionError('first'), {'status': 'passed'}]) as stop:
            result = run.stop_all(children, records)
        self.assertEqual(2, stop.call_count)
        self.assertEqual('failed', result['status'])
        self.assertEqual('passed', result['groups'][1]['status'])

    def test_malformed_registry_does_not_skip_known_child(self):
        with tempfile.TemporaryDirectory() as temp, patch.object(run, 'ROOT', Path(temp)), \
                patch.object(resources, 'ROOT', Path(temp)), \
                patch.object(ownership, 'stop_child', return_value={'status': 'passed'}) as stop:
            registry = Path(temp) / 'native-processes'
            registry.mkdir()
            (registry / 'broken.json').write_text('{')
            child = SimpleNamespace(pid=50)
            result = run.stop_all({1: child}, {1: self.binding()})
        stop.assert_called_once_with(child, self.binding())
        self.assertEqual('failed', result['status'])
        self.assertEqual('passed', result['groups'][1]['status'])

    def test_late_native_registration_is_swept_after_suites(self):
        with tempfile.TemporaryDirectory() as temp, patch.object(run, 'ROOT', Path(temp)), \
                patch.object(resources, 'ROOT', Path(temp)):
            registry = Path(temp) / 'native-processes'
            registry.mkdir()
            def late(_child, _binding):
                resources.write(registry / 'late.json', self.binding())
                return dict(status='passed')
            with patch.object(ownership, 'stop_child', side_effect=late), \
                    patch.object(ownership, 'stop_group', return_value={'status': 'passed'}) as stop:
                result = run.stop_all({1: SimpleNamespace(pid=80)}, {1: dict(pid=80, pgid=80, started='other')})
        stop.assert_called_once()
        self.assertEqual('after-suite-shutdown', result['groups'][-1]['sweep'])
        self.assertEqual('passed', result['status'])

    def test_unbound_started_child_is_still_handled(self):
        child = Mock(pid=50)
        child.poll.return_value = None
        live = dict(row(50), ppid=ownership.os.getpid())
        with patch.object(ownership, 'snapshot', return_value=[live]), \
                patch.object(ownership, 'stop_group', return_value={'status': 'passed'}) as stop:
            result = ownership.stop_child(child)
        self.assertTrue(result['binding_recovered_from_live_direct_child'])
        self.assertEqual(stop.call_args.args[0]['started'], 'original')

    def test_snapshot_failure_only_signals_owned_direct_child(self):
        child = Mock(pid=50)
        child.poll.return_value = None
        with patch.object(ownership, 'snapshot', side_effect=OSError('snapshot unavailable')), \
                patch.object(ownership.os, 'killpg') as signal_group:
            result = ownership.stop_child(child)
        child.terminate.assert_called_once()
        signal_group.assert_not_called()
        self.assertEqual('failed', result['status'])

    def test_finished_child_without_binding_is_not_claimed_never_started(self):
        child = Mock(pid=50, returncode=0)
        child.poll.return_value = 0
        result = ownership.stop_child(child)
        self.assertEqual('failed', result['status'])
        self.assertTrue(result['binding_missing'])

    def test_snapshot_failure_with_original_binding_still_cleans_direct_child(self):
        child = Mock(pid=50)
        child.poll.return_value = None
        with patch.object(ownership, 'snapshot', side_effect=OSError('snapshot unavailable')), \
                patch.object(ownership.os, 'killpg') as signal_group:
            result = ownership.stop_child(child, self.binding())
        child.terminate.assert_called_once()
        signal_group.assert_not_called()
        self.assertEqual('failed', result['status'])
        self.assertFalse(result['binding_missing'])

    def test_process_inventory_is_bounded_and_utc(self):
        with patch.object(ownership.subprocess, 'check_output', return_value='50 1 50 10 S Fri Oct  2 12:00:00 2026\n') as ps:
            ownership.snapshot()
        self.assertEqual(15, ps.call_args.kwargs['timeout'])
        self.assertEqual('UTC', ps.call_args.kwargs['env']['TZ'])


class AdmissionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.patch = patch.object(resources, 'ROOT', self.root)
        self.patch.start()
        self.addCleanup(self.patch.stop)

    def test_prelude_accepts_only_exact_c2_cost_command(self):
        resources.authorize_native(2, 'whole-c2-verify-cost', resources.PLAN['prelude_command'], 'verify')

    def test_prelude_blocks_other_generation(self):
        with self.assertRaisesRegex(ValueError, 'restricted prelude'):
            resources.authorize_native(1, 'whole-c2-verify-cost', resources.PLAN['prelude_command'], 'verify')

    def test_prelude_blocks_helper(self):
        with self.assertRaisesRegex(ValueError, 'restricted prelude'):
            resources.authorize_native(2, 'whole-c2-construct-cost', ['/helper', 'mutate'], 'helper')

    def test_prelude_rejects_even_one_changed_verifier_argument(self):
        command = resources.PLAN['prelude_command'].copy()
        command[-1] = '--changed'
        with self.assertRaisesRegex(ValueError, 'restricted prelude'):
            resources.authorize_native(2, 'whole-c2-verify-cost', command, 'verify')

    def test_full_admission_bound_to_current_initial_receipt(self):
        resources.write(self.root / 'admission.json', {'actual': 'a'})
        resources.write(self.root / 'full-admission.json', {'admission': resources.identity(self.root / 'admission.json')})
        resources.authorize_native(1, 'construct', ['/helper'], 'helper')
        resources.write(self.root / 'admission.json', {'actual': 'b'})
        with self.assertRaisesRegex(ValueError, 'identity mismatch'):
            resources.authorize_native(1, 'construct', ['/helper'], 'helper')

    def test_stop_blocks_even_valid_prelude(self):
        resources.fail('test failure')
        with self.assertRaisesRegex(ValueError, 'shared stop'):
            resources.authorize_native(2, 'whole-c2-verify-cost', resources.PLAN['prelude_command'], 'verify')

    def test_plan_preserves_original_resource_caps(self):
        old = run.load(run.BASE / 'whole-proof-attacks-completion-v3/plan.json')
        for key in ('shared_disk_bytes', 'free_floor_bytes', 'combined_sampled_rss_bytes',
                    'mutant_growth_ceiling_bytes', 'canonical_and_output_bucket_per_generation',
                    'metadata_bucket_per_generation', 'per_stream_log_bytes', 'outer_schedule_seconds'):
            self.assertEqual(old[key], resources.PLAN[key], key)

    def test_full_reservation_charges_both_mutants(self):
        with patch.object(run.shutil, 'disk_usage', return_value=SimpleNamespace(free=100 * 1024**3)):
            prelude = run.reservation({'old': 1024}, False)
            full = run.reservation({'old': 1024}, True)
        expected = sum(row['bytes'] + resources.PLAN['mutant_growth_ceiling_bytes']
                       for row in resources.PLAN['proofs'].values())
        self.assertEqual(expected, full['reservation_bytes'] - prelude['reservation_bytes'])

    def test_full_reservation_cannot_project_future_retirement(self):
        with patch.object(run.shutil, 'disk_usage', return_value=SimpleNamespace(free=100 * 1024**3)):
            with self.assertRaisesRegex(ValueError, 'exceed shared cap'):
                run.reservation({'retained-cost': run.COST_ID['bytes']}, True)

    def test_free_floor_is_insufficient_without_growth_headroom(self):
        with patch.object(run.shutil, 'disk_usage', return_value=SimpleNamespace(free=resources.PLAN['free_floor_bytes'])):
            with self.assertRaisesRegex(ValueError, 'headroom'):
                run.reservation({}, True)


if __name__ == '__main__':
    unittest.main()
