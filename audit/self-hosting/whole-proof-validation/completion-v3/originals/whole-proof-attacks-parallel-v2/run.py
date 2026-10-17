"""Run the unchanged diagnostic matrix concurrently under one disk reservation."""
import fcntl
import json
import os
from pathlib import Path
import selectors
import shutil
import signal
import subprocess
import sys
import time
import traceback

import resources as shared

ROOT, BASE, PLAN = shared.ROOT, shared.BASE, shared.PLAN


def require(value, message):
    if not value:
        raise ValueError(message)


def load(path):
    return json.loads(Path(path).read_text())


def cancelled(signum, _frame):
    raise InterruptedError('coordinator signal: ' + str(signum))


def process_identity(pid):
    result = subprocess.run(['/bin/ps', '-p', str(pid), '-o', 'pgid=,lstart='],
                            capture_output=True, text=True)
    return result.stdout.strip() if result.returncode == 0 else None


def stop_all(children):
    rows = shared.process_snapshot(os.getpid())
    groups = {}
    for row in rows:
        if row['pgid'] != os.getpgrp():
            groups.setdefault(row['pgid'], {})[row['pid']] = process_identity(row['pid'])
    def owned(members):
        # A surviving original member binds even a leaderless group. Process
        # birth and group identity remain stable through a legitimate exec.
        return any(expected is not None and process_identity(pid) == expected
                   for pid, expected in members.items())
    for group, members in groups.items():
        if owned(members):
            try:
                os.killpg(group, signal.SIGTERM)
            except ProcessLookupError:
                pass
    deadline = time.monotonic() + 12
    while time.monotonic() < deadline and any(owned(m) for m in groups.values()):
        time.sleep(0.2)
    for group, members in groups.items():
        if group in {p.pid for p in children.values()}:
            continue  # Suite parents may still be retaining input hashes.
        if owned(members):
            try:
                os.killpg(group, signal.SIGKILL)
            except ProcessLookupError:
                pass
    for child in children.values():
        if child.poll() is None:
            try:
                child.wait(timeout=60)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait()
    # Re-scan durable registrations after parent cleanup, including a group
    # whose leader disappeared between the initial ps and birth observation.
    final = []
    for _ in range(25):
        final = [row for row in shared.process_snapshot(os.getpid())
                 if row['pgid'] != os.getpgrp()]
        if not final:
            break
        observed = {row['pid']: process_identity(row['pid']) for row in final}
        for row in final:
            expected = observed[row['pid']]
            if expected is not None and process_identity(row['pid']) == expected:
                try:
                    os.killpg(row['pgid'], signal.SIGKILL)
                except ProcessLookupError:
                    pass
        time.sleep(0.2)
    shared.write(ROOT / 'shutdown.json', dict(time_ns=time.time_ns(), remaining=final,
                                            all_owned_groups_empty=not final))
    require(not final, 'owned native processes remain after bounded cleanup')


