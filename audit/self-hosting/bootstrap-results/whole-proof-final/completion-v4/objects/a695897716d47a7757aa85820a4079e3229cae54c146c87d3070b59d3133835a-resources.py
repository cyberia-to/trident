"""Shared admission and fail-closed accounting for two owned diagnostic suites."""
import contextlib
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import time

import ownership

ROOT = Path(__file__).resolve().parent
BASE = ROOT.parent
PLAN = json.loads((ROOT / 'plan.json').read_text())
ROOTS = {int(g): BASE / p for g, p in PLAN['roots'].items()}
ORIGINAL = BASE / PLAN['original_scope']
RETAINED_ROOTS = tuple(BASE / name for name in PLAN['retained_scopes'])


def identity(path):
    path = Path(path)
    if not stat.S_ISREG(path.lstat().st_mode):
        raise ValueError('regular file required: ' + str(path))
    with path.open('rb') as stream:
        digest = hashlib.file_digest(stream, 'sha256').hexdigest()
    return dict(bytes=path.stat().st_size, sha256=digest)


def write(path, value):
    path = Path(path)
    temporary = path.with_name(path.name + '.pending-' + str(os.getpid()))
    with temporary.open('x') as stream:
        stream.write(json.dumps(value, indent=2) + '\n')
        stream.flush()
        os.fsync(stream.fileno())
    os.replace(temporary, path)
    directory = os.open(path.parent, os.O_RDONLY)
    try:
        os.fsync(directory)
    finally:
        os.close(directory)


@contextlib.contextmanager
def accounting(exclusive=False):
    with (ROOT / 'accounting.lock').open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX if exclusive else fcntl.LOCK_SH)
        yield


def files(scope):
    for directory, dirs, names in os.walk(scope):
        dirs[:] = [d for d in dirs if d != 'target-helper']
        for d in dirs:
            if (Path(directory) / d).is_symlink():
                raise ValueError('symlink directory in owned scope')
        for name in names:
            path = Path(directory) / name
            try:
                mode = path.lstat().st_mode
            except FileNotFoundError:
                if '.pending-' in name:
                    continue
                raise
            if not stat.S_ISREG(mode):
                raise ValueError('nonregular owned file: ' + str(path))
            yield path


def inventory():
    with accounting():
        result = {}
        for root in (*RETAINED_ROOTS, *ROOTS.values(), ROOT):
            for path in files(root):
                try:
                    result[str(path)] = path.stat().st_size
                except FileNotFoundError:
                    if '.pending-' not in path.name:
                        raise
        return result


def stop_requested():
    return (ROOT / 'stop.json').exists()


def fail(reason, detail=None):
    # Exclusive creation preserves the first actual failure across both suites.
    try:
        with (ROOT / 'stop.json').open('x') as stream:
            json.dump(dict(reason=reason, detail=detail, pid=os.getpid(), time_ns=time.time_ns()), stream)
    except FileExistsError:
        pass


def admitted(scope):
    scope = Path(scope).resolve()
    deadline = time.monotonic() + 30
    while not (ROOT / 'admission.json').exists():
        if stop_requested() or time.monotonic() > deadline:
            raise ValueError('shared admission unavailable')
        time.sleep(0.1)
    admission = json.loads((ROOT / 'admission.json').read_text())
    matches = [g for g, p in ROOTS.items() if p == scope]
    if len(matches) != 1:
        raise ValueError('unknown generation scope')
    generation = matches[0]
    if admission['pid'] != os.getppid() or admission['children'][str(generation)] != os.getpid():
        raise ValueError('suite must be the registered direct coordinator child')
    if stop_requested():
        raise ValueError('shared stop already recorded')
    return generation


def reclaim(path):
    path = Path(path).resolve()
    with accounting(exclusive=True):
        if stop_requested():
            raise ValueError('retain payload after shared stop')
        matches = [(g, root) for g, root in ROOTS.items() if path.parent == root / f'whole-c{g}']
        if len(matches) != 1 or admitted(matches[0][1]) != matches[0][0]:
            raise ValueError('reclaim only the caller generation mutation')
        if not path.name.startswith('certificate-') or path.suffix != '.joysc':
            raise ValueError('reclaim only a checked certificate mutation')
        path.unlink()


def wait_full_admission(scope):
    admitted(scope)
    end = time.monotonic() + PLAN['outer_schedule_seconds']
    while not (ROOT / 'full-admission.json').exists():
        if stop_requested() or time.monotonic() >= end:
            raise ValueError('full admission unavailable')
        time.sleep(0.2)
    record = json.loads((ROOT / 'full-admission.json').read_text())
    if record['admission'] != identity(ROOT / 'admission.json'):
        raise ValueError('full admission belongs to a different coordinator')


