"""Replay the failed schedule and its explicit, separately reviewed reclamation."""
import json
from pathlib import Path
from common import require, identity, load, same, files, PRIOR
from schedule import GIB, MIB, RETAINED, reservation, sample

TARGET = 'whole-proof-attacks-v2-c2/whole-c2/certificate-rebound-job-limit.joysc'
MUTANT = dict(bytes=10569174820, sha256='2f6ce311471e969b92d2d23fda33077bfe7bd3895fe2d415c8e6e0c08b94a4ae')
PARTIALS = {
    'whole-proof-attacks/whole-c2/certificate-rechain.joysc': dict(
        bytes=920494713, sha256='3aa4ea4bc79ace206166c23952c104d22effb69fd0b80bd9a4d32b834f1477a1'),
    'whole-proof-attacks-v2-c1/whole-c1/certificate-rebound-job-limit.joysc': dict(
        bytes=878826753, sha256='5adc09f6d706e9c001b350a7d962f3902ec51cc065ea6670f7524429ce0837fb'),
}


def prior_schedule(base):
    root = base/'whole-proof-attacks-parallel-v2'
    run, plan, admission = (load(root/p) for p in ('run-1/receipt.json','plan.json','admission.json'))
    require(run['schema'] == 'trident/parallel-whole-proof-orchestration/v2'
            and run['status'] == 'failed', 'original complete schedule remains failed')
    expected = dict(shared_disk_bytes=26*GIB, free_floor_bytes=8*GIB,
                    combined_sampled_rss_bytes=12*GIB, mutant_growth_ceiling_bytes=32*MIB,
                    canonical_and_output_bucket_per_generation=112*MIB,
                    metadata_bucket_per_generation=256*MIB, per_stream_log_bytes=MIB,
                    readiness_seconds=15500, outer_schedule_seconds=86400)
    require(all(plan[k] == v for k,v in expected.items()), 'all original aggregate bounds retained')
    review, manifest = load(root/'independent-review.json'), load(root/'sources.json')
    require(review['status'] == 'passed-source-review' and review['sources'] == manifest == run['source'],
            'original reviewed aggregate source')
    for path, expected_identity in manifest.items():
        same(Path(path), expected_identity)
    same(root/'independent-review.json', run['independent_review'])
    same(root/'plan.json', run['plan'])
    same(root/'plan.json', admission['plan'])
    same(root/'sources.json', admission['source_manifest'])
    baseline = load(root/'baseline.json')
    existing, total, floor = reservation(plan, baseline)
    require(run['admission_existing_bytes'] == existing
            and run['admission_reservation_bytes'] == admission['reservation_bytes'] == total <= 26*GIB
            and run['admission_free_bytes'] == admission['free_bytes'] >= floor,
            'actual original aggregate reservation and headroom')
    for path, expected_identity in baseline['immutable'].items():
        same(Path(path), expected_identity)
    stop = load(root/'stop.json')
    require(stop['reason'] == 'suite-failed' and stop['detail'] == {'generation':2,'exit_code':1},
            'specific original diagnostic failure; no resource-cap waiver')
    shutdown = load(root/'shutdown.json')
    require(shutdown['all_owned_groups_empty'] is True and shutdown['remaining'] == [], 'original shutdown complete')
    require(set(run['files']) == {'c1.stdout','c1.stderr','c2.stdout','c2.stderr','resources.jsonl'},
            'complete original aggregate logs')
    files(root/'run-1', run['files'])
    count = peak = 0
    last = None
    with (root/'run-1/resources.jsonl').open() as stream:
        while line := stream.readline(4*MIB+1):
            require(len(line) <= 4*MIB, 'bounded original sample')
            row = json.loads(line)
            sample(row, plan, base, version=2)
            require(run['started_ns'] <= row['time_ns'] <= run['ended_ns']
                    and (last is None or last['time_ns'] <= row['time_ns']), 'original sample time order')
            peak = max(peak,row['rss_bytes'])
            count += 1
            last = row
    require(count > 0 and peak == run['sampled_peak_rss_bytes'] and last == run['latest_sample'],
            'original aggregate samples/peak exactly bound')
    if run.get('final_sample') is not None:
        sample(run['final_sample'],plan,base,version=2)
        peak = max(peak,run['final_sample']['rss_bytes'])
    require(len(run['generations']) == 2 and set(run['children_final']) == {'1','2'}, 'both original generation processes')
    for g,row in zip((1,2),run['generations']):
        scope = base/f'whole-proof-attacks-v2-c{g}'
        python = row['command'][0]
        require(row['generation'] == g and row['pid'] == admission['children'][str(g)]
                and Path(python).is_absolute() and row['cwd'] == str(scope)
                and row['command'] == [python,'-B',str(scope/'whole_suite.py'),'--generation',str(g)]
                and run['children_final'][str(g)] == dict(pid=row['pid'],exit_code=1), 'actual stopped original children')
        suite = load(scope/f'whole-c{g}/receipt.json')
        require(suite['status'] == 'failed' and [r['name'] for r in suite['rejections']] == list(PRIOR),
                'original selected rows retain enclosing failure')
        paths = [scope/f'attempts/whole-c{g}-index/receipt.json']
        for case in [suite['controls'][1],*suite['rejections']]:
            paths.append(scope/case['verification_receipt'])
            if case['recipe'] is not None:
                paths.append(scope/case['recipe']['construction_receipt'])
        for path in paths:
            command = load(path)
            require(run['started_ns'] <= command['started_ns'] <= command['ended_ns'] < stop['time_ns'],
                    'only completed command evidence before the original shared stop')
    return dict(status='passed-original-resource-replay', original_schedule_status='failed',
                receipt=identity(root/'run-1/receipt.json'), samples=count,
                sampled_peak_rss_bytes=peak, last_observation=last['time_ns'],
                original_failure=identity(root/'stop.json'), shutdown=identity(root/'shutdown.json'))