def execute():
    output = ROOT / 'run-1'
    output.mkdir()
    report = dict(schema='trident/parallel-whole-proof-orchestration/v2', status='prepared',
                  started_ns=time.time_ns(), plan=shared.identity(ROOT / 'plan.json'),
                  generations=[], sampled_peak_rss_bytes=0)
    children, streams = {}, []
    selector = selectors.DefaultSelector()
    def save():
        shared.write(output / 'receipt.json', report)
    save()
    try:
        review_path = ROOT / 'independent-review.json'
        review = load(review_path)
        require(review['status'] == 'passed-source-review', 'independent exact-source review required')
        manifest = load(ROOT / 'sources.json')
        require(review['sources'] == manifest, 'review must bind the complete prepared source manifest')
        require(all(shared.identity(p) == value for p, value in manifest.items()), 'reviewed source changed')
        report.update(source=manifest, independent_review=shared.identity(review_path))
        copied = load(ROOT / 'copied-inputs.json')
        for g, entries in copied.items():
            root = shared.ROOTS[int(g)]
            for name, original in entries.items():
                expected = {k: original[k] for k in ('bytes', 'sha256')}
                require(shared.identity(original['original']) == expected, 'original prepared copy source changed')
                if name in ('guard.py', 'whole_suite.py'):
                    require(str(root / name) in manifest, 'adapted source must have an explicit reviewed identity')
                else:
                    require(shared.identity(root / name) == expected, 'copied input/helper differs from original identity')

        transition = load(ROOT / 'transition.json')
        require(transition['status'] == 'original-schedule-quiescent', 'original suite must be stopped')
        for p, expected in transition['retained_partial_mutants'].items():
            require(shared.identity(p) == {k: expected[k] for k in ('bytes', 'sha256')}, 'original partial retained')
        observed_partials = {str(p) for p in shared.ORIGINAL.glob('whole-c*/certificate-*.joysc')}
        require(observed_partials == set(transition['retained_partial_mutants']), 'every original partial classified')
        for g, root in shared.ROOTS.items():
            require(not (root / 'attempts').exists() and not (root / f'whole-c{g}').exists(), 'fresh generation scope')
            for action, suffix in (('prove', 'selfbuild'), ('verify', 'fresh-verification')):
                directory = BASE / f'whole-proof/attempts/c{g}-{suffix}-1'
                receipt = load(directory / 'receipt.json')
                require(receipt['status'] == 'passed' and receipt['exit_code'] == 0 and
                        receipt['generation'] == g and receipt['action'] == action, 'actual generation readiness')
                if action == 'prove':
                    require(shared.identity(directory / 'proof.joysc') == PLAN['proofs'][str(g)] ==
                            receipt['files']['proof.joysc'], 'actual complete producer proof identity')
                else:
                    require(receipt['proof_input_after'] == PLAN['proofs'][str(g)], 'fresh verifier used exact proof')
        # Static evidence is charged once. Reserve both possible full mutants and
        # every dynamic output/metadata bucket before either process may execute.
        baseline = shared.inventory()
        baseline_hashes = {p: shared.identity(p) for p in baseline
                           if not Path(p).is_relative_to(output)}
        reservation = sum(baseline.values()) + sum(p['bytes'] + PLAN['mutant_growth_ceiling_bytes']
                         for p in PLAN['proofs'].values()) + 2 * PLAN['canonical_and_output_bucket_per_generation'] + \
                      3 * PLAN['metadata_bucket_per_generation']
        growth = reservation - sum(baseline.values())
        free = shutil.disk_usage(ROOT).free
        require(reservation <= PLAN['shared_disk_bytes'], 'both generation reservations exceed shared cap')
        minimum_free = max(growth + PLAN['free_floor_bytes'],
                           sum(p['bytes'] + PLAN['mutant_growth_ceiling_bytes'] for p in PLAN['proofs'].values()) + 10 * 1024**3)
        require(free >= minimum_free, 'free space cannot cover both mutation ceilings plus original headroom')
        shared.write(ROOT / 'baseline.json', dict(files=baseline, immutable=baseline_hashes))
        report.update(status='running', admission_reservation_bytes=reservation,
                      admission_existing_bytes=sum(baseline.values()), admission_free_bytes=free)
        for g, root in shared.ROOTS.items():
            command = [sys.executable, '-B', str(root / 'whole_suite.py'), '--generation', str(g)]
            child = subprocess.Popen(command, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                     env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1'), start_new_session=True)
            children[g] = child
            row = dict(generation=g, status='running', command=command, cwd=str(root), pid=child.pid)
            report['generations'].append(row)
            for label, pipe in (('stdout', child.stdout), ('stderr', child.stderr)):
                stream = (output / f'c{g}.{label}').open('xb')
                streams.append(stream)
                os.set_blocking(pipe.fileno(), False)
                selector.register(pipe, selectors.EVENT_READ, stream)
        admission = dict(pid=os.getpid(), children={str(g): p.pid for g, p in children.items()},
                         reservation_bytes=reservation, free_bytes=free, plan=shared.identity(ROOT / 'plan.json'),
                         source_manifest=shared.identity(ROOT / 'sources.json'), time_ns=time.time_ns())
        pending = ROOT / 'admission.pending'
        shared.write(pending, admission)
        pending.rename(ROOT / 'admission.json')
        save()
        started, next_sample = time.monotonic(), 0
        with (output / 'resources.jsonl').open('x') as samples:
            while any(p.poll() is None for p in children.values()) or selector.get_map():
                for key, _ in selector.select(timeout=0.2):
                    chunk = os.read(key.fileobj.fileno(), 65536)
                    if not chunk:
                        selector.unregister(key.fileobj)
                        key.fileobj.close()
                        continue
                    remaining = PLAN['per_stream_log_bytes'] - key.data.tell()
                    key.data.write(chunk[:max(remaining, 0)])
                    key.data.flush()
                    if len(chunk) > remaining:
                        shared.fail('suite-log-bytes', key.data.name)
                now = time.monotonic()
                if now >= next_sample:
                    next_sample = now + 1
                    sample = shared.sample(baseline, os.getpid())
                    samples.write(json.dumps(sample) + '\n')
                    samples.flush()
                    report['sampled_peak_rss_bytes'] = max(report['sampled_peak_rss_bytes'], sample['rss_bytes'])
                    report['latest_sample'] = sample
                    save()
                for g, child in children.items():
                    if child.poll() is not None and child.returncode != 0:
                        shared.fail('suite-failed', dict(generation=g, exit_code=child.returncode))
                if now - started > PLAN['outer_schedule_seconds']:
                    shared.fail('schedule-wall')
                if shared.stop_requested():
                    raise RuntimeError('shared failure: ' + (ROOT / 'stop.json').read_text())
        report['final_sample'] = shared.sample(baseline, os.getpid())
        require(not shared.stop_requested(), 'final shared resource verdict')
        for row in report['generations']:
            g = row['generation']
            receipt_path = shared.ROOTS[g] / f'whole-c{g}/receipt.json'
            suite = load(receipt_path)
            require(children[g].wait() == 0 and suite['status'] == 'passed' and
                    len(suite['controls']) == 2 and len(suite['rejections']) == 23, 'complete unchanged case matrix')
            row.update(status='passed', exit_code=0, suite_receipt=shared.identity(receipt_path),
                       logs={label: shared.identity(output / f'c{g}.{label}') for label in ('stdout', 'stderr')})
        require(all(shared.identity(p) == value for p, value in baseline_hashes.items()), 'initial evidence changed')
        require(all(shared.identity(p) == value for p, value in manifest.items()), 'reviewed source changed')
        require(not shared.stop_requested(), 'shared failure recorded at completion')
        report.update(status='passed', inputs_unchanged=True)
    except BaseException:
        shared.fail('coordinator-exception', traceback.format_exc())
        stop_all(children)
        report.update(status='failed', error=traceback.format_exc())
        raise
    finally:
        selector.close()
        for stream in streams:
            stream.close()
        report['children_final'] = {str(g): dict(pid=p.pid, exit_code=p.poll()) for g, p in children.items()}
        report.update(ended_ns=time.time_ns(), files={p.name: shared.identity(p) for p in output.iterdir()
                      if p.is_file() and p.name != 'receipt.json'})
        save()


def main():
    signal.signal(signal.SIGTERM, cancelled)
    with (ROOT / 'coordinator.lock').open('a+') as lease, (shared.ORIGINAL / 'whole-suite.lock').open('a+') as old:
        fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
        fcntl.flock(old, fcntl.LOCK_EX | fcntl.LOCK_NB)
        require(not shared.stop_requested() and not (ROOT / 'admission.json').exists(), 'one fresh v2 schedule only')
        execute()


if __name__ == '__main__':
    main()
