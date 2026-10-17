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
    Path(path).write_text(json.dumps(value, indent=2) + '\n')


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
            if not stat.S_ISREG(path.lstat().st_mode):
                raise ValueError('nonregular owned file: ' + str(path))
            yield path


def inventory():
    with accounting():
        return {str(p): p.stat().st_size for root in (*RETAINED_ROOTS, *ROOTS.values(), ROOT)
                for p in files(root)}


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


def start_identity(pid):
    result = subprocess.run(['/bin/ps', '-p', str(pid), '-o', 'lstart='],
                            capture_output=True, text=True)
    return result.stdout.strip() if result.returncode == 0 else None


def register_native(attempt, argv):
    # Runs inside the new session before exec. A parent crash after Popen cannot
    # leave an executing unregistered native child outside coordinator accounting.
    admission = json.loads((ROOT / 'admission.json').read_text())
    if os.getppid() not in admission['children'].values() or os.getpgrp() != os.getpid():
        raise ValueError('native command must belong to an admitted suite in its own session')
    directory = ROOT / 'native-processes'
    directory.mkdir(exist_ok=True)
    record = dict(pid=os.getpid(), pgid=os.getpgrp(), parent=os.getppid(),
                  started=start_identity(os.getpid()), argv=argv, attempt=attempt,
                  coordinator=admission['pid'], registered_ns=time.time_ns())
    if record['started'] is None:
        raise ValueError('cannot bind native process birth')
    path = directory / (attempt + '.json')
    with accounting(exclusive=True):
        with path.open('x') as stream:
            json.dump(record, stream)
            stream.flush()
            os.fsync(stream.fileno())


def process_snapshot(root_pid):
    with accounting(exclusive=True):
        rows = subprocess.check_output(['/bin/ps', '-axo', 'pid=,ppid=,pgid=,rss='], text=True)
        all_rows = [dict(zip(('pid', 'ppid', 'pgid', 'rss_kib'), map(int, row.split())))
                    for row in rows.splitlines()]
        selected = {root_pid}
        while True:
            larger = selected | {r['pid'] for r in all_rows if r['ppid'] in selected}
            if larger == selected:
                break
            selected = larger
        # Registrations outlive suite parents. A group remains owned until observed
        # empty, even if its leader exits and descendants become reparented.
        for path in sorted((ROOT / 'native-processes').glob('*.json')):
            retired = path.with_suffix('.retired')
            if retired.exists():
                continue
            record = json.loads(path.read_text())
            if record['coordinator'] != root_pid:
                raise ValueError('foreign coordinator process registration')
            group = [r for r in all_rows if r['pgid'] == record['pgid']]
            if not group:
                retired.write_text(str(time.time_ns()) + '\n')
                continue
            leader = next((r for r in group if r['pid'] == record['pid']), None)
            birth = start_identity(record['pid']) if leader else None
            if leader and birth != record['started']:
                # The old group disappeared between samples. Do not signal or
                # charge an unrelated reused PID; retain the anomalous evidence.
                if birth is not None:
                    fail('native-process-identity-changed', record)
                continue
            selected.update(r['pid'] for r in group)
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
