"""Construction-only admission and stable identity adversarial tests, tiny files."""
import copy
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock
import prior_cases_v4 as M
import prior_quiescence_v4 as Q
from prior_fixture_v4 import write, identity


class CostFixture:
    def __init__(self,base):
        self.base=base;self.root=base/'whole-proof-attacks-v3-c2';self.work=self.root/'whole-c2'
        self.proof=self.put(base/'whole-proof/attempts/c2-selfbuild-1/proof.joysc',b'proof123')
        self.verifier=base/'whole-proof/attempts/c2-fresh-verification-1'
        self.certificate=self.put(self.work/'certificate-cost.joysc',b'mutant12')
        self.cost=identity(self.certificate)
        self.helper=self.put(self.root/'target-helper/release/whole-proof-mutator',b'not executable')
        self.index=self.work/'index.json'
        write(self.index,dict(decoded_bytes=80,frames=2,records=7))
        self.put(self.work/'result.dag',b'canonical result')
        self.joy=self.put(base/'production-install/installed/bin/joy',b'not executable')
        self.compiler=self.put(self.root/'inputs/frozen/c2.dag',b'compiler')
        self.job=self.put(self.root/'inputs/frozen/c2-job.dag',b'job')
        self.output=self.put(self.work/'cost.dag',b'protected')
        self.profile=dict(host_flags=['--budget','99'],wire_bytes=100,decoded_bytes=200,records=9,steps=10,cache_slots=2)
        self.prior=dict(original_proof=identity(self.proof),profile=self.profile)
        self.construct=self.root/'attempts/whole-c2-construct-cost'
        write(self.construct/'stdout',dict(decoded_bytes=80,fixed_changed_bytes=0,frames=2,mode='cost',source_records=7,
                                           source_sha256=identity(self.proof)['sha256'],terminal_suffix_rebuilt=True,
                                           wire_bytes=self.cost['bytes']))
        write(self.construct/'receipt.json',{'status':'passed','synthetic':True})
        self.interrupted=self.root/'attempts/whole-c2-verify-cost'
        inputs={str(p):identity(p) for p in (self.joy,self.compiler,self.job,self.output,self.certificate)}
        self.row=dict(schema='trident/whole-proof-attack-command/v2',status='failed',exit_code=-15,expected_exit=1,
                      resource_stop='shared-stop',argv=list(map(str,[self.joy,'verify-artifact',self.compiler,'--input',self.job,
                          '--proof',self.certificate,'--output',self.output,'--emit','program',*M.C.profile_flags(self.profile),'--force'])),
                      cwd=str(self.interrupted),environment={'PATH':''},metadata=dict(generation=2,expected_error=M.ERRORS['cost']),
                      inputs_before=inputs,inputs_after=inputs)
        for name,data in (('stdout',b''),('stderr',b''),('resources.jsonl',b'{}\n')):self.put(self.interrupted/name,data)
        self.row['files']={p.name:identity(p) for p in self.interrupted.iterdir()}
        write(self.interrupted/'receipt.json',self.row)
    @staticmethod
    def put(path,data):path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(data);return path
    def admit(self):
        # Eligibility/resource diagnostics are independently tested with constructed receipt trees.
        with mock.patch.object(M,'review',return_value=self.prior),mock.patch.object(M,'v3_pins',return_value={}), \
             mock.patch.object(M,'diagnostic',return_value=dict(elapsed_seconds=2,started_ns=100,ended_ns=10**10)), \
             mock.patch.object(M,'review_quiescence',return_value={'synthetic':'historical absence'}), \
             mock.patch.object(M,'observe_current',return_value={'synthetic':'current absence'}), \
             mock.patch.object(M,'C2_COST',self.cost):
            return M.admit_c2_cost(self.base,self.proof,self.verifier,{'synthetic':'expected result'})


