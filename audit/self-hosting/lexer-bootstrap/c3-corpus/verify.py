#!/usr/bin/env python3
"""Verify retained S1 C3 corpus bindings without running any language stage."""
from pathlib import Path
import gzip, hashlib, json, subprocess, tarfile
A = Path(__file__).resolve().parent
REPO = A.parents[3]
REV = '77213171d39b88c5f41221912251cc4813ac2b11'
def sha(b): return hashlib.sha256(b).hexdigest()
def read(name):
    b = (A / name).read_bytes()
    return json.loads(gzip.decompress(b) if name.endswith('.gz') else b)
def limits(x, path=''):
    rows = {}
    if isinstance(x, dict):
        for k, v in x.items():
            if k == 'limits': rows[path + '/limits'] = v
            else: rows.update(limits(v, path + '/' + k))
    elif isinstance(x, list):
        for n, v in enumerate(x): rows.update(limits(v, path + '/' + str(n)))
    return rows
FLAGS = ['--budget','--arena-nodes','--frames','--time-ms','--validation-visits','--resident-nodes','--collection-work']
def resources(receipt):
    rows = []
    for command in receipt['commands']:
        argv = command['command']
        if len(argv)>1 and argv[1] in ['run-artifact','pack-job']:
            rows.append((argv[1], {f:argv[argv.index(f)+1] for f in FLAGS if f in argv}))
    return rows
inputs, summary, validation = read('inputs.json'), read('summary.json'), read('validation.json')
producer_bytes = (A.parent / 'fixed-point/c2-to-c3.json').read_bytes()
producer = json.loads(producer_bytes)
assert summary['passed'] and summary['inputs_unchanged']
assert sha(producer_bytes) == inputs['closure_receipt_sha256']
assert producer['status'] == 'compiler-returned'
assert producer['result_sha256'] == inputs['compiler_sha256'] == sha(gzip.decompress((A.parent/'fixed-point/c3.dag.gz').read_bytes()))
assert producer['execution']['artifact'] == inputs['compiler']
assert inputs['compiler'].endswith('/lexer-v9-c2-to-c3-files-jb8je2q4/result.dag')
assert inputs['closure_receipt'].endswith('/lexer-v9-c2-to-c3.json')
assert producer['execution']['published_particle'] == inputs['compiler_particle']
assert producer['binary_sha256'] == producer['binary_sha256_end'] == inputs['binary_sha256']
assert len(inputs['source_sha256']) == 94
assert {r['path']:r['sha256'] for r in producer['sources'].values()} == inputs['source_sha256']
for path, expected in inputs['source_sha256'].items():
    assert sha(subprocess.check_output(['git','show',REV+':'+path],cwd=REPO)) == expected
for name, row in validation['evidence'].items():
    b = (A / name).read_bytes()
    assert sha(b) == row['sha256'] and len(b) == row['bytes']
    if 'original_sha256' in row:
        raw = gzip.decompress(b) if name.endswith('.gz') else b
        assert sha(raw) == row['original_sha256'] and len(raw) == row['original_bytes']
