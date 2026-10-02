"""Read-only process birth classification; no actual reclamation action."""
import os
from pathlib import Path
import subprocess
import sys
import time
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import reclaim_completed_v2 as reclaim


class ProcessIdentity(unittest.TestCase):
    def setUp(self):
        self.shutdown = reclaim.birth_ns("Fri Oct  2 09:30:25 2026") + 826_238_000
        self.observed = reclaim.birth_ns("Fri Oct  2 11:00:00 2026")
        self.groups = {84604, 59873, 59934}

    def classify(self, rows):
        return reclaim.classify_processes(rows.encode("ascii"), self.groups, self.shutdown, self.observed)

    def test_old_live_owner_rejected(self):
        with self.assertRaisesRegex(ValueError, "live prior owner"):
            self.classify("84604 84604 Fri Oct  2 06:11:13 2026\n")

    def test_reused_pid_and_group_after_empty_shutdown_accepted(self):
        raw = "84604 84604 Fri Oct  2 10:18:23 2026"
        result = self.classify(raw + "\n")
        self.assertTrue(result["no_live_prior_owners"])
        self.assertEqual(result["reused_rows"][0]["raw"], raw)

    def test_reused_pid_in_different_group_accepted(self):
        result = self.classify("84604 90001 Fri Oct  2 10:18:23 2026\n")
        self.assertEqual(len(result["reused_rows"]), 1)

    def test_old_leaderless_group_member_rejected(self):
        with self.assertRaisesRegex(ValueError, "live prior owner"):
            self.classify("90001 84604 Fri Oct  2 08:00:00 2026\n")

    def test_new_leaderless_group_member_after_shutdown_accepted(self):
        result = self.classify("90001 84604 Fri Oct  2 10:18:23 2026\n")
        self.assertEqual(result["reused_rows"][0]["pgid"], 84604)

    def test_one_old_member_blocks_other_new_group_members(self):
        with self.assertRaisesRegex(ValueError, "live prior owner"):
            self.classify("84604 84604 Fri Oct  2 10:18:23 2026\n90001 84604 Fri Oct  2 08:00:00 2026\n")

    def test_rounding_boundary_is_ambiguous(self):
        for second in (25, 26, 27):
            with self.subTest(second=second), self.assertRaisesRegex(ValueError, "ambiguous birth"):
                self.classify(f"84604 84604 Fri Oct  2 09:30:{second} 2026\n")
        self.assertTrue(self.classify("84604 84604 Fri Oct  2 09:30:28 2026\n")["no_live_prior_owners"])

    def test_future_overlapping_birth_rejected(self):
        with self.assertRaisesRegex(ValueError, "ambiguous birth"):
            self.classify("84604 84604 Fri Oct  2 12:00:00 2026\n")

    def test_malformed_or_missing_birth_rejected(self):
        for row in ("84604 84604", "84604 84604 unknown", "84604 84604 Fri Oct 2 25:00:00 2026",
                    "84604 84604 Thu Oct 2 10:18:23 2026", ""):
            with self.subTest(row=row), self.assertRaises(ValueError):
                self.classify(row + "\n")

    def test_duplicate_pid_snapshot_rejected(self):
        with self.assertRaisesRegex(ValueError, "duplicate process snapshot PID"):
            self.classify("84604 84604 Fri Oct  2 10:18:23 2026\n84604 84604 Fri Oct  2 10:18:23 2026\n")

    def test_unrelated_old_process_is_not_an_old_owner(self):
        result = self.classify("1 1 Fri Oct  2 00:00:00 2026\n")
        self.assertEqual(result["reused_rows"], [])

    def test_actual_current_process_before_cutoff_is_rejected(self):
        # Actual host process snapshot, no signalling or removal. The test
        # process existed before the cutoff and must never be called reuse.
        shutdown = time.time_ns()
        result = subprocess.run(["/bin/ps", "-axo", "pid=,pgid=,lstart="], capture_output=True,
                                check=True, timeout=15, env=dict(os.environ, TZ="UTC", LC_ALL="C", LANG="C"))
        with self.assertRaisesRegex(ValueError, "live prior owner"):
            reclaim.classify_processes(result.stdout, {os.getpid()}, shutdown, time.time_ns())


if __name__ == "__main__":
    unittest.main(verbosity=2)
