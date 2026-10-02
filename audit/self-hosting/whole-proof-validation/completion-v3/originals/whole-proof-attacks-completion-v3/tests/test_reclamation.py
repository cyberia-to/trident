"""Only tiny owned temporary files; never execute the reclamation action."""
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import reclaim_completed as reclaim


class ExactUnlink(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name).resolve()
        self.target = self.root / "owned-complete.joysc"
        self.target.write_bytes(b"classified owned temporary\n")
        self.state = reclaim.state(self.target.stat())
        self.identity = reclaim.identity(self.target)

    def tearDown(self):
        self.temp.cleanup()

    def unlink(self, callback=lambda _identity: None):
        return reclaim.unlink_exact(self.target, self.state, self.identity, callback)

    def test_success_removes_only_classified_owned_file_after_callback(self):
        sibling = self.root / "must-remain"
        sibling.write_bytes(b"unrelated preserved evidence")
        observed = []

        def durable(_identity):
            self.assertTrue(self.target.exists())
            self.assertEqual(_identity, self.identity)
            observed.append(_identity)
            reclaim.save(self.root / "journal.json", {"validated": _identity})

        self.assertEqual(self.unlink(durable), self.identity)
        self.assertFalse(self.target.exists())
        self.assertEqual(len(observed), 1)
        self.assertEqual(sibling.read_bytes(), b"unrelated preserved evidence")
        self.assertTrue((self.root / "journal.json").is_file())

    def test_changed_state_keeps_file(self):
        self.target.write_bytes(b"different state\n")
        with self.assertRaisesRegex(ValueError, "inode/state changed"):
            self.unlink()
        self.assertTrue(self.target.is_file())

    def test_changed_bytes_refused_even_if_current_state_is_supplied(self):
        self.target.write_bytes(b"x" * self.identity["bytes"])
        self.state = reclaim.state(self.target.stat())
        with self.assertRaisesRegex(ValueError, "bytes changed"):
            self.unlink()
        self.assertTrue(self.target.is_file())

    def test_symlink_keeps_referent_and_link(self):
        link = self.root / "link.joysc"
        link.symlink_to(self.target)
        with self.assertRaisesRegex(ValueError, "exact regular owned path"):
            reclaim.unlink_exact(link, self.state, self.identity, lambda _: None)
        self.assertTrue(link.is_symlink())
        self.assertEqual(reclaim.identity(self.target), self.identity)

    def test_extra_hard_link_refused(self):
        other = self.root / "second-link"
        os.link(self.target, other)
        self.state = reclaim.state(self.target.stat())
        with self.assertRaisesRegex(ValueError, "unique owned temporary link"):
            self.unlink()
        self.assertTrue(self.target.exists())
        self.assertTrue(other.exists())

    def test_callback_path_replacement_keeps_both_files(self):
        moved = self.root / "original-held-inode"

        def replace(_identity):
            self.target.rename(moved)
            self.target.write_bytes(b"replacement must remain")

        with self.assertRaisesRegex(ValueError, "changed after durable classification"):
            self.unlink(replace)
        self.assertEqual(reclaim.identity(moved), self.identity)
        self.assertEqual(self.target.read_bytes(), b"replacement must remain")

    def test_callback_failure_keeps_file(self):
        def refuse(_identity):
            raise RuntimeError("journal persistence failed")

        with self.assertRaisesRegex(RuntimeError, "journal persistence failed"):
            self.unlink(refuse)
        self.assertEqual(reclaim.identity(self.target), self.identity)

    def test_callback_mutation_keeps_changed_file(self):
        def alter(_identity):
            self.target.write_bytes(b"mutated during journal")

        with self.assertRaisesRegex(ValueError, "changed after durable classification"):
            self.unlink(alter)
        self.assertEqual(self.target.read_bytes(), b"mutated during journal")

    def test_pre_unlink_directory_sync_failure_keeps_file(self):
        def durable(_identity):
            reclaim.save(self.root / "journal.json", {"validated": _identity})

        with mock.patch.object(reclaim, "sync_directory", side_effect=OSError("journal directory sync failed")):
            with self.assertRaisesRegex(OSError, "journal directory sync failed"):
                self.unlink(durable)
        self.assertEqual(reclaim.identity(self.target), self.identity)

    def test_post_unlink_sync_failure_records_actual_removal(self):
        report = dict(removed=False)

        def removed():
            self.assertFalse(self.target.exists())
            report["removed"] = True

        with mock.patch.object(reclaim, "sync_directory", side_effect=OSError("target directory sync failed")):
            with self.assertRaisesRegex(OSError, "target directory sync failed"):
                reclaim.unlink_exact(self.target, self.state, self.identity, lambda _: None, removed)
        self.assertTrue(report["removed"])
        self.assertFalse(self.target.exists())


if __name__ == "__main__":
    unittest.main(verbosity=2)
