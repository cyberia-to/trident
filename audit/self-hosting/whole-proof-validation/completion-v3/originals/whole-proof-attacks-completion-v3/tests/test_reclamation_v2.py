"""Repeat every exact-unlink regression against the separate v2 action copy."""
import test_reclamation as original_tests
import reclaim_completed_v2


class ExactUnlinkV2(original_tests.ExactUnlink):
    def setUp(self):
        previous = original_tests.reclaim
        self.addCleanup(setattr, original_tests, "reclaim", previous)
        original_tests.reclaim = reclaim_completed_v2
        super().setUp()
