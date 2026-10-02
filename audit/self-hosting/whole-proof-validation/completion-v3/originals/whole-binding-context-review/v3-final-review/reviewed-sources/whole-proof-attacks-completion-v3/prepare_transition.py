"""Freeze the completed reviewed cleanup transition and missing-case sources."""
import hashlib,json,time
from pathlib import Path
ROOT=Path(__file__).resolve().parent
BASE=ROOT.parent

def identity(path):
 with path.open('rb') as stream:
  return dict(bytes=path.stat().st_size,sha256=hashlib.file_digest(stream,'sha256').hexdigest())

def load(path):return json.loads(path.read_text())
def save(path,value):
 with path.open('x') as stream:json.dump(value,stream,indent=2);stream.write('\n')

def main():
 action=ROOT/'reclamation-action-v2/receipt.json';r=load(action)
 assert r['status']=='passed' and r['removed'] is True and r['original_failed_evidence_unchanged'] is True
 old=load(ROOT/'reclamation-action/receipt.json')
 assert old['status']=='failed' and old['removed'] is False
 plan=load(ROOT/'reclamation-plan.json');packet=BASE/'whole-reclamation-review';classified=load(packet/'classification.json')
 names=set(classified['retained'])
 paths={BASE/name for name in names}|{packet/'retained'/name for name in names}
 paths.update(p for scope in ['reclamation-action','reclamation-action-v2','reclamation-failure-observation'] for p in (ROOT/scope).iterdir() if p.is_file())
 paths.update(ROOT/name for name in ['reclaim_completed.py','reclaim_completed_v2.py','reclamation-plan.json','reclamation-review.json','reclamation-review-v2.json'])
 paths.update(packet/name for name in ['classification.json','comparison.json','compare.py','retain.py'])
 prior=BASE/'whole-proof-attacks-parallel-v2'
 paths.update(prior/name for name in ['stop.json','shutdown.json','run-1/receipt.json','run-1/resources.jsonl','coordinator.stdout','coordinator.stderr','transition.json','admission.json'])
 paths.update(p for p in (prior/'native-processes').iterdir() if p.is_file())
 paths.add(BASE/'whole-proof-final-review-v2/orchestration/receipt.json')
 partials={str(BASE/name):value for name,value in plan['retained_partials'].items()}
 assert all(identity(Path(p))==value for p,value in partials.items())
 assert not (BASE/plan['target']).exists()
 transition=dict(schema='trident/classified-prior-attempts-transition/v1',status='prior-attempts-classified-and-quiescent',scope='Original attempts stay failed. Nine successful cases and two controls per generation may be explicitly replayed; fourteen missing cases must execute afresh under unchanged bounds.',created_ns=time.time_ns(),reclamation_receipt=dict(path=str(action),**identity(action)),failed_reclamation_receipt=dict(path=str(ROOT/'reclamation-action/receipt.json'),**identity(ROOT/'reclamation-action/receipt.json')),retained_partial_mutants=partials,files={str(p):identity(p) for p in sorted(paths)})
 save(ROOT/'transition.json',transition)
 manifest_paths={ROOT/name for name in ['run.py','resources.py','plan.json','transition.json','copied-inputs.json','prior_cases.py','binding_context.py','binding-fixture-acceptance.json','prepare_transition.py','reclaim_completed.py','reclaim_completed_v2.py','reclamation-plan.json','reclamation-review.json','reclamation-review-v2.json']}
 manifest_paths.update(p for p in (ROOT/'tests').glob('*.py'))
 manifest_paths.update(BASE/f'whole-proof-attacks-v3-c{g}'/name for g in (1,2) for name in ['guard.py','whole_suite.py'])
 manifest_paths.add(BASE/'whole-proof-final-review-v2/check.py')
 manifest_paths.update(BASE/'whole-binding-context-review'/name for name in ['review.json','actual/receipt.json','sources.json','check_result.py','run.py','guard.py'])
 manifest={str(p):identity(p) for p in sorted(manifest_paths)}
 save(ROOT/'sources.json',manifest)
 print(json.dumps(dict(status='prepared-for-independent-review',sources=identity(ROOT/'sources.json'),transition=identity(ROOT/'transition.json'),source_files=len(manifest),transition_files=len(paths))))

if __name__=='__main__':main()