rows = {}; totals = {'observations':0,'commands':0,'raw_reference_builds':0,'compiler_seed_builds':0,'equal_fixed_job_limit_objects':0,'adaptive_boundary_job_limit_objects':0,'equal_host_resource_commands':0}
for name, record in validation['results'].items():
    current = read(name+'.json.gz'); history_bytes = (REPO/record['historical_receipt']).read_bytes()
    assert sha(history_bytes) == record['historical_receipt_sha256']
    history = json.loads(history_bytes)
    assert current['status'] == 'passed' and current['compiler_mode'] == 'provided'
    assert current['compiler_path'] == inputs['compiler']
    assert current['compiler_sha256_start'] == current['compiler_sha256_end'] == inputs['compiler_sha256']
    assert current.get('binary_sha256',current.get('binary_sha256_start')) == inputs['binary_sha256']
    assert current.get('binary_sha256_end',inputs['binary_sha256']) == inputs['binary_sha256']
    assert len(current['observations']) == len(history['observations']) == record['observations']
    checked = adaptive = 0
    for old, new in zip(history['observations'],current['observations']):
        for key in ['case','limit','source_hex','sources','expected','boundary']:
            assert old.get(key) == new.get(key), (name,new.get('case'),key)
        old_limits, new_limits = limits(old), limits(new)
        if old_limits != new_limits:
            assert name == 'main' and new.get('boundary') == 'exact'
            limit = new.get('limit')
            assert limit in ['reductions','evaluator_frames','arena_nodes']
            assert old_limits.keys() == new_limits.keys() and len(new_limits) == 1
            for key in old_limits:
                before, after = dict(old_limits[key]), dict(new_limits[key])
                assert before.pop(limit) == old['requested']
                assert after.pop(limit) == new['requested']
                assert before == after
            adaptive += len(new_limits)
        else:
            checked += len(old_limits)
        if new.get('limit') not in ['reductions','evaluator_frames','arena_nodes']:
            assert old.get('requested') == new.get('requested')
    if name == 'main':
        baseline = current['observations'][0]['compiler_execution']['execution']
        calibration = [json.loads(c['stdout'])['execution'] for c in current['commands']
                       if c['command'][1] == 'run-artifact' and any('arena-calibration/job.dag' in a for a in c['command'])]
        assert len(calibration) == 1
        exact = {'reductions':baseline['charged_reductions'],'evaluator_frames':baseline['peak_frames'],'arena_nodes':calibration[0]['allocated_nodes']}
        for limit, value in exact.items():
            cases = [o for o in current['observations'] if o.get('limit') == limit]
            assert len(cases) == 2
            for case in cases:
                delta = 0 if case['boundary'] == 'exact' else -1
                assert case['boundary'] in ['exact','one-below']
                assert case['requested'] == value + delta and case['previous_program_preserved']
                assert (case['compiler_execution'] is not None) == (delta == 0)
    assert resources(current) == resources(history), name
    builds = [c for c in current['commands'] if c['command'][1] == 'build']
    for command in builds:
        argv = command['command']
        assert command['reference_only'] is True
        assert argv[argv.index('--artifact-profile')+1] == 'raw'
        assert argv[argv.index('-o')+1] != inputs['compiler']
    row = {'observations':len(current['observations']),'commands':len(current['commands']),'raw_reference_builds':len(builds),'compiler_seed_builds':0,'equal_fixed_job_limit_objects':checked,'adaptive_boundary_job_limit_objects':adaptive,'equal_host_resource_commands':len(resources(current))}
    rows[name] = row
    for k,v in row.items(): totals[k] += v
generated = read('generated-profile.json.gz')
with tarfile.open(A/'generated-artifacts.tar.gz') as archive:
    assert len(archive.getmembers()) == len(generated['files'])
    assert set(archive.getnames()) == {r['path'] for r in generated['files']}
    for row in generated['files']:
        b = archive.extractfile(row['path']).read()
        assert sha(b) == row['sha256'] and len(b) == row['bytes']
assert totals['observations'] == 547 and totals['commands'] == 1816
assert totals['raw_reference_builds'] == 119 and totals['compiler_seed_builds'] == 0
result = {'schema':'trident/c3-corpus-binding/v1','status':'passed','command':['python3','audit/self-hosting/lexer-bootstrap/c3-corpus/verify.py'],'source_revision':REV,'captured_revision':inputs['revision'],'source_count':94,'source_git_blobs_equal':True,'producer_receipt_sha256':sha(producer_bytes),'actual_c3_sha256':inputs['compiler_sha256'],'actual_c3_particle':inputs['compiler_particle'],'runtime_sha256':inputs['binary_sha256'],'validation_sha256':sha((A/'validation.json').read_bytes()),'summary_sha256':sha((A/'summary.json').read_bytes()),'results':rows,'totals':totals,'generated_archive_members':len(generated['files']),'scope':'Actual local S1 C3 six-corpus acceptance only; fixed-point, clean bootstrap, platforms and proofs are separate.'}
b = (json.dumps(result,indent=2)+'\n').encode(); out=A/'binding.json'
if out.exists(): assert out.read_bytes()==b
else: out.write_bytes(b)
print(json.dumps({'status':'passed','totals':totals,'binding_sha256':sha(b)}))
