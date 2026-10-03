"""Independent terminal metadata review. Does not invoke the checker or native code."""
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys
import time

B = Path('/Users/master/cyber/.worktrees/selfhost-0.4-finalization-20261002')
O = B / 'whole-proof-actual-independent-review'
F = B / 'whole-proof-final-review-v5'
V = B / 'whole-proof-attacks-completion-v5'
W = B / 'whole-proof-final-launch-v5/orchestration'
P = B / 'whole-proof-final-delivery-preparation/staging/actual-1'
LIMIT = 256 * 1024 * 1024
seen = {}
skipped = {}
read_bytes = 0
started = time.time_ns()
NEW = ['cost','valid-output-payload','valid-output-topology','valid-output-payload-rebound',
       'valid-output-topology-rebound','omit-terminal','drop-first','swap-first-two',
       'omit-completion','truncate-last-byte','trailing-byte']
OLD = ['binding-compiler','rebound-compiler','binding-source','rebound-source',
       'binding-dependency','rebound-dependency','binding-cfg','rebound-cfg','binding-job-limit']
MORE = ['rebound-job-limit','continuation','generation']
PROOFS = [dict(bytes=11977015727,sha256='80212be832ce0a5caafa69d9dd346ff20d51ce20fc86892f0c7039492619bbb0'),
          dict(bytes=10569174820,sha256='4db898cbc5133e0b96c758507d32b62aa82c4f39271cedd949b96d641a67de86')]
COMPILER = dict(bytes=9691488,sha256='76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8')


def require(value, label):
    if not value:
        raise ValueError(label)


def digest(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def read(path, expected=None):
    global read_bytes
    path = Path(path)
    require(path.is_absolute() and path.is_relative_to(B), 'owned input namespace')
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, 'rb') as src:
        before = os.fstat(src.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_size <= 64*1024*1024, 'bounded regular metadata')
        require(read_bytes + before.st_size <= LIMIT, 'total review read bound')
        raw = src.read(64*1024*1024+1)
        after = os.fstat(src.fileno())
    require((before.st_dev,before.st_ino,before.st_size,before.st_mtime_ns) ==
            (after.st_dev,after.st_ino,after.st_size,after.st_mtime_ns), 'stable opened metadata')
    read_bytes += len(raw)
    identity = digest(raw)
    if expected is not None:
        require(identity == {k:expected[k] for k in ('bytes','sha256')}, 'exact metadata identity: '+str(path.relative_to(B)))
    if str(path) in seen:
        require(seen[str(path)] == identity, 'unchanged repeated input')
    seen[str(path)] = identity
    return raw


def load(path, expected=None):
    return json.loads(read(path, expected))


def same(path, expected):
    path = Path(path)
    if str(path) in seen:
        require(seen[str(path)] == {k:expected[k] for k in ('bytes','sha256')}, 'consistent duplicate identity')
    else:
        read(path, expected)


def reference(path):
    path = Path(path)
    if str(path) not in seen:
        read(path)
    return dict(path=str(path.relative_to(B)), **seen[str(path)])


def source_map(root, manifest, expected_sha):
    raw = read(root/manifest)
    require(digest(raw)['sha256'] == expected_sha, 'reviewed source manifest')
    mapping = json.loads(raw)
    for name, identity in mapping.items():
        path = Path(name) if Path(name).is_absolute() else root/name
        if path.name in ('joy','whole-proof-mutator') or path.suffix == '.dag':
            skipped[str(path.relative_to(B))] = identity
        else:
            same(path, identity)
    return mapping


def streams(directory, receipt, resources=False):
    for name in ('stdout','stderr'):
        same(directory/name, receipt['files'][name])
    if resources and 'resources.jsonl' in receipt['files']:
        same(directory/'resources.jsonl', receipt['files']['resources.jsonl'])


