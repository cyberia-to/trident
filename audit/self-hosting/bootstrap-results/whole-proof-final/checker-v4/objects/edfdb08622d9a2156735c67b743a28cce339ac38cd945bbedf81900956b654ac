"""Replay v3 reservations, retained scopes, registrations and final observations."""
import json
from pathlib import Path
from common import require, identity, load, same, files

GIB, MIB = 1024**3, 1024**2
RETAINED = ('whole-proof-attacks', 'whole-proof-attacks-v2-c1',
            'whole-proof-attacks-v2-c2', 'whole-proof-attacks-parallel-v2')


def resource_contract(plan):
    require(plan['schema'] == 'trident/complete-proof-missing-cases/v3', 'explicit composite schedule schema')
    expected = dict(shared_disk_bytes=26*GIB, free_floor_bytes=8*GIB,
                    combined_sampled_rss_bytes=12*GIB, mutant_growth_ceiling_bytes=32*MIB,
                    canonical_and_output_bucket_per_generation=112*MIB,
                    metadata_bucket_per_generation=256*MIB, per_stream_log_bytes=MIB,
                    readiness_seconds=15500, outer_schedule_seconds=86400,
                    fresh_rejections_per_generation=14, replayed_rejections_per_generation=9,
                    replayed_controls_per_generation=2)
    require(all(plan[k] == value for k, value in expected.items()), 'unchanged v3 physical bounds and counts')
    require(plan['roots'] == {'1': 'whole-proof-attacks-v3-c1', '2': 'whole-proof-attacks-v3-c2'}
            and plan['retained_scopes'] == list(RETAINED) and plan['generations'] == [1, 2],
            'complete original and fresh owned scope inventory')


def reservation(plan, baseline):
    existing = sum(baseline['files'].values())
    mutants = sum(plan['proofs'][str(g)]['bytes'] + 32*MIB for g in (1, 2))
    total = existing + mutants + 2*112*MIB + 3*256*MIB
    minimum_free = max(total - existing + 8*GIB, mutants + 10*GIB)
    return existing, total, minimum_free


def sample(row, plan, base, version=3):
    require(row['reason'] is None and 0 <= row['owned_bytes'] <= 26*GIB
            and row['free_bytes'] >= 8*GIB and 0 <= row['rss_bytes'] <= 12*GIB,
            'shared sampled resource verdict')
    require(set(row['generations']) == {'1', '2'}, 'both generation resource observations')
    for g, observed in row['generations'].items():
        require(len(observed['mutants']) <= 1, 'one live mutant per generation')
        for path, size in observed['mutants'].items():
            p = Path(path)
            require(p.parent == base / f'whole-proof-attacks-v{version}-c{g}/whole-c{g}'
                    and p.suffix == '.joysc' and 0 <= size <= plan['proofs'][g]['bytes'] + 32*MIB,
                    'bounded owned generation mutant')
        require(0 <= observed['canonical_output_bytes'] <= 112*MIB
                and 0 <= observed['metadata_bytes'] <= 256*MIB, 'generation output/metadata buckets')
    require(0 <= row['coordinator_metadata_bytes'] <= 256*MIB, 'coordinator metadata bucket')
    require(all(type(p['pid']) is int and p['pid'] > 0 and type(p['rss_kib']) is int
                and p['rss_kib'] >= 0 for p in row['processes'])
            and len({p['pid'] for p in row['processes']}) == len(row['processes']), 'valid unique raw process rows')
    require(row['rss_bytes'] == sum(p['rss_kib']*1024 for p in row['processes']), 'raw process RSS sum')


def sample_time(row, run, previous=None):
    require(all(type(run[k]) is int for k in ('started_ns','ended_ns'))
            and 0 < run['started_ns'] <= run['ended_ns'] and type(row['time_ns']) is int
            and run['started_ns'] <= row['time_ns'] <= run['ended_ns']
            and (previous is None or previous['time_ns'] <= row['time_ns']),
            'ordered resource sample belongs to actual run interval')


def terminal_samples(last, run):
    require(last == run['latest_sample'], 'latest observation is the last timed sample')
    sample_time(run['final_sample'],run,last)


def terminal_children(run, admission):
    require([row['generation'] for row in run['generations']] == [1,2]
            and set(admission['children']) == set(run['children_final']) == {'1','2'}, 'both final child identities')
    for row in run['generations']:
        g = str(row['generation'])
        require(type(row['pid']) is int and row['pid'] > 0 and row['pid'] == admission['children'][g]
                and run['children_final'][g] == dict(pid=row['pid'],exit_code=0), 'actual final child exited successfully')


def native_retirement(path, run):
    registered = load(path)
    retired = path.with_suffix('.retired')
    require(retired.is_file(), 'native group observed empty')
    raw = retired.read_text()
    require(raw.endswith('\n') and raw[:-1].isascii() and raw[:-1].isdigit(),
            'numeric native retirement observation')
    require(type(registered['registered_ns']) is int
            and run['started_ns'] <= registered['registered_ns'] <= int(raw) <= run['ended_ns'],
            'native registration and retirement belong to actual run interval')


