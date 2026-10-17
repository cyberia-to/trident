"""Exercise actual bounded subprocesses and shared accounting in isolated scopes."""
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

BASE = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(BASE / 'whole-proof-attacks-parallel-v2'))
import resources


class ParallelGuards(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.saved = {k: getattr(resources, k) for k in ('ROOT', 'ROOTS', 'ORIGINAL', 'PLAN')}
        base = Path(self.temp.name).resolve()
        resources.ROOT = base / 'shared'
        resources.ROOTS = {1: base / 'c1', 2: base / 'c2'}
        resources.ORIGINAL = base / 'original'
        resources.PLAN = dict(resources.PLAN)
        for p in (resources.ROOT, resources.ORIGINAL, *resources.ROOTS.values()):
            p.mkdir()
        resources.write(resources.ROOT / 'plan.json', resources.PLAN)
        # Actual direct native commands belong to this process for these tests.
        resources.write(resources.ROOT / 'admission.json', dict(pid=os.getppid(), children={'1': os.getpid()}))
        spec = importlib.util.spec_from_file_location('isolated_guard', BASE / 'whole-proof-attacks-v2-c1/guard.py')
        self.guard = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.guard)
        self.guard.ROOT = resources.ROOTS[1]
        self.executable = str(Path(sys.executable).resolve())

    def tearDown(self):
        for k, v in self.saved.items():
            setattr(resources, k, v)
        self.temp.cleanup()

    def test_shared_accounting_includes_both_mutants_and_retained_partial(self):
        (resources.ORIGINAL / 'partial').write_bytes(b'p' * 7)
        baseline = resources.inventory()
        for g, root in resources.ROOTS.items():
            work = root / f'whole-c{g}'
            work.mkdir()
            (work / 'certificate-test.joysc').write_bytes(b'x' * g)
        sample = resources.sample(baseline, os.getpid())
        self.assertEqual(sample['owned_bytes'], sum(baseline.values()) + 3)
        self.assertEqual(len(sample['generations']['1']['mutants']), 1)
        self.assertEqual(len(sample['generations']['2']['mutants']), 1)
        self.assertIsNone(sample['reason'])

    def test_second_mutant_fails_shared_reservation(self):
        baseline = resources.inventory()
        work = resources.ROOTS[1] / 'whole-c1'
        work.mkdir()
        for name in ('a', 'b'):
            (work / f'certificate-{name}.joysc').write_bytes(b'x')
        sample = resources.sample(baseline, os.getpid())
        self.assertEqual(sample['reason'], 'generation-mutant-reservation')
        self.assertTrue(resources.stop_requested())

    def test_other_generation_cannot_be_reclaimed(self):
        work = resources.ROOTS[2] / 'whole-c2'
        work.mkdir()
        path = work / 'certificate-test.joysc'
        path.write_bytes(b'x')
        with self.assertRaises((ValueError, KeyError)):
            resources.reclaim(path)
        self.assertTrue(path.exists())

    def test_stop_retains_own_mutant(self):
        work = resources.ROOTS[1] / 'whole-c1'
        work.mkdir()
        path = work / 'certificate-test.joysc'
        path.write_bytes(b'x')
        resources.fail('intentional-test')
        with self.assertRaises(ValueError):
            resources.reclaim(path)
        self.assertTrue(path.exists())

    def test_actual_log_overflow_is_resource_failure(self):
        with self.assertRaises(RuntimeError):
            self.guard.run('log-overflow', [self.executable, '-c', 'import os; os.write(1,b"x"*2097152)'], 'pack', [])
        d = resources.ROOTS[1] / 'attempts/log-overflow'
        receipt = json.loads((d / 'receipt.json').read_text())
        self.assertEqual(receipt['status'], 'failed')
        self.assertIn(receipt['resource_stop'], ('log-bytes', 'shared-stop'))
        self.assertEqual((d / 'stdout').stat().st_size, 1048576)
        self.assertTrue(resources.stop_requested())
        self.assertTrue((resources.ROOT / 'native-processes/log-overflow.json').exists())

    def test_actual_expected_rejection_without_resource_failure(self):
        receipt = self.guard.run('expected-rejection', [self.executable, '-c', 'import sys; sys.stderr.write("specific rejection\\n"); sys.exit(1)'], 'pack', [], expected_exit=1)
        self.assertEqual(receipt['status'], 'passed')
        self.assertFalse(resources.stop_requested())
        self.assertNotIn('resource_stop', receipt)

    def test_registered_native_group_survives_parent_exit_and_is_stopped(self):
        coordinator_spec = importlib.util.spec_from_file_location('isolated_coordinator', BASE / 'whole-proof-attacks-parallel-v2/run.py')
        coordinator = importlib.util.module_from_spec(coordinator_spec)
        coordinator_spec.loader.exec_module(coordinator)
        code = """import os, subprocess, sys, time
from pathlib import Path
sys.path.insert(0, sys.argv[1])
import resources
resources.ROOT = Path(sys.argv[2])
resources.write(resources.ROOT / 'admission.json', dict(pid=int(sys.argv[3]),children={'1':os.getpid()}))
argv=[sys.executable,'-c','import signal,time; signal.signal(signal.SIGTERM,signal.SIG_IGN); time.sleep(60)']
child=subprocess.Popen(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True,
                       preexec_fn=lambda: resources.register_native('orphan-test',argv))
time.sleep(0.3)
print(child.pid,flush=True)
"""
        parent = subprocess.Popen([self.executable, '-B', '-c', code,
                    str(BASE / 'whole-proof-attacks-parallel-v2'), str(resources.ROOT), str(os.getpid())],
                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
        stdout, stderr = parent.communicate(timeout=10)
        self.assertEqual(parent.returncode, 0, stderr)
        native_pid = int(stdout.strip())
        try:
            owned = resources.process_snapshot(os.getpid())
            self.assertIn(native_pid, [row['pid'] for row in owned])
            self.assertNotEqual(next(row['ppid'] for row in owned if row['pid'] == native_pid), parent.pid)
            coordinator.stop_all({1: parent})
            deadline = __import__('time').monotonic() + 5
            while coordinator.process_identity(native_pid) is not None and __import__('time').monotonic() < deadline:
                __import__('time').sleep(0.1)
            self.assertIsNone(coordinator.process_identity(native_pid))
        finally:
            try:
                os.killpg(native_pid, 9)
            except ProcessLookupError:
                pass

    def test_registered_leaderless_native_group_is_stopped(self):
        coordinator_spec = importlib.util.spec_from_file_location('isolated_coordinator', BASE / 'whole-proof-attacks-parallel-v2/run.py')
        coordinator = importlib.util.module_from_spec(coordinator_spec)
        coordinator_spec.loader.exec_module(coordinator)
        code = """import os, subprocess, sys, time
from pathlib import Path
sys.path.insert(0, sys.argv[1])
import resources
resources.ROOT = Path(sys.argv[2])
resources.write(resources.ROOT / 'admission.json', dict(pid=int(sys.argv[3]),children={'1':os.getpid()}))
native_code='import os,signal,time; from pathlib import Path; signal.signal(signal.SIGTERM,signal.SIG_IGN); pid=os.fork(); os._exit(0) if pid else None; Path('+repr(str(resources.ROOT / 'descendant.pid'))+').write_text(str(os.getpid())); time.sleep(60)'
argv=[sys.executable,'-c',native_code]
child=subprocess.Popen(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True,
                       preexec_fn=lambda: resources.register_native('orphan-test',argv))
time.sleep(0.3)
print(child.pid,flush=True)
"""
        parent = subprocess.Popen([self.executable, '-B', '-c', code,
                    str(BASE / 'whole-proof-attacks-parallel-v2'), str(resources.ROOT), str(os.getpid())],
                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
        stdout, stderr = parent.communicate(timeout=10)
        self.assertEqual(parent.returncode, 0, stderr)
        native_group = int(stdout.strip())
        native_pid = int((resources.ROOT / 'descendant.pid').read_text())
        self.assertNotEqual(native_pid, native_group)
        try:
            owned = resources.process_snapshot(os.getpid())
            self.assertIn(native_pid, [row['pid'] for row in owned])
            self.assertNotEqual(next(row['ppid'] for row in owned if row['pid'] == native_pid), parent.pid)
            coordinator.stop_all({1: parent})
            deadline = __import__('time').monotonic() + 5
            while coordinator.process_identity(native_pid) is not None and __import__('time').monotonic() < deadline:
                __import__('time').sleep(0.1)
            self.assertIsNone(coordinator.process_identity(native_pid))
        finally:
            try:
                os.killpg(native_group, 9)
            except ProcessLookupError:
                pass


if __name__ == '__main__':
    unittest.main(verbosity=2)