def authorize_native(generation, attempt, argv, kind=None):
    if stop_requested():
        raise ValueError('shared stop before native command')
    if (ROOT / 'full-admission.json').exists():
        full = json.loads((ROOT / 'full-admission.json').read_text())
        if full['admission'] != identity(ROOT / 'admission.json'):
            raise ValueError('full admission identity mismatch')
        return
    if (generation != 2 or attempt != 'whole-c2-verify-cost' or
            list(map(str, argv)) != PLAN['prelude_command'] or kind not in (None, 'verify')):
        raise ValueError('restricted prelude permits only exact C2 cost verification')


def register_native(attempt, argv):
    admission = json.loads((ROOT / 'admission.json').read_text())
    matches = [int(g) for g, pid in admission['children'].items() if pid == os.getppid()]
    if len(matches) != 1 or os.getpgrp() != os.getpid():
        raise ValueError('native command must belong to an admitted suite in its own session')
    authorize_native(matches[0], attempt, argv)
    record = ownership.bind(os.getpid())
    record.update(parent=os.getppid(), argv=argv, attempt=attempt,
                  coordinator=admission['pid'], registered_ns=time.time_ns())
    directory = ROOT / 'native-processes'
    directory.mkdir(exist_ok=True)
    path = directory / (attempt + '.json')
    with accounting(exclusive=True):
        authorize_native(matches[0], attempt, argv)
        if path.exists():
            raise ValueError('native registration already exists')
        write(path, record)


def process_snapshot(root_pid):
    with accounting(exclusive=True):
        all_rows = ownership.snapshot()
        selected = {root_pid}
        while True:
            larger = selected | {r['pid'] for r in all_rows if r['ppid'] in selected}
            if larger == selected:
                break
            selected = larger
        for path in sorted((ROOT / 'native-processes').glob('*.json')):
            retired = path.with_suffix('.retired')
            if retired.exists():
                continue
            record = json.loads(path.read_text())
            if record['coordinator'] != root_pid:
                raise ValueError('foreign coordinator process registration')
            observed_path = path.with_suffix('.observed')
            if observed_path.exists():
                observed = json.loads(observed_path.read_text())
                if any(observed[k] != record[k] for k in ('pid', 'pgid', 'started')):
                    raise ValueError('process membership binding changed')
                record = observed
            result = ownership.observe(record, all_rows)
            write(observed_path, record)
            if result['status'] == 'empty':
                write(retired, dict(time_ns=time.time_ns(), original=identity(path),
                                    final_observation=result))
            elif result['status'] == 'unbound':
                fail('native-process-identity-changed', dict(record=record, observation=result))
            else:
                selected.update(row['pid'] for row in result['members'])
        return [r for r in all_rows if r['pid'] in selected]


def sample(baseline, coordinator_pid):
    sizes = inventory()
    generations = {}
    reason = None
    for g, root in ROOTS.items():
        work = root / f'whole-c{g}'
        dynamic = {Path(p): n for p, n in sizes.items() if Path(p).is_relative_to(root) and p not in baseline}
        mutants = {str(p): n for p, n in dynamic.items() if p.parent == work and p.suffix == '.joysc'}
        dags = {str(p): n for p, n in dynamic.items() if p.parent == work and p.suffix == '.dag'}
        metadata = sum(dynamic.values()) - sum(mutants.values()) - sum(dags.values())
        ceiling = PLAN['proofs'][str(g)]['bytes'] + PLAN['mutant_growth_ceiling_bytes']
        generations[str(g)] = dict(mutants=mutants, canonical_output_bytes=sum(dags.values()), metadata_bytes=metadata)
        if len(mutants) > 1 or any(n > ceiling for n in mutants.values()):
            reason = reason or 'generation-mutant-reservation'
        if sum(dags.values()) > PLAN['canonical_and_output_bucket_per_generation']:
            reason = reason or 'generation-output-reservation'
        for p, n in dags.items():
            limit = 32 * 1024**2 if Path(p).name == 'rechain.dag' else 16 * 1024**2
            if n > limit:
                reason = reason or 'individual-canonical-output'
        if metadata > PLAN['metadata_bucket_per_generation']:
            reason = reason or 'generation-metadata-reservation'
    shared_metadata = sum(n for p, n in sizes.items() if Path(p).is_relative_to(ROOT) and (p not in baseline or Path(p).is_relative_to(ROOT / 'run-1')))
    if shared_metadata > PLAN['metadata_bucket_per_generation']:
        reason = reason or 'coordinator-metadata-reservation'
    processes = process_snapshot(coordinator_pid)
    rss = sum(p['rss_kib'] * 1024 for p in processes)
    free = shutil.disk_usage(ROOT).free
    total = sum(sizes.values())
    if total > PLAN['shared_disk_bytes']:
        reason = reason or 'shared-owned-disk'
    if free < PLAN['free_floor_bytes']:
        reason = reason or 'shared-free-disk'
    if rss > PLAN['combined_sampled_rss_bytes']:
        reason = reason or 'shared-rss'
    result = dict(time_ns=time.time_ns(), owned_bytes=total, free_bytes=free,
                  rss_bytes=rss, processes=processes, generations=generations,
                  coordinator_metadata_bytes=shared_metadata, reason=reason)
    if reason:
        fail(reason, result)
    return result
