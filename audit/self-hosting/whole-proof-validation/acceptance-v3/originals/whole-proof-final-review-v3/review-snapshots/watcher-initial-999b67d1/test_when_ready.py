"""Offline watcher gates; these tests never launch its checker or poll live jobs."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import when_ready as W


class WatcherTests(unittest.TestCase):
    def test_all_ten_actual_prerequisites_are_required(self):
        rows=W.prerequisites(Path('/synthetic'))
        self.assertEqual(len(rows),10)
        self.assertEqual(sum(v=='passed-completion' for v in rows.values()),2)
        self.assertTrue(all('/v2-c' not in str(p) for p in rows))

    def test_pending_and_failed_generation_cannot_launch(self):
        rows={'old-positive':dict(status='passed',required='passed'),
              'fresh-suite':dict(status='running',required='passed-completion')}
        self.assertFalse(W.verdict(rows))
        rows['fresh-suite']['status']='failed'
        with self.assertRaisesRegex(W.Blocked,'prerequisite failed'):
            W.verdict(rows)
        rows['fresh-suite']['status']='passed'
        with self.assertRaises(W.Blocked):
            W.verdict(rows)
        rows['fresh-suite']['status']='passed-completion'
        self.assertTrue(W.verdict(rows))

    def test_partial_receipt_waits_and_complete_bytes_are_bound(self):
        with tempfile.TemporaryDirectory() as directory:
            p=Path(directory)/'receipt.json'
            self.assertEqual(W.observe({p:'passed'})[str(p)]['status'],'waiting')
            p.write_text('{"status":')
            self.assertEqual(W.observe({p:'passed'})[str(p)]['status'],'waiting')
            p.write_text('{"status":"passed"}')
            self.assertEqual(W.observe({p:'passed'})[str(p)]['identity'],W.identity(p))

    def test_source_review_binds_watcher_and_checker(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            for name in W.SOURCE_NAMES:(root/name).write_text('synthetic source\n')
            manifest={str(root/name):W.identity(root/name) for name in W.SOURCE_NAMES}
            (root/'sources.json').write_text(json.dumps(manifest))
            (root/'independent-review.json').write_text(json.dumps(dict(status='passed-source-review',sources=manifest)))
            args=(root,W.identity(root/'sources.json')['sha256'],W.identity(root/'independent-review.json')['sha256'])
            self.assertEqual(W.source_gate(*args),manifest)
            (root/'check.py').write_text('changed source\n')
            with self.assertRaisesRegex(W.Blocked,'source changed'):
                W.source_gate(*args)

    def test_failed_prerequisite_writes_blocked_receipt_without_child(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            with patch.object(W,'ROOT',root),patch.object(W,'BASE',root),patch.object(W,'source_gate',return_value={}),\
                 patch.object(W,'observe',return_value={'gate':dict(status='failed',required='passed')}),\
                 patch.object(W.subprocess,'Popen') as child,\
                 patch('sys.argv',['when_ready.py','--source-manifest-sha256','00','--review-sha256','11']):
                with self.assertRaises(W.Blocked):W.main()
                child.assert_not_called()
            receipt=json.loads((root/'orchestration/receipt.json').read_text())
            self.assertEqual(receipt['status'],'blocked-prerequisite')
            self.assertFalse(receipt['checker_launched'])


if __name__=='__main__':unittest.main()