def check_command(path, exit_code, generation=None):
    d = load(path)
    require(d['status']=='passed' and d['exit_code']==exit_code, 'actual command outcome')
    require(d['inputs_before']==d['inputs_after'], 'actual command input equality')
    require(d['started_ns'] <= d['ended_ns'], 'ordered command time')
    streams(path.parent,d)
    if generation is not None:
        require(d['expected_exit']==exit_code and d['metadata']['generation']==generation, 'explicit diagnostic expectation')
        require(d['final_group']=={'status':'empty','members':[]}, 'native command process closure')
    return d


def main():
    fmap=source_map(F,'sources.json','9c56357c798c6e3e7727c2f55991e66c69b243b88340f7a334461f56fbdc4584')
    vmap=source_map(V,'sources.json','32f57009374ce8b542202ebaf1a3b0b05ab171f7aaa0665a838be0204ba8912e')
    pins=load(F/'pins.json')['files']
    for name, identity in pins.items():
        path=B/name
        if path.name in ('joy','whole-proof-mutator') or path.suffix=='.dag':
            skipped[name]=identity
        else:same(path,identity)
    f=load(W/'final-review.json'); w=load(W/'receipt.json'); v=load(V/'run-1/receipt.json')
    require(f['status']=='passed-composite-replay' and w['status']=='passed' and v['status']=='passed', 'three actual terminal passes')
    require(f['checker_sources']==fmap==w['reviewed_sources'] and v['source']==vmap,'actual source closure')
    same(F/'independent-review.json',f['independent_review']);same(V/'independent-review.json',v['independent_review'])
    require(w['checker_launched'] is True and w['exit_code']==0,'actual checker executed successfully')
    require(w['source_manifest_sha256']==digest(read(F/'sources.json'))['sha256'],'watcher source binding')
    require(w['independent_review_sha256']==f['independent_review']['sha256'],'watcher review binding')
    require(w['checker_command'][1:] == ['-B','-W','error',str(F/'check.py'),'--base',str(B),'--output',str(W/'final-review.json')], 'exact checker arguments')
    require(f['command'][1:]==w['checker_command'][4:], 'checker reported argument binding')
    require(w['checker_environment']==dict(PATH='',LANG='C',PYTHONDONTWRITEBYTECODE='1'),'checker environment')
    require(v['phase']=='complete' and v['active_generation']==2 and v['inputs_unchanged'] is True,'complete actual schedule')
    require(not any(k in v for k in ('error','cleanup_error')),'no hidden coordinator failure')
    same(W/'final-review.json',w['files']['final-review.json']);streams(W,w)
    require(read(W/'stderr')==b'','no checker error output')
    expected_prerequisites={str(B/f'whole-proof/attempts/c{g}-{action}-1/receipt.json'):'passed'
       for g in (1,2) for action in ('selfbuild','fresh-verification')}
    expected_prerequisites.update({str(B/f'whole-proof-attacks-v5-c{g}/whole-c{g}/receipt.json'):'passed-completion' for g in (1,2)})
    expected_prerequisites.update({str(V/'run-1/receipt.json'):'passed',str(B/'whole-proof-corpus-v2/orchestration/receipt.json'):'passed',
       **{str(B/f'whole-proof-corpus-v2/c{g}/receipt.json'):'passed' for g in (2,3)}})
    require(set(w['observed_statuses'])==set(expected_prerequisites),'exact ten terminal prerequisites')
    for path, required in expected_prerequisites.items():
        row=w['observed_statuses'][path];d=load(Path(path),row['identity'])
        require(row['status']==row['required']==d['status']==required,'terminal prerequisite exact status')
    require([g['generation'] for g in f['generations']]==[1,2],'two complete generations')
    generations=[]; fresh_commands=[]; command_rows=[]
    for g in f['generations']:
        n=g['generation'];c=g['composite_cases'];root=B/f'whole-proof-attacks-v5-c{n}';work=root/f'whole-c{n}'
        suite=load(work/'receipt.json',c['suite']);prior=suite['prior_cases']
        require(suite['immutable_inputs_before']==suite['immutable_inputs_after'],'terminal suite unchanged inputs')
        require(not any(k in suite for k in ('error','input_error')),'suite has no hidden failure')
        require(g['proof']==suite['original_proof']==PROOFS[n-1] and g['compiler']==COMPILER,'original proof/compiler identity')
        require(c['reused_controls']==2 and [x['name'] for x in prior['controls']]==['original-fresh-verification','rechain'],'exact controls')
        fresh=NEW if n==1 else NEW[1:]
        require(c['reused_v2_rejections']==OLD and c['reused_v3_rejections']==MORE and c['fresh_v5_rejections']==fresh,'exact case partition')
        require([r['name'] for r in suite['rejections']]==fresh and c['distinct_rejections']==23,'fresh list and distinct total')
        require(len(set(OLD+MORE+fresh+([] if n==1 else ['cost'])))==23,'no duplicate counted cases')
        require((c['original_v2_status'],c['original_v3_status'],c['original_v4_status'])==('failed','failed','input-changed'),'old failures preserved')
        old2=load(Path(prior['original_v2']['original_suite']['path']),prior['original_v2']['original_suite'])
        old3=load(Path(prior['additional_v3']['suite']['path']),prior['additional_v3']['suite'])
        require(old2['status']==old3['status']=='failed','original failed suites remain failed')
        for label, rows, old in (('v2',prior['original_v2']['rejections'],old2),('v3',prior['additional_v3']['rejections'],old3)):
            oldrows={r['name']:r for r in old['rejections']}
            for row in rows:
                require(row==oldrows[row['name']],'selected original case exact bytes represented')
                oldroot=B/f'whole-proof-attacks-{label}-c{n}'
                check_command(oldroot/row['verification_receipt'],1)
        for control in prior['controls']:
            require(control['certificate']==g['proof'],'original and rechain proof identities')
        positive=[]
        for action,key in [('selfbuild','producer_receipt'),('fresh-verification','verifier_receipt')]:
            directory=B/f'whole-proof/attempts/c{n}-{action}-1';d=load(directory/'receipt.json',g[key])
            require(d['status']=='passed' and d['exit_code']==0 and d['inputs_before']==d['inputs_after'],'positive actual command')
            require(d['binary']==d['binary_after'],'positive runtime unchanged')
            streams(directory,d);require(read(directory/'stderr')==b'','positive stderr empty')
            envelope=json.loads(read(directory/'stdout'));require(envelope['ok'] is True,'positive envelope success');result=envelope['verification'];positive.append(result)
            if action=='fresh-verification':
                require(result==g['verification'] and {k:d['proof_input'][k] for k in ('bytes','sha256')}==d['proof_input_after']==g['proof'],'fresh verification exact relation')
                require(d['files']['compiler.dag']==g['compiler'],'fresh extracted compiler identity')
        verification=g['verification']
        require(verification['format']=='joy-nox-disclosed-compiler-v1' and verification['disclosure']=='complete public witness'
                and verification['physical_resource_claim']=='unattested','precise proof claim')
        for key in ('program_particle','input_particle','output_particle','compiler_job','charged_reductions','expanded_steps','logical_peak_frames'):
            require(positive[0][key]==positive[1][key],'producer/fresh verifier semantic coordinates')
        corpusdir=B/f'whole-proof-corpus-v2/c{n+1}';corpus=load(corpusdir/'receipt.json',g['corpus_receipt'])
        require(corpus['status']=='passed' and corpus['observations']==g['corpus_observations']==547,'actual corpus completeness')
        require(corpus['generation']==n+1 and corpus['proof_generation']==n and corpus['verified_receipt']==g['verifier_receipt'],'actual corpus generation binding')
        require(corpus['inputs_before']==corpus['inputs_after'] and len(corpus['corpora'])==6,'corpus input/source invariance')
        for row in corpus['corpora'].values():same(corpusdir/row['path'],row)
        actual=[]
        for row in suite['rejections']:
            name=row['name']; recipe=row['recipe'];mutant=work/f'certificate-{name}.joysc'
            require(recipe['mode']==name and recipe['context']==[] and recipe['certificate']==row['certificate'],'actual construction recipe')
            for mode,key,exit_code in [('construct','construction_receipt',0),('verify','verification_receipt',1)]:
                relative=recipe[key] if mode=='construct' else row[key]
                path=root/relative;expected=root/f'attempts/whole-c{n}-{mode}-{name}/receipt.json'
                require(path==expected,'case command path')
                d=check_command(path,exit_code,n);actual.append(reference(path));fresh_commands.append(path);command_rows.append(d)
                require(suite['started_ns']<=d['started_ns']<=d['ended_ns']<=suite['ended_ns'],'command inside suite')
                require(d['argv'][1]==('mutate' if mode=='construct' else 'verify-artifact'),'actual native mode')
                if mode=='verify':
                    require(d['metadata']['expected_error']==row['error'],'expected negative diagnostic')
                    require(read(path.parent/'stdout')==b'' and row['error'] in read(path.parent/'stderr').decode(),'actual rejection and no success output')
                    require(d['inputs_before'][str(mutant)]==row['certificate'],'verified exact constructed mutant')
                regpath=V/'native-processes'/f'whole-c{n}-{mode}-{name}.json';reg=load(regpath)
                observed=load(regpath.with_suffix('.observed'));retired=load(regpath.with_suffix('.retired'))
                require(reg==observed and reg['argv']==d['argv'] and reg['pid']==d['pid'],'registered and observed native owner')
                same(regpath,retired['original']);require(retired['final_observation']=={'status':'empty','members':[]},'native retired empty')
                require(d['started_ns']<=reg['registered_ns']<=retired['time_ns']<=d['ended_ns'],'native registration interval')
                same(Path(reg['capacity_permit']['path']),reg['capacity_permit'])
                same(path.parent/'resources.jsonl',d['files']['resources.jsonl'])
            same(work/(name+'.dag'),row['protected_output']);require(not mutant.exists(),'no surviving completed fresh mutant')
        require(actual==c['actual_v5_commands'],'final checker command references exact')
        if n==2:
            selected=load(work/'selected-v4-cost.json',c['c2_cost']['selected'])
            require(selected==suite['selected_v4_cost'],'selected historical cost exact object')
            require(selected['status']=='passed-selected-v4-cost' and selected['generation']==2,'selected V4 scope')
            for key in ('verification','retirement'):same(Path(c['c2_cost'][key]['path']),c['c2_cost'][key])
            check_command(Path(c['c2_cost']['verification']['path']),1,2)
            require(c['c2_cost']['original_suite_status']=='input-changed' and c['c2_cost']['original_controller_status']=='failed','no relabelled V4 success')
        else:require(c['c2_cost'] is None and suite['selected_v4_cost'] is None,'no C1 selected cost')
        generations.append(dict(generation=n,proof=g['proof'],compiler=g['compiler'],controls=2,prior_v2=9,prior_v3=3,
            selected_v4_cost=n==2,fresh_v5=len(fresh),distinct_rejections=23,corpus_observations=547,suite=reference(work/'receipt.json')))
    require(len(fresh_commands)==42,'exact 42 fresh native commands')
    schedule=f['completion_schedule'];same(V/'run-1/receipt.json',schedule['receipt'])
    require(schedule['native_commands']==42 and schedule['capacity_permits']==23,'actual schedule totals')
    require(len(list((V/'native-processes').glob('*.json')))==42,'no extra native registrations')
    require(len(list((V/'capacity').glob('*.json')))==23,'exact capacity permit inventory')
    require(len(list((V/'capacity').glob('*.jsonl')))==23,'exact capacity observation inventory')
    completed={n:load(V/f'completed-c{n}.json',schedule['completed'][str(n)]) for n in (1,2)}
    admissions={n:load(V/f'admission-c{n}.json',schedule['admissions'][str(n)]) for n in (1,2)}
    require(completed[1]['quiescent_ns']<admissions[2]['time_ns'],'strict sequential generation admission')
    for n in (1,2):
        require(completed[n]['status']=='passed' and completed[n]['exit_code']==0 and v['children_final'][str(n)]['exit_code']==0,'successful suite child closure')
        require(v['children_final'][str(n)]['pid']==admissions[n]['suite_binding']['pid'],'exact suite child identity')
        require(admissions[n]['reservation']['accepted'] is True and not admissions[n]['reservation']['reasons'],'actual capacity accepted')
    raw=read(V/'run-1/resources.jsonl',v['files']['resources.jsonl']);count=0;previous=v['started_ns'];peak=0
    for line in raw.splitlines():
        row=json.loads(line);count+=1
        require(previous<=row['time_ns']<=v['ended_ns'],'ordered in-run physical observations');previous=row['time_ns']
        for key in ('rss_bytes','free_bytes','owned_bytes'):
            require(type(row[key]) is int and row[key]>=0,'nonnegative resource observation')
        require(row['rss_bytes']<=12*1024**3 and row['owned_bytes']<=26*1024**3 and row['free_bytes']>=8*1024**3,'unchanged physical profile')
        require(row['reason'] is None,'no observed refusal hidden')
        require(sum(p['rss_kib']*1024 for p in row['processes'])==row['rss_bytes'],'actual sampled RSS arithmetic')
        require(len({p['pid'] for p in row['processes']})==len(row['processes']),'no duplicate process accounting')
        require(sum(len(x['mutants']) for x in row['generations'].values())<=1,'one simultaneous actual mutant')
        peak=max(peak,row['rss_bytes'])
    require(count==schedule['samples'] and peak==v['sampled_peak_rss_bytes']==schedule['sampled_peak_rss_bytes'],'actual resource count and peak')
    require(v['latest_sample']==row and v['final_sample']['time_ns']>=row['time_ns'],'latest/final resource observations')
    require(not any(x['mutants'] for x in v['final_sample']['generations'].values()),'no terminal mutant')
    for name, identity in v['files'].items():same(V/'run-1'/name,identity)
    for n in (1,2):require(read(V/f'run-1/c{n}.stderr')==b'','no suite child stderr')
    # Bind all explicit retained-history references without rereading artifact/native bodies.
    def refs(value):
        if isinstance(value,dict):
            if {'path','bytes','sha256'}<=set(value):
                path=Path(value['path'])
                if path.is_absolute() and path.is_relative_to(B):
                    if path.suffix in ('.dag','.joysc') or path.name in ('joy','whole-proof-mutator'):skipped[str(path.relative_to(B))]={k:value[k] for k in ('bytes','sha256')}
                    else:same(path,value)
            for child in value.values():refs(child)
        elif isinstance(value,list):
            for child in value:refs(child)
    refs(f)
    return dict(status='passed-actual-metadata-review',schema='trident/whole-proof-actual-independent-review/v1',
        actual_F5=reference(W/'final-review.json'),actual_watcher=reference(W/'receipt.json'),actual_V5=reference(V/'run-1/receipt.json'),
        generations=generations,source_map=reference(F/'sources.json'),v5_source_map=reference(V/'sources.json'),
        native_commands=42,capacity_permits=23,resource_samples=count,sampled_peak_rss_bytes=peak,
        scope='Independent actual terminal receipt, source, case/command, output, retained-failure and sampled process/resource review. No native execution, checker rerun or proof/compiler/native-body reread. Physical resources are sampled observations; proof disclosure is complete public witness. Durable retention, package delivery and publication are separate.',
        artifact_body_identity_refs_not_reread=skipped)


if __name__=='__main__':
    report={'status':'failed','started_ns':started,'command':[sys.executable,*sys.argv]}
    try:report.update(main())
    except BaseException as error:
        report['error']=str(error)
        raise
    finally:
        report['ended_ns']=time.time_ns();report['read_bytes']=read_bytes
        report['observed_metadata_files']=len(seen)
        report['metadata_identities']={str(Path(k).relative_to(B)):v for k,v in sorted(seen.items())}
        destination=O/sys.argv[1]
        with destination.open('x') as out:json.dump(report,out,indent=2);out.write('\n')
        print(json.dumps({k:report[k] for k in ('status','read_bytes','observed_metadata_files')}))
