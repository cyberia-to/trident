"""Tiny constructed provenance trees; no proof/native execution evidence."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock
import prior_cases_v4 as M
import prior_replay_v4 as R
from prior_fixture_v4 import PriorFixture, write, identity

BASE = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location('test_pinned_prior', BASE/'whole-proof-attacks-completion-v3/prior_cases.py')
OLD = importlib.util.module_from_spec(spec)
spec.loader.exec_module(OLD)


class Fixture:
    def __init__(self, base):
        self.old = PriorFixture(base, OLD)
        self.base, self.root = base, base/'whole-proof-attacks-v3-c1'
        self.shared = base/'whole-proof-attacks-completion-v3'
        for p in self.old.attacks.glob('attempts/*/receipt.json'):
            self.samples(p)
        self.prior = self.old.review()
        self.f = copy.copy(self.old)
        f = self.f
        f.attacks, f.shared, f.work = self.root, self.shared, self.root/'whole-c1'
        f.frozen = self.root/'inputs/frozen'
        f.helper = f.put(self.root/'target-helper/release/whole-proof-mutator', b'not executable')
        f.guard = f.put(self.root/'guard.py', b'v3 fixture guard')
        f.index, f.noun = f.work/'index.json', f.work/'result.dag'
        f.put(f.index, self.old.index.read_bytes())
        f.put(f.noun, self.old.noun.read_bytes())
        for p in self.old.frozen.iterdir():
            f.put(f.frozen/p.name, p.read_bytes())
        write(self.shared/'plan.json', dict(proofs={'1': identity(f.proof)}))
        f.put(self.shared/'resources.py', b'v3 fixture resource accounting')
        write(self.shared/'admission.json', dict(pid=900, children={'1':901}))
        write(self.shared/'run-1/receipt.json', dict(status='failed', started_ns=1, ended_ns=10000))
        self.prep = copy.deepcopy(self.old.preparation)
        variant = self.prep['generations']['1']['variants']['job-limit']
        job = f.put(self.root/'inputs/c1/job-limit/job.dag', b'NOXDAG01'+bytes.fromhex('4'*64))
        variant.update(job=identity(job), admission=dict(compiler_particle='1'*64,job_particle='4'*64,
                                                       limits=dict(reductions=19999999999,evaluator_frames=65536)))
        write(self.root/'preparation.json', self.prep)
        self.suite = dict(schema='trident/whole-self-build-certificate-completion/v3', status='failed', generation=1,
                          profile=f.profile, original_proof=identity(f.proof), prior_cases=self.prior,
                          immutable_inputs_before={str(f.helper):identity(f.helper)},
                          immutable_inputs_after={str(f.helper):identity(f.helper)},
                          error='unrelated later construct-cost timeout', rejections=[])
        for name in M.ADDITIONAL:
            self.case(name)
        self.path = f.work/'receipt.json'
        write(self.path, self.suite)
        self.pins = dict(self.old.pins, **{'whole-proof-attacks-v3-c1/guard.py':identity(f.guard)['sha256']})

    @staticmethod
    def samples(path):
        row = json.loads(path.read_text())
        observed = dict(time_ns=300, elapsed=1, rss_bytes=7, shared_stop=False,
                        processes=[dict(pid=row['pid'],rss_bytes=7)])
        (path.parent/'resources.jsonl').write_text(json.dumps(observed)+'\n')
        row.update(started_ns=100,ended_ns=500,elapsed_seconds=2,sampled_peak_rss_bytes=7,latest_sample=observed)
        row['files']['resources.jsonl'] = identity(path.parent/'resources.jsonl')
        write(path,row)

    def case(self,name):
        f=self.f
        compiler,job=f.frozen/'c1.dag',f.frozen/'c1-job.dag'
        mode,context=name,[]
        if name=='rebound-job-limit':
            job=self.root/'inputs/c1/job-limit/job.dag'
            mode='rebind'
            context=['1'*64,'3'*64,'4'*64,'1','19999999999','65536']
        certificate=f.work/f'certificate-{name}.joysc'
        cert=dict(bytes=37,sha256='a'*64)
        inputs={str(p):identity(p) for p in (f.helper,f.proof,f.index,f.noun)}
        construction=f.diagnostic('construct-'+name,[f.helper,'mutate',f.proof,f.index,f.noun,certificate,mode,*context],
                                  0,dict(generation=1,mode=mode),inputs,'helper',
                                  dict(mode=mode,wire_bytes=37,source_records=f.result['records'],
                                       source_sha256=identity(f.proof)['sha256']))
        output=f.put(f.work/(name+'.dag'),b'protected')
        inputs={str(p):identity(p) for p in (f.joy,compiler,job,output)}
        inputs[str(certificate)]=cert
        verification=f.diagnostic('verify-'+name,[f.joy,'verify-artifact',compiler,'--input',job,'--proof',certificate,
                                                 '--output',output,'--emit','program',*f.flags,'--force'],
                                  1,dict(generation=1,expected_error=M.ERRORS[name]),inputs,'verify',None,M.ERRORS[name].encode())
        for rel in (construction,verification):
            path=self.root/rel
            self.samples(path)
            reg=self.shared/'native-processes'/(path.parent.name+'.json')
            data=json.loads(reg.read_text());data['registered_ns']=200;write(reg,data)
            reg.with_suffix('.retired').write_text('400\n')
        self.suite['rejections'].append(dict(name=name,certificate=cert,error=M.ERRORS[name],protected_output=identity(output),
            verification_receipt=verification,recipe=dict(mode=mode,context=context,certificate=cert,construction_receipt=construction)))

    def review(self,expected=None):
        def checked_pins(base):
            for name,value in self.pins.items():
                M.require(identity(base/name)['sha256']==value,'reviewed source/runtime pin')
            return self.pins
        with mock.patch.object(M,'v3_pins',side_effect=checked_pins), \
             mock.patch.object(M,'original_admission',side_effect=lambda b,g,p,v,e:self.old.review(e)), \
             mock.patch.object(M,'review_quiescence',return_value={'scope':'synthetic isolated quiescence tested separately'}), \
             mock.patch.dict(M.C.PINS,self.pins,clear=True):
            return M.review(self.base,1,self.f.proof,self.f.verifier,self.f.result if expected is None else expected)


class PriorCasesV4(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.f=Fixture(Path(self.temp.name).resolve())
    def tearDown(self):
        self.temp.cleanup()
    def edit(self,path,change):
        value=json.loads(path.read_text());change(value);write(path,value)
    def reject(self,path,change,message):
        self.edit(path,change)
        with self.assertRaisesRegex(ValueError,message):self.f.review()
    def command(self,name):
        return self.f.root/'attempts'/('whole-c1-'+name)/'receipt.json'
    def test_twelve_disjoint_cases_and_original_controls(self):
        row=self.f.review()
        self.assertEqual([r['name'] for r in row['rejections']],list(M.PRIOR_NAMES))
        self.assertEqual(len(row['controls']),2)
        self.assertEqual(row['index'],self.f.prior['index'])
        self.assertEqual(row['additional_v3']['status'],'failed')
    def test_wrong_caller_result(self):
        with self.assertRaisesRegex(ValueError,'caller result equals'):
            self.f.review(dict(self.f.f.result,charged_reductions=8))
    def test_no_reclassification_of_failed_suite(self):
        self.reject(self.f.path,lambda x:x.update(status='passed-completion'),'original failed v3')
    def test_omitted_additional_case(self):
        self.reject(self.f.path,lambda x:x['rejections'].pop(),'additional v3 cases')
    def test_duplicate_additional_case(self):
        self.reject(self.f.path,lambda x:x['rejections'].__setitem__(2,x['rejections'][1]),'additional v3 cases')
    def test_cost_cannot_be_inherited(self):
        self.reject(self.f.path,lambda x:x['rejections'][2].update(name='cost'),'additional v3 cases')
    def test_old_wrong_context_rejected(self):
        self.reject(self.f.path,lambda x:x['rejections'][0]['recipe']['context'].__setitem__(4,'20000000000'),
                    'exact construction request and context')
    def test_changed_guard_source(self):
        self.f.f.guard.write_bytes(b'changed source')
        with self.assertRaisesRegex(ValueError,'reviewed source/runtime pin'):
            self.f.review()
    def test_wrong_native_input(self):
        self.reject(self.command('verify-continuation'),lambda x:x['argv'].__setitem__(2,'wrong-compiler'),
                    'exact fresh diagnostic invocation')
    def test_wrong_native_outcome(self):
        self.reject(self.command('verify-generation'),lambda x:x.update(exit_code=-15),'bounded command did not pass')
    def test_hidden_cleanup_failure(self):
        self.reject(self.command('verify-generation'),lambda x:x.update(cleanup_error='still live'),
                    'successful command has no hidden failure')
    def test_missing_retirement(self):
        (self.f.shared/'native-processes/whole-c1-verify-generation.retired').unlink()
        with self.assertRaises(FileNotFoundError):self.f.review()
    def test_retirement_outside_command(self):
        (self.f.shared/'native-processes/whole-c1-verify-generation.retired').write_text('501\n')
        with self.assertRaisesRegex(ValueError,'retirement belongs'):self.f.review()
    def test_registration_wrong_parent(self):
        self.reject(self.f.shared/'native-processes/whole-c1-verify-generation.json',lambda x:x.update(parent=999),
                    'native process registered')
    def test_resource_sample_wrong_peak(self):
        self.reject(self.command('verify-generation'),lambda x:x.update(sampled_peak_rss_bytes=0),
                    'complete command samples')
    def test_changed_protected_output(self):
        (self.f.f.work/'generation.dag').write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError,'exact fresh diagnostic input identities|retained identity'):
            self.f.review()
    def test_unexpected_fresh_control(self):
        self.reject(self.f.path,lambda x:x.update(controls=[]),'fabricated fresh control')
    def test_inconsistent_prior_eligibility(self):
        self.reject(self.f.path,lambda x:x['prior_cases']['original_proof'].update(sha256='0'*64),'original eligibility')


if __name__=='__main__':unittest.main(verbosity=2)
