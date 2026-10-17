"""Offline contract/mutation checks only; no whole-proof acceptance result."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import cases
import common
import schedule


class ContextTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.compiler, self.job = self.root/'compiler.dag', self.root/'job.dag'
        self.program, self.formula, self.object = ('01'*32, '02'*32, '03'*32)
        self.compiler.write_bytes(b'NOXDAG01'+bytes.fromhex(self.program))
        self.job.write_bytes(b'NOXDAG01'+bytes.fromhex(self.object))
        self.coordinates = dict(program_particle=self.program, formula_particle=self.formula)
        self.admission = dict(compiler_particle=self.program, job_particle=self.object,
                              limits=dict(reductions=19999999999, evaluator_frames=65536))

    def call(self):
        return cases.admitted_context(self.coordinates, self.admission, self.compiler, self.job)

    def test_admitted_budget_not_host_ceiling(self):
        self.assertEqual(self.call(), [self.program, self.formula, self.object, '1', '19999999999', '65536'])
        wrong = [self.program, self.formula, self.object, '1', '20000000000', '65536']
        self.assertNotEqual(self.call(), wrong)

    def test_changed_job_or_compiler_header_rejected(self):
        for target in (self.job, self.compiler):
            original = target.read_bytes()
            target.write_bytes(b'NOXDAG01'+b'\x04'*32)
            with self.assertRaisesRegex(ValueError, 'coordinates'):
                self.call()
            target.write_bytes(original)

    def test_noncanonical_formula_and_bool_budget_rejected(self):
        self.coordinates['formula_particle'] = 'ff'*32
        with self.assertRaisesRegex(ValueError, 'canonical'):
            self.call()
        self.coordinates['formula_particle'] = self.formula
        self.admission['limits']['reductions'] = True
        with self.assertRaisesRegex(ValueError, 'bounded limits'):
            self.call()

    def test_missing_or_truncated_noun_header_rejected(self):
        self.job.write_bytes(b'NOXDAG01')
        with self.assertRaisesRegex(ValueError, 'header'):
            self.call()


class ProvenanceTests(unittest.TestCase):
    def test_exact_disjoint_9_plus_14_case_matrix(self):
        self.assertEqual((len(common.PRIOR), len(common.FRESH)), (9, 14))
        self.assertEqual(set(common.PRIOR) | set(common.FRESH), set(common.ERRORS))
        self.assertFalse(set(common.PRIOR) & set(common.FRESH))
        self.assertNotIn('rebound-job-limit', common.PRIOR)
        self.assertEqual(common.FRESH[0], 'rebound-job-limit')

    def test_duplicate_missing_or_relabelled_cases_rejected(self):
        correct = [dict(name=n) for n in common.PRIOR]
        cases.names(correct, common.PRIOR, 'prior')
        for changed in (correct[:-1], correct + [correct[0]],
                        [*correct[:-1], dict(name='rebound-job-limit')], list(reversed(correct))):
            with self.assertRaisesRegex(ValueError, 'case set'):
                cases.names(changed, common.PRIOR, 'prior')

    def test_pending_source_gate_refuses_acceptance(self):
        with patch.object(common, 'load', return_value={'status': 'pending-reviewed-v3-source-freeze', 'sha256': {}}):
            with self.assertRaisesRegex(ValueError, 'pending'):
                common.v3_pins(Path('/synthetic'))

    def test_partial_frozen_source_map_rejected(self):
        with patch.object(common, 'load', return_value={'status': 'frozen-reviewed-v3-sources', 'sha256': {}}):
            with self.assertRaisesRegex(ValueError, 'all reviewed'):
                common.v3_pins(Path('/synthetic'))

    def test_original_pin_cannot_be_overridden(self):
        source = next(iter(common.PINS))
        with patch.object(common, 'load', return_value={'status': 'frozen-reviewed-v3-sources', 'sha256': {source:'00'*32}}):
            with self.assertRaisesRegex(ValueError, 'original reviewed pins'):
                common.v3_pins(Path('/synthetic'))

    def test_original_positive_and_corpus_assertions_retained(self):
        original = common.CHECKER.read_text()
        fresh = (common.ROOT/'positive.py').read_text()
        start = original.index('def generation')
        producer = original[original.index('    whole, attacks =', start):original.index('    suite_path =', start)]
        producer = producer.replace('whole, attacks = base / "whole-proof", base / f"whole-proof-attacks-v2-c{number}"',
                                    'whole = base / "whole-proof"')
        corpus = original[original.index('    corpus_dir =', start):original.index('    return dict(generation=number', start)]
        self.assertIn(producer, fresh)
        self.assertIn(corpus, fresh)


class DiagnosticTests(unittest.TestCase):
    """Small metadata fixtures exercise path/source/registration binding only."""
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.directory = self.base/'whole-proof-attacks-v3-c2/attempts/whole-c2-verify-continuation'
        self.directory.mkdir(parents=True)
        self.shared = self.base/'whole-proof-attacks-completion-v3'
        (self.shared/'native-processes').mkdir(parents=True)
        self.argv = ['/synthetic/joy', 'verify-artifact', '/synthetic/compiler']
        self.metadata = dict(generation=2, expected_error='semantic record: Key')
        self.inputs = {'/synthetic/proof': dict(bytes=1, sha256='01'*32)}
        self.pins = {'whole-proof-attacks-v3-c2/guard.py': '02'*32}
        self.write(self.shared/'plan.json', {'proofs':{'2':{'bytes':10569174820}}})
        self.write(self.shared/'admission.json', {'pid':400, 'children':{'2':500}})
        (self.shared/'resources.py').write_text('# synthetic metadata fixture; never imported\n')
        self.registered = dict(pid=600, pgid=600, parent=500, coordinator=400, argv=self.argv)
        self.registration = self.shared/'native-processes'/(self.directory.name+'.json')
        self.write(self.registration, self.registered)
        self.sample = dict(time_ns=1001, elapsed=0.01, shared_stop=False,
                           rss_bytes=1024, processes=[dict(pid=600,rss_bytes=1024)])
        for name, body in [('stdout',''),('stderr','semantic record: Key\n'),('resources.jsonl',json.dumps(self.sample)+'\n')]:
            (self.directory/name).write_text(body)
        self.row = dict(schema='trident/whole-proof-attack-command/v2', status='passed', exit_code=1,
                        expected_exit=1, argv=self.argv, metadata=self.metadata, cwd=str(self.directory),
                        environment={'PATH':''}, inputs_before=self.inputs, inputs_after=self.inputs,
                        driver={'sha256':'02'*32}, caps=dict(wall=7500,cpu=7500,rss=6*1024**3,file=32*1024**2),
                        sampled_scope_disk_cap=26*1024**3, free_floor=8*1024**3, sample_interval_seconds=1,
                        started_ns=1000, ended_ns=1002, sampled_peak_rss_bytes=1024, latest_sample=self.sample,
                        per_stream_log_bytes=1024**2, pid=600,
                        files={name:common.identity(self.directory/name) for name in ('stdout','stderr','resources.jsonl')})
        for key, name in [('shared_profile','plan.json'),('shared_driver','resources.py'),('admission','admission.json')]:
            self.row[key] = common.identity(self.shared/name)

    @staticmethod
    def write(path, value):
        path.write_text(json.dumps(value))

    def call(self):
        self.write(self.directory/'receipt.json',self.row)
        return cases.diagnostic(self.directory,self.argv,1,self.metadata,self.inputs,'verify',3,self.pins)

    def test_correct_synthetic_metadata_admission(self):
        self.assertEqual(self.call(),self.row)

    def test_relocated_or_changed_command_rejected(self):
        for key,value in [('cwd','/different'),('argv',['/different/joy']),('environment',{'PATH':'','GH_TOKEN':'synthetic'})]:
            old=self.row[key];self.row[key]=value
            with self.assertRaisesRegex(ValueError,'invocation'):
                self.call()
            self.row[key]=old

    def test_wrong_guard_or_shared_profile_rejected(self):
        self.row['driver']['sha256']='03'*32
        with self.assertRaisesRegex(ValueError,'reviewed fresh guard'):
            self.call()
        self.row['driver']['sha256']='02'*32
        self.row['shared_profile']['sha256']='03'*32
        with self.assertRaisesRegex(ValueError,'identity differs'):
            self.call()

    def test_wrong_generation_registration_rejected(self):
        self.registered['parent']=501
        self.write(self.registration,self.registered)
        with self.assertRaisesRegex(ValueError,'registered before'):
            self.call()

    def test_changed_cap_or_recorded_resource_stop_rejected(self):
        self.row['caps']['wall']+=1
        with self.assertRaisesRegex(ValueError,'resource contract'):
            self.call()
        self.row['caps']['wall']-=1
        self.row['resource_stop']='rss'
        with self.assertRaisesRegex(ValueError,'did not pass'):
            self.call()

    def test_sample_over_cap_or_shared_stop_rejected(self):
        for key,value in [('elapsed',7501),('shared_stop',True),('rss_bytes',6*1024**3+1)]:
            changed = dict(self.sample);changed[key]=value
            path=self.directory/'resources.jsonl'
            path.write_text(json.dumps(changed)+'\n')
            self.row['files']['resources.jsonl']=common.identity(path)
            with self.assertRaisesRegex(ValueError,'sample obeys original caps'):
                self.call()

    def test_unmatched_sample_peak_rejected(self):
        self.row['sampled_peak_rss_bytes']+=1
        with self.assertRaisesRegex(ValueError,'peak binding'):
            self.call()


class ResourceTests(unittest.TestCase):
    def setUp(self):
        self.base = Path('/synthetic')
        self.plan = dict(proofs={'1': dict(bytes=11977015727), '2': dict(bytes=10569174820)})
        self.row = dict(reason=None, owned_bytes=25*schedule.GIB, free_bytes=8*schedule.GIB,
                        rss_bytes=12*schedule.GIB, processes=[dict(pid=99, rss_kib=12*schedule.GIB//1024)],
                        coordinator_metadata_bytes=256*schedule.MIB,
                        generations={str(g): dict(mutants={}, canonical_output_bytes=112*schedule.MIB,
                                                  metadata_bytes=256*schedule.MIB) for g in (1, 2)})

    def test_reservation_counts_both_mutants_all_buckets_and_old_bytes(self):
        existing = 920494713 + 878826753 + 1234567
        result = schedule.reservation(self.plan, {'files': {'v1partial':920494713,
                                      'v2partial':878826753, 'metadata':1234567}})
        mutants = 11977015727 + 10569174820 + 2*32*schedule.MIB
        total = existing + mutants + 2*112*schedule.MIB + 3*256*schedule.MIB
        self.assertEqual(result, (existing, total, max(total-existing+8*schedule.GIB, mutants+10*schedule.GIB)))

    def test_exact_resource_bounds_and_final_sample_contract(self):
        schedule.sample(self.row, self.plan, self.base)
        for key in ('owned_bytes', 'rss_bytes', 'free_bytes'):
            changed = copy.deepcopy(self.row)
            changed[key] = (26*schedule.GIB+1 if key == 'owned_bytes' else
                            12*schedule.GIB+1 if key == 'rss_bytes' else 8*schedule.GIB-1)
            with self.assertRaisesRegex(ValueError, 'resource verdict'):
                schedule.sample(changed, self.plan, self.base)

    def test_missing_generation_and_fabricated_rss_sum_rejected(self):
        changed = copy.deepcopy(self.row)
        del changed['generations']['2']
        with self.assertRaisesRegex(ValueError, 'both generation'):
            schedule.sample(changed, self.plan, self.base)
        changed = copy.deepcopy(self.row)
        changed['processes'][0]['rss_kib'] -= 1
        with self.assertRaisesRegex(ValueError, 'RSS sum'):
            schedule.sample(changed, self.plan, self.base)

    def test_two_mutants_or_foreign_mutant_rejected(self):
        root = self.base/'whole-proof-attacks-v3-c2/whole-c2'
        for mutants in ({str(root/'certificate-a.joysc'):1, str(root/'certificate-b.joysc'):1},
                        {'/foreign/certificate-a.joysc':1},
                        {str(root/'certificate-a.joysc'):10569174820+32*schedule.MIB+1}):
            changed = copy.deepcopy(self.row)
            changed['generations']['2']['mutants'] = mutants
            with self.assertRaises(ValueError):
                schedule.sample(changed, self.plan, self.base)


if __name__ == '__main__':
    unittest.main()
