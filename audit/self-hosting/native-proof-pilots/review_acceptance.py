import hashlib,json,tarfile,datetime
from pathlib import Path
root=Path('/Users/master/cyber/.worktrees/selfhost-0.4-finalization-20261002/proof-acceptance/trident')
p=root/'audit/self-hosting/native-proof-pilots'
out=Path(__file__).resolve().parent

def identity(b): return {'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()}
summary=json.loads((p/'pilot-summary.json').read_text())
checks={}
with tarfile.open(p/'pilot-receipts.tar.gz') as archive:
    def raw(n): return archive.extractfile('actual-c2-pilot-evidence/'+n).read()
    def read(n): return json.loads(raw(n))
    files=read('package-files.json')
    assert len(files)==637
    for name,value in files.items(): assert identity(raw(name))==value,name
    assert raw('summary.json')==(p/'pilot-summary.json').read_bytes()
    checks['small_package_file_hashes_verified']=len(files)
    checks['summary_matches_original_package']=True
    for pilot in summary['pilots']:
        reports=[]
        for field,operation in [('prover_receipt','prove-artifact'),('verifier_receipt','verify-artifact')]:
            receipt=read(pilot[field]);folder=Path(pilot[field]).parent.as_posix()
            assert receipt['status']=='passed' and receipt['exit_code']==0
            assert receipt['argv'][1]==operation
            assert receipt['binary']==summary['binary']
            assert receipt['binary_end']=={k:summary['binary'][k] for k in ('bytes','sha256')}
            assert receipt['argument_files_start']==receipt['argument_files_end']
            assert receipt['pilot_sources_start']==receipt['pilot_sources_end']
            report=read(folder+'/stdout')['verification']
            if field=='verifier_receipt': assert 'prover_observations' not in report
            report.pop('elapsed_micros',None);report.pop('prover_observations',None)
            assert report==pilot['verified']
            reports.append(report)
        assert reports[0]==reports[1]
        assert pilot['proof']['bytes']==reports[0]['transport']['wire_bytes']
    assert len(summary['pilots'])==5
    checks['positive_pilots_and_fresh_verified_fields']=5
    accepted=rejected=0
    for name in ['adversarial-results.json','adversarial-valid-output-results.json']:
        rows=read(name)['results']
        for row in rows:
            receipt=read(row['receipt']);folder=Path(row['receipt']).parent.as_posix()
            assert receipt['argv'][1]=='verify-artifact'
            assert receipt['binary']==summary['binary']
            assert receipt['binary_end']=={k:summary['binary'][k] for k in ('bytes','sha256')}
            assert receipt['exit_code']==row['actual_exit']==row['expected_exit']
            if row['expected_exit']==0:
                accepted+=1
            else:
                rejected+=1
                assert row['destination_preserved'] and raw(folder+'/stdout')==b''
                assert raw(folder+'/stderr').decode()==row['stderr']
    assert (accepted,rejected)==(3,24)
    checks['accepted_controls']=accepted;checks['rejected_mutations']=rejected
    v=next(x for x in summary['pilots'] if x['name']=='scale-valid65536')
    assert v['prover_host_observations']['resets']==3
    assert v['proof']['bytes']==417642285 and v['verified']['charged_reductions']==488109499
    assert v['verified']['expanded_steps']==437670002 and v['verified']['logical_peak_frames']==4324
    checks['long_pilot_metrics_and_two_collections_match']=True
historical=json.loads((root/'audit/self-hosting-2026-09-23/soft3-runtime-probes.json').read_text())
dyn=p/'historical-dynamic/receipts/execution';receipt=json.loads((dyn/'receipt.json').read_text())
assert receipt['source']==historical['dynamic_apply_source'].strip()
assert receipt['public_inputs']==historical['public_inputs']==[1]
assert receipt['historical_probe']==identity((root/'audit/self-hosting-2026-09-23/soft3-runtime-probes.json').read_bytes())
for name,value in receipt['files'].items():assert identity((dyn/name).read_bytes())==value
assert (dyn/'expected.dag').read_bytes()==(dyn/'executed.dag').read_bytes()==(dyn/'verified.dag').read_bytes()
assert receipt['verification']['charged_reductions']==6 and receipt['verification']['transport']['wire_bytes']==406
assert all(c['exit_code']==0 for c in receipt['commands'])
checks['exact_historical_dynamic_probe_receipts']=True
paths=['.claude/plans/native-compiler-arena.md','audit/self-hosting-progress.md','reference/self-hosting.md','audit/self-hosting-2026-09-23/soft3-runtime-probes.json']
paths+=['audit/self-hosting/native-proof-pilots/'+n for n in ['README.md','pilot-report.md','pilot-summary.json','root-review.json','independent-review.json','historical-dynamic/main.rs','historical-dynamic/receipts/execution/receipt.json','pilot-receipts.tar.gz']]
record={'schema':'trident/sh7-doc-acceptance-independent-review/v1','status':'passed','time_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'review_scope':'Read-only changed roadmap/ledger/plan and normative SH7/SH8 scope against retained pilot/historical dynamic receipts and previously reviewed production relation. Independently checked all 637 small-package indexed files and selected command/report bindings. No new proof execution, no full 602 MB archive byte replay, no SH8 proof consumed, no root-owned source writes.','source_files':{str(root/n):identity((root/n).read_bytes()) for n in paths},'checker':identity(Path(__file__).read_bytes()),'checks':checks,'findings':[],'acceptance_reasoning':['Existing SH7 criteria permit a declared full-witness public relation; this delivery changes no acceptance criterion and cites actual SH3/SH4 workloads.','Five pilots use the complete accepted C2; they are not represented as complete self-build proofs. Separate fresh-process verifier reports bind the same compiler/job/result/cost.','Adversarial coverage includes rebuilt contexts and canonical wrong outputs with rebound terminals, so output-binding claims are not based only on malformed codecs.','The historical dynamic formula and public input are exact; production proof/verifier/run receipts report output42, charge6 and 406 proof bytes.','Disclosure, physical-resource non-attestation and non-preservation/succinctness limits agree with Joy structured certificate contract.','SH8 remains explicitly open for both complete frozen self-builds, fresh verification, exact output equality and their adversarial tests.'], 'pending_external_evidence':['Root-owned large archive upload/download receipt remains pending as explicitly stated in the reviewed README. This review does not claim completed remote retention or authorize promotion/tag creation.']}
profile=Path('/Users/master/cyber/.worktrees/selfhost-0.4-finalization-20261002/certificate/joy/specs/structured-certificates.md')
record['source_files'][str(profile)]=identity(profile.read_bytes())
(out/'receipt.json').write_text(json.dumps(record,indent=2)+'\n')
print(json.dumps({'status':record['status'],'checks':checks,'receipt':str(out/'receipt.json')},indent=2))