class CostAdmission(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.f=CostFixture(Path(self.temp.name).resolve())
    def edit(self,path,change):value=json.loads(path.read_text());change(value);write(path,value)
    def reject_receipt(self,change,message):
        self.edit(self.f.interrupted/'receipt.json',change)
        with self.assertRaisesRegex(ValueError,message):self.f.admit()
    def test_admits_construction_without_admitting_rejection_or_deletion(self):
        value=self.f.admit()
        self.assertEqual(value['status'],'passed-construction-admission')
        self.assertFalse(value['accepted_as_rejection']);self.assertFalse(value['deletion_authorized'])
        self.assertTrue(value['fresh_verification_required'])
        self.assertEqual(value['certificate']['stat']['inode'],self.f.certificate.stat().st_ino)
        observed=self.f.certificate.stat()
        if hasattr(observed,'st_birthtime'):
            self.assertEqual(value['certificate']['stat']['birthtime_seconds'],observed.st_birthtime)
            self.assertIsNotNone(value['certificate']['stat']['birthtime_ns'])
        self.assertEqual(value['interrupted_verification']['status'],'failed')
    def test_old_verifier_cannot_be_relabelled_success(self):
        self.reject_receipt(lambda x:x.update(status='passed',exit_code=1),'old cost verifier is interrupted')
    def test_wrong_failed_verifier_input(self):
        self.reject_receipt(lambda x:x['argv'].__setitem__(2,'another compiler'),'exact interrupted verifier provenance')
    def test_wrong_old_cost_identity(self):
        self.reject_receipt(lambda x:x['inputs_before'][str(self.f.certificate)].update(sha256='0'*64),'exact interrupted verifier provenance')
    def test_old_unexpected_verdict(self):
        p=self.f.interrupted/'stderr';p.write_bytes(b'semantic terminal: Claim')
        self.reject_receipt(lambda x:x['files'].update(stderr=identity(p)),'did not produce a terminal verdict')
    def test_wrong_source_in_construction(self):
        self.edit(self.f.construct/'stdout',lambda x:x.update(source_sha256='0'*64))
        with self.assertRaisesRegex(ValueError,'exact complete cost-construction output'):self.f.admit()
    def test_missing_terminal_rebuild(self):
        self.edit(self.f.construct/'stdout',lambda x:x.update(terminal_suffix_rebuilt=False))
        with self.assertRaisesRegex(ValueError,'exact complete cost-construction output'):self.f.admit()
    def test_truncated_retained_file(self):
        self.f.certificate.write_bytes(b'x')
        with self.assertRaisesRegex(ValueError,'complete byte count'):self.f.admit()
    def test_changed_retained_bytes(self):
        self.f.certificate.write_bytes(b'changed!')
        with self.assertRaisesRegex(ValueError,'complete file hash'):self.f.admit()
    def test_link_is_rejected(self):
        target=self.f.certificate.with_suffix('.other');self.f.certificate.rename(target);self.f.certificate.symlink_to(target)
        with self.assertRaisesRegex(ValueError,'regular single-link'):self.f.admit()
    def test_additional_hardlink_is_rejected(self):
        os.link(self.f.certificate,self.f.certificate.with_suffix('.other'))
        with self.assertRaisesRegex(ValueError,'regular single-link'):self.f.admit()
    def test_path_replacement_during_hash_is_rejected(self):
        real=os.read;changed=False
        def swap(fd,size):
            nonlocal changed
            value=real(fd,size)
            if not changed:
                changed=True
                new=self.f.certificate.with_suffix('.replacement');new.write_bytes(b'mutant12');os.replace(new,self.f.certificate)
            return value
        with mock.patch.object(Q.os,'read',side_effect=swap):
            with self.assertRaisesRegex(ValueError,'inode stable'):self.f.admit()


class BirthObservations(unittest.TestCase):
    cutoff=int(datetime(2026,10,2,10,0,0,tzinfo=timezone.utc).timestamp()*1e9)
    def rows(self,raw):return Q.inspect_birth_rows(raw,{123},{456},self.cutoff)
    def test_old_owner_is_rejected(self):
        with self.assertRaisesRegex(ValueError,'original or ambiguous'):self.rows('123 123 Fri Oct  2 09:00:00 2026\n')
    def test_new_pid_reuse_is_explicit(self):
        value=self.rows('456 789 Fri Oct  2 11:00:00 2026\n')
        self.assertEqual(value[0]['classification'],'reused-after-observed-absence')
    def test_new_pgid_reuse_is_explicit(self):self.assertEqual(len(self.rows('789 123 Fri Oct  2 11:00:00 2026\n')),1)
    def test_old_leaderless_group_rejected(self):
        with self.assertRaisesRegex(ValueError,'original or ambiguous'):self.rows('789 123 Fri Oct  2 09:00:00 2026\n')
    def test_rounding_boundary_rejected(self):
        with self.assertRaisesRegex(ValueError,'original or ambiguous'):self.rows('789 123 Fri Oct  2 10:00:02 2026\n')
    def test_ambiguous_birth_rejected(self):
        with self.assertRaises(ValueError):self.rows('789 123 unknown\n')
    def test_duplicate_rows_rejected(self):
        with self.assertRaisesRegex(ValueError,'unique current'):self.rows('789 900 Fri Oct  2 09:00:00 2026\n789 900 Fri Oct  2 09:00:00 2026\n')
    def test_empty_snapshot_rejected(self):
        with self.assertRaisesRegex(ValueError,'nonempty current'):self.rows('')


if __name__=='__main__':unittest.main(verbosity=2)
