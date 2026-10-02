"""Tiny isolated metadata replay; never reads or runs active whole certificates."""
import copy
import json
from pathlib import Path
import sys
import tempfile
from unittest.mock import patch

SOURCE = Path(__file__).resolve().parent.parent / 'whole-proof-final-review-v3'
sys.path.insert(0, str(SOURCE))
import common
import history
import schedule


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value) + '\n')


def replay(change=None):
    with tempfile.TemporaryDirectory() as d:
        base = Path(d)
        shared = base / 'whole-proof-attacks-completion-v3'
        plan = dict(schema='trident/complete-proof-missing-cases/v3',
            shared_disk_bytes=26*schedule.GIB, free_floor_bytes=8*schedule.GIB,
            combined_sampled_rss_bytes=12*schedule.GIB, mutant_growth_ceiling_bytes=32*schedule.MIB,
            canonical_and_output_bucket_per_generation=112*schedule.MIB,
            metadata_bucket_per_generation=256*schedule.MIB, per_stream_log_bytes=schedule.MIB,
            readiness_seconds=15500, outer_schedule_seconds=86400,
            fresh_rejections_per_generation=14, replayed_rejections_per_generation=9,
            replayed_controls_per_generation=2, generations=[1, 2],
            roots={str(g): f'whole-proof-attacks-v3-c{g}' for g in (1, 2)},
            retained_scopes=list(schedule.RETAINED), proofs={str(g): dict(bytes=1) for g in (1, 2)})
        write(shared/'plan.json', plan)
        write(shared/'sources.json', {})
        write(shared/'independent-review.json', dict(status='passed-source-review', sources={}))
        write(shared/'baseline.json', dict(files={}, immutable={}))
        write(shared/'copied-inputs.json', {'1': {}, '2': {}})
        existing, reserved, floor = schedule.reservation(plan, dict(files={}))
        write(shared/'admission.json', dict(plan=common.identity(shared/'plan.json'),
            source_manifest=common.identity(shared/'sources.json'), reservation_bytes=reserved,
            free_bytes=floor, children={'1': 501, '2': 502}))
        observed = dict(time_ns=150, reason=None, owned_bytes=0, free_bytes=floor,
            rss_bytes=0, processes=[], coordinator_metadata_bytes=0,
            generations={str(g): dict(mutants={}, canonical_output_bytes=0, metadata_bytes=0)
                         for g in (1, 2)})
        samples = [observed]
        run = dict(schema='trident/complete-proof-missing-cases-orchestration/v3',
            status='passed', inputs_unchanged=True, started_ns=100, ended_ns=200,
            plan=common.identity(shared/'plan.json'),
            independent_review=common.identity(shared/'independent-review.json'), source={},
            admission_existing_bytes=existing, admission_reservation_bytes=reserved,
            admission_free_bytes=floor, generations=[], sampled_peak_rss_bytes=0,
            latest_sample=copy.deepcopy(observed), final_sample=dict(observed, time_ns=199),
            children_final={str(g): dict(pid=500+g, exit_code=0) for g in (1, 2)})
        for g in (1, 2):
            root = base / plan['roots'][str(g)]
            receipt = root/f'whole-c{g}/receipt.json'
            write(receipt, dict(status='passed-completion'))
            for name in ('stdout', 'stderr'):
                write(shared/f'run-1/c{g}.{name}', {})
            run['generations'].append(dict(generation=g, status='passed', exit_code=0,
                pid=500+g, cwd=str(root), command=['/synthetic/python', '-B', str(root/'whole_suite.py'),
                                                '--generation', str(g)],
                suite_receipt=common.identity(receipt),
                logs={name:common.identity(shared/f'run-1/c{g}.{name}') for name in ('stdout', 'stderr')}))
        if change:
            change(run, samples)
        (shared/'run-1/resources.jsonl').write_text(''.join(json.dumps(s)+'\n' for s in samples))
        run['files'] = {name:common.identity(shared/'run-1'/name) for name in
            ('c1.stdout','c1.stderr','c2.stdout','c2.stderr','resources.jsonl')}
        write(shared/'run-1/receipt.json', run)
        # Reclamation is an independently reviewed function and outside this
        # metadata fixture. All schedule file/identity/cap checks remain real.
        with patch.object(history, 'transition', return_value={'synthetic': True}):
            return schedule.review(base)


if __name__ == '__main__':
    probes = {
        'control': None,
        'resource_row_outside_run': lambda r,s: s[0].update(time_ns=1000),
        'recorded_latest_differs': lambda r,s: r['latest_sample'].update(time_ns=151),
        'final_sample_outside_run': lambda r,s: r['final_sample'].update(time_ns=1000),
        'child_terminal_failure': lambda r,s: r['children_final']['1'].update(exit_code=1),
        'descending_samples': lambda r,s: s.append(dict(s[0], time_ns=149)),
    }
    outcomes = {}
    for name, change in probes.items():
        try:
            replay(change)
            outcomes[name] = {'accepted': True}
        except Exception as error:
            outcomes[name] = {'accepted': False, 'error': str(error)}
    print(json.dumps(outcomes, indent=2))
