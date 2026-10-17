"""Later absence observations must never rehabilitate an incomplete shutdown."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock
import prior_quiescence_v4 as Q
from prior_fixture_v4 import write, identity


class HistoricalAbsence(unittest.TestCase):
    def setUp(self):
        temp=tempfile.TemporaryDirectory();self.addCleanup(temp.cleanup)
        self.base=Path(temp.name).resolve();self.shared=self.base/'whole-proof-attacks-completion-v3'
        self.root=self.base/'whole-v3-emergency-quiescence'
        self.run=self.shared/'run-1/receipt.json'
        write(self.run,dict(status='failed',cleanup_error='historical cleanup failed',ended_ns=100,
                           children_final={'1':dict(exit_code=None),'2':dict(exit_code=None)}))
        write(self.shared/'admission.json',dict(pid=900,children={'1':901,'2':902}))
        write(self.shared/'stop.json',dict(reason='wall'))
        reg=self.shared/'native-processes/native.json'
        write(reg,dict(pid=903,pgid=903,coordinator=900,parent=902))
        write(self.root/'original-registrations/native.json',json.loads(reg.read_text()))
        first=dict(schema='trident/v3-post-failure-process-observation/v1',status='observed-quiescent',
                   started_ns=200,observed_ns=201,command=['/bin/ps','-axo','pid=,ppid=,pgid=,lstart=,stat=,comm='],
                   source_registrations={str(reg):identity(reg)},coordinator_suite_rows=[],signals=[],
                   originals_modified=False,partials_deleted=False)
        raw=b'1 0 1 Wed Sep 23 18:00:12 2026 Ss /sbin/launchd\n'
        for name,data in (('ps-first.stdout',raw),('ps-second.stdout',raw),('ps-first.stderr',b''),('ps-second.stderr',b'')):
            (self.root/name).write_bytes(data)
        first.update(ps=identity(self.root/'ps-first.stdout'),stderr=identity(self.root/'ps-first.stderr'))
        write(self.root/'first.json',first)
        self.q=dict(schema='trident/v3-post-failure-quiescence/v1',status='observed-quiescent',
                    first=identity(self.root/'first.json'),second_observed_ns=300,
                    second_ps=identity(self.root/'ps-second.stdout'),second_stderr=identity(self.root/'ps-second.stderr'),
                    registration_count=1,registered_pgids=[903],coordinator_and_suites=[900,901,902],
                    matching_current_processes=[],signals_sent=[],partials_deleted=False,
                    original_failed_evidence_unchanged={str(p):identity(p) for p in (self.run,self.shared/'admission.json',self.shared/'stop.json')})
        self.path=self.root/'receipt.json';write(self.path,self.q)
    def review(self):
        with mock.patch.object(Q,'QUIESCENCE_SHA',identity(self.path)['sha256']):return Q.review_quiescence(self.base)
    def test_later_observation_retains_failed_shutdown(self):
        row=self.review();self.assertIn('not relabelled',row['claim'])
    def test_successful_shutdown_relabel_is_rejected(self):
        row=json.loads(self.run.read_text());row['status']='passed';write(self.run,row)
        with self.assertRaisesRegex(ValueError,'failed shutdown remains failed'):self.review()
    def test_invented_successful_child_exit_is_rejected(self):
        row=json.loads(self.run.read_text());row['children_final']['2']['exit_code']=0;write(self.run,row)
        with self.assertRaisesRegex(ValueError,'failed shutdown remains failed'):self.review()
    def test_snapshot_before_failed_shutdown_is_rejected(self):
        first=json.loads((self.root/'first.json').read_text());first['started_ns']=99;write(self.root/'first.json',first)
        self.q['first']=identity(self.root/'first.json');write(self.path,self.q)
        with self.assertRaisesRegex(ValueError,'later ordered observations'):self.review()
    def test_hidden_live_group_is_rejected_even_with_updated_hash(self):
        p=self.root/'ps-second.stdout';p.write_text('904 1 903 Fri Oct 2 10:00:00 2026 S helper\n')
        self.q['second_ps']=identity(p);write(self.path,self.q)
        with self.assertRaisesRegex(ValueError,'owned process remains live'):self.review()
    def test_incomplete_registration_set_rejected(self):
        self.q['registered_pgids']=[];write(self.path,self.q)
        with self.assertRaisesRegex(ValueError,'complete historical ownership'):self.review()


if __name__=='__main__':unittest.main(verbosity=2)