def review(base):
    shared = base / 'whole-proof-attacks-completion-v3'
    plan, admission = load(shared/'plan.json'), load(shared/'admission.json')
    resource_contract(plan)
    reviewed = load(shared/'independent-review.json')
    manifest = load(shared/'sources.json')
    require(reviewed['status'] == 'passed-source-review' and reviewed['sources'] == manifest,
            'independent exact source review')
    for path, expected in manifest.items():
        same(Path(path), expected)
    require(not (shared/'stop.json').exists(), 'v3 stop must never be relabelled')
    run = load(shared/'run-1/receipt.json')
    require(run['schema'] == 'trident/complete-proof-missing-cases-orchestration/v3'
            and run['status'] == 'passed' and run['inputs_unchanged'] is True,
            'successful complete v3 orchestration')
    same(shared/'plan.json', run['plan'])
    same(shared/'independent-review.json', run['independent_review'])
    require(run['source'] == manifest and len(run['generations']) == 2, 'reviewed complete generation launch')
    terminal_children(run,admission)
    same(shared/'plan.json', admission['plan'])
    same(shared/'sources.json', admission['source_manifest'])
    baseline = load(shared/'baseline.json')
    existing, total, minimum_free = reservation(plan, baseline)
    require(run['admission_existing_bytes'] == existing
            and run['admission_reservation_bytes'] == admission['reservation_bytes'] == total <= 26*GIB,
            'recomputed full shared reservation')
    require(run['admission_free_bytes'] == admission['free_bytes'] >= minimum_free,
            'original growth headroom and free floor')
    static = {p for p in baseline['files'] if not Path(p).is_relative_to(shared/'run-1')}
    require(set(baseline['immutable']) == static, 'all admitted static file identities')
    allowed = [base/p for p in (*RETAINED, *plan['roots'].values(), shared.name)]
    for path, size in baseline['files'].items():
        require(any(Path(path).is_relative_to(p) for p in allowed), 'baseline belongs to charged scope')
        if path in static:
            require(baseline['immutable'][path]['bytes'] == size, 'baseline size binding')
            same(Path(path), baseline['immutable'][path])
    for name in RETAINED:
        current = {str(p) for p in (base/name).rglob('*')
                   if p.is_file() and 'target-helper' not in p.relative_to(base/name).parts}
        require(current <= static, 'every retained old file remains charged and immutable')
    copied = load(shared/'copied-inputs.json')
    require(set(copied) == {'1', '2'}, 'both copied generation inventories')
    for g, entries in copied.items():
        for name, original in entries.items():
            same(Path(original['original']), original)
            destination = base / f'whole-proof-attacks-v3-c{g}' / name
            if name in ('guard.py', 'whole_suite.py'):
                require(str(destination) in manifest, 'adapted source explicitly reviewed')
            else:
                same(destination, original)
    for g, row in zip((1, 2), run['generations']):
        root = base / f'whole-proof-attacks-v3-c{g}'
        python = row['command'][0]
        require(row['generation'] == g and row['status'] == 'passed' and row['exit_code'] == 0
                and row['pid'] == admission['children'][str(g)] and row['cwd'] == str(root)
                and Path(python).is_absolute()
                and row['command'] == [python, '-B', str(root/'whole_suite.py'), '--generation', str(g)],
                'exact successful fresh generation process')
        same(root/f'whole-c{g}/receipt.json', row['suite_receipt'])
        require(not list((root/f'whole-c{g}').glob('certificate-*.joysc')), 'all completed fresh mutants reclaimed')
        require(set(row['logs']) == {'stdout', 'stderr'}, 'both generation streams')
        for name, expected in row['logs'].items():
            same(shared/f'run-1/c{g}.{name}', expected)
    require(set(run['files']) == {'c1.stdout', 'c1.stderr', 'c2.stdout', 'c2.stderr', 'resources.jsonl'},
            'complete orchestration log inventory')
    files(shared/'run-1', run['files'])
    count = peak = 0
    last = None
    with (shared/'run-1/resources.jsonl').open() as stream:
        while line := stream.readline(4*MIB+1):
            require(len(line) <= 4*MIB, 'bounded resource row')
            observed = json.loads(line)
            sample(observed, plan, base)
            sample_time(observed,run,last)
            last = observed
            count += 1
            peak = max(peak, observed['rss_bytes'])
    require(count > 0 and peak == run['sampled_peak_rss_bytes'], 'actual timed sampled peak')
    terminal_samples(last,run)
    sample(run['final_sample'], plan, base)
    for path in (shared/'native-processes').glob('*.json'):
        native_retirement(path,run)
    import history
    transition = history.transition(base, shared)
    return dict(receipt=identity(shared/'run-1/receipt.json'), transition=transition,
                shared_sampled_peak_rss_bytes=max(peak, run['final_sample']['rss_bytes']),
                reused_controls_per_generation=2, reused_rejections_per_generation=9,
                fresh_rejections_per_generation=14, distinct_rejections_per_generation=23)