def transition(base, shared):
    document = load(shared/'transition.json')
    require(document['status'] == 'prior-attempts-classified-and-quiescent', 'explicit poststop transition')
    expected_partials = {str(base/p): value for p,value in PARTIALS.items()}
    require(document['retained_partial_mutants'] == expected_partials, 'exact original incomplete partials retained')
    for path, expected in expected_partials.items():
        same(Path(path),expected)
    observed = {str(p) for name in RETAINED for p in (base/name).glob('whole-c*/certificate-*.joysc')}
    require(observed == set(expected_partials), 'only classified old partials remain')
    retained = document['files']
    for path, expected in retained.items():
        require(Path(path).is_absolute() and Path(path).is_relative_to(base), 'local transition evidence path')
        same(Path(path),expected)
    packet = base/'whole-reclamation-review'
    required = {shared/p for p in ('reclaim_completed.py','reclamation-plan.json','reclamation-review.json',
                                  'reclamation-action/receipt.json','reclamation-action/processes.stdout',
                                  'reclamation-action/processes.stderr')}
    required.update(packet/p for p in ('classification.json','comparison.json','retain.py','compare.py'))
    old = base/'whole-proof-attacks-parallel-v2'
    required.update(old/p for p in ('stop.json','shutdown.json','run-1/receipt.json','transition.json'))
    required.update(base/f'whole-proof-attacks-v2-c{g}/whole-c{g}/receipt.json' for g in (1,2))
    require({str(p) for p in required} <= set(retained), 'complete transition evidence inventory')
    action_path = shared/'reclamation-action/receipt.json'
    require(document['reclamation_receipt']['path'] == str(action_path), 'exact action receipt path')
    same(action_path,document['reclamation_receipt'])
    action, plan, review = (load(shared/p) for p in ('reclamation-action/receipt.json','reclamation-plan.json','reclamation-review.json'))
    require(action['schema'] == 'trident/classified-temporary-reclamation/v1' and action['status'] == 'passed'
            and action['removed'] is True and action['original_failed_evidence_unchanged'] is True
            and action['target'] == plan['target'] == TARGET
            and action['complete_mutant_identity'] == plan['identity'] == MUTANT,
            'one exact reviewed completed mutant reclaimed')
    require(review['status'] == 'passed-reclamation-source-review' and review['sources'] == {
        'reclaim_completed.py':identity(shared/'reclaim_completed.py'),
        'reclamation-plan.json':identity(shared/'reclamation-plan.json')}, 'exact independent reclamation review')
    for field,name in (('source','reclaim_completed.py'),('plan','reclamation-plan.json'),('review','reclamation-review.json')):
        same(shared/name,action[field])
    python = action['command'][0]
    require(Path(python).is_absolute() and action['command'] == [python,str(shared/'reclaim_completed.py'),
            '--review-sha256',identity(shared/'reclamation-review.json')['sha256']], 'exact reviewed reclamation invocation')
    same(packet/'classification.json',action['classification'])
    require(action['classification'] == plan['classification'], 'classified receipt identity')
    classified = load(packet/'classification.json')
    require(classified['status'] == 'passed-readonly-classification'
            and classified['classification']['completed_mutant'] == MUTANT
            and classified['classification']['original_suite_status'] == 'failed'
            and classified['classification']['intended_semantic_case_status'] == 'failed', 'wrong-context case remains failed')
    same(packet/'comparison.json',classified['comparison'])
    comparison = load(packet/'comparison.json')
    require(comparison['status'] == 'passed-readonly-comparison', 'complete read-only byte comparison')
    payload = comparison['comparison']
    require(payload['states_before'][1] == payload['states_after'][1] == plan['state']
            and payload['sha256'][1] == MUTANT['sha256'] and payload['bytes'][1] == MUTANT['bytes']
            and payload['equal_header_first18_count'] == payload['equal_payload_count'] == payload['frames'] == 422849
            and payload['single_terminal_exact_eof'] is True, 'complete classified same-payload mutant')
    for name,expected in classified['retained'].items():
        for p in (base/name,packet/'retained'/name):
            require(str(p) in retained and retained[str(p)] == expected, 'original and retained failure copies bound')
            same(p,expected)
    for name,expected in classified['sources'].items():
        same(packet/name,expected)
    require(action['retained_partials'] == plan['retained_partials'] == PARTIALS, 'both unfinished originals unchanged')
    target = base/TARGET
    require(not target.exists() and not target.is_symlink(), 'only classified completed temporary absent')
    admission = load(old/'admission.json')
    groups = {admission['pid'],*admission['children'].values()}
    for path in (old/'native-processes').glob('*.json'):
        native = load(path)
        require(native['coordinator'] == admission['pid'] and path.with_suffix('.retired').is_file(), 'prior process ownership retired')
        groups.add(native['pgid'])
    require(action['quiescence'] == dict(command=['/bin/ps','-axo','pid=,pgid='],
            checked_groups=sorted(groups),no_live_owners=True), 'exact recorded owner quiescence check')
    process_file = shared/'reclamation-action/processes.stdout'
    require(process_file.stat().st_size <= 4*MIB and (shared/'reclamation-action/processes.stderr').stat().st_size == 0,
            'bounded successful process inventory')
    processes = [tuple(map(int,line.split())) for line in process_file.read_text().splitlines()]
    require(processes and all(len(row)==2 for row in processes)
            and not any(pid in groups or group in groups for pid,group in processes), 'actual prior groups absent')
    return dict(receipt=identity(shared/'transition.json'), reclamation=identity(action_path),
                original_v2_status='failed', classified_failed_case_accepted=False,
                retained_partial_mutants=expected_partials)
