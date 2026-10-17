"""Read-only replay of historical quiescence and stable, no-follow file hashes."""
import hashlib
import os
from pathlib import Path
import stat
from prior_primitives_v4 import require, identity, load, same

QUIESCENCE_SHA = '7561060564c3591aafab1b431cf657cf01e984abe2aa62cebee28686cbc1b13a'


def reference(path):
    return dict(path=str(path), **identity(path))


def snapshot_absent(path, groups, processes):
    rows = path.read_text().splitlines()
    require(bool(rows), 'nonempty complete process snapshot')
    pids = set()
    for line in rows:
        fields = line.split(None, 3)
        require(len(fields) == 4 and all(value.isascii() and value.isdigit() for value in fields[:3]),
                'actual process snapshot row')
        pid, parent, group = map(int, fields[:3])
        require(pid > 0 and parent >= 0 and group >= 0 and pid not in pids, 'unique process snapshot identity')
        pids.add(pid)
        require(group not in groups and pid not in processes, 'historical owned process remains live')


def review_quiescence(base):
    root = base / 'whole-v3-emergency-quiescence'
    shared = base / 'whole-proof-attacks-completion-v3'
    receipt = root / 'receipt.json'
    require(identity(receipt)['sha256'] == QUIESCENCE_SHA, 'exact historical quiescence receipt')
    q, first = load(receipt), load(root / 'first.json')
    same(root / 'first.json', q['first'])
    require(q['schema'] == 'trident/v3-post-failure-quiescence/v1'
            and first['schema'] == 'trident/v3-post-failure-process-observation/v1'
            and q['status'] == first['status'] == 'observed-quiescent', 'later observation only')
    run = load(shared / 'run-1/receipt.json')
    admission = load(shared / 'admission.json')
    require(run['status'] == 'failed' and run.get('cleanup_error')
            and all(row['exit_code'] is None for row in run['children_final'].values()),
            'original failed shutdown remains failed and incomplete')
    require(run['ended_ns'] < first['started_ns'] <= first['observed_ns'] < q['second_observed_ns'],
            'two later ordered observations')
    require(first['command'] == ['/bin/ps', '-axo', 'pid=,ppid=,pgid=,lstart=,stat=,comm='],
            'actual complete process observation command')
    registered = sorted((shared / 'native-processes').glob('*.json'))
    require(set(first['source_registrations']) == {str(p) for p in registered}, 'complete original registrations')
    groups = set()
    for path in registered:
        same(path, first['source_registrations'][str(path)])
        same(root / 'original-registrations' / path.name, first['source_registrations'][str(path)])
        row = load(path)
        require(type(row['pid']) is int and row['pid'] > 0 and row['pid'] == row['pgid']
                and row['coordinator'] == admission['pid']
                and row['parent'] in admission['children'].values(), 'registered original native ownership')
        groups.add(row['pgid'])
    processes = {admission['pid'], *admission['children'].values()}
    require(q['registration_count'] == len(registered) and q['registered_pgids'] == sorted(groups)
            and set(q['coordinator_and_suites']) == processes, 'complete historical ownership set')
    require(q['matching_current_processes'] == q['signals_sent'] == []
            and first['coordinator_suite_rows'] == first['signals'] == []
            and q['partials_deleted'] is False and first['partials_deleted'] is False
            and first['originals_modified'] is False, 'read-only observations cannot imply cleanup')
    paths = {'ps-first.stdout': first['ps'], 'ps-first.stderr': first['stderr'],
             'ps-second.stdout': q['second_ps'], 'ps-second.stderr': q['second_stderr']}
    for name, expected in paths.items():
        same(root / name, expected)
        if name.endswith('.stdout'):
            snapshot_absent(root / name, groups, processes)
        else:
            require(expected['bytes'] == 0, 'process observation succeeded without stderr')
    expected_originals = {str(shared / p) for p in ('run-1/receipt.json', 'stop.json', 'admission.json')}
    require(set(q['original_failed_evidence_unchanged']) == expected_originals, 'exact original failed evidence set')
    for path, expected in q['original_failed_evidence_unchanged'].items():
        same(Path(path), expected)
    return dict(receipt=reference(receipt), first=reference(root / 'first.json'),
                process_snapshots={name: dict(path=str(root/name), **value) for name, value in paths.items()},
                claim='Later historical absence observations; original failed shutdown is not relabelled.')


def file_state(value):
    return dict(device=value.st_dev, inode=value.st_ino, bytes=value.st_size,
                mtime_ns=value.st_mtime_ns, ctime_ns=value.st_ctime_ns,
                birthtime_seconds=getattr(value, 'st_birthtime', None),
                birthtime_ns=getattr(value, 'st_birthtime_ns',
                                    int(value.st_birthtime*10**9) if hasattr(value, 'st_birthtime') else None),
                links=value.st_nlink,
                mode=value.st_mode)


def stable_hash(path, expected):
    """Keep one no-follow inode open through complete bounded streaming readback."""
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1, 'owned regular single-link file')
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    try:
        state = file_state(before)
        require(file_state(os.fstat(fd)) == state, 'opened file is exactly inspected inode')
        require(before.st_size == expected['bytes'], 'retained complete byte count')
        digest, count = hashlib.sha256(), 0
        while block := os.read(fd, min(8*1024**2, expected['bytes'] - count + 1)):
            count += len(block)
            require(count <= expected['bytes'], 'retained file cannot grow beyond exact byte count')
            digest.update(block)
        require(dict(bytes=count, sha256=digest.hexdigest()) == expected, 'retained complete file hash')
        require(file_state(os.fstat(fd)) == state == file_state(path.lstat()), 'retained inode stable throughout full read')
        return dict(path=str(path), **expected, stat=state)
    finally:
        os.close(fd)


def inspect_birth_rows(raw, groups, processes, cutoff_ns):
    """A reused numerical ID is safe only when every overlap was born later."""
    from datetime import datetime, timezone
    seen, overlap = set(), []
    require(bool(raw.strip()), 'nonempty current process snapshot')
    for line in raw.splitlines():
        fields = line.split(None, 2)
        require(len(fields) == 3 and all(s.isascii() and s.isdigit() for s in fields[:2]),
                'current PID/PGID/birth row')
        pid, group = map(int, fields[:2])
        require(pid > 0 and group >= 0 and pid not in seen, 'unique current process identity')
        seen.add(pid)
        if pid in processes or group in groups:
            born = datetime.strptime(fields[2], '%a %b %d %H:%M:%S %Y').replace(tzinfo=timezone.utc)
            born_ns = int(born.timestamp()) * 10**9
            require(born_ns > cutoff_ns + 2*10**9, 'original or ambiguous process still owns historical identity')
            overlap.append(dict(pid=pid, pgid=group, birth_utc=fields[2], birth_ns=born_ns,
                                classification='reused-after-observed-absence'))
    return overlap


def observe_current(base):
    import subprocess
    import time
    shared = base / 'whole-proof-attacks-completion-v3'
    admission = load(shared/'admission.json')
    registered = [load(p) for p in (shared/'native-processes').glob('*.json')]
    groups = {row['pgid'] for row in registered}
    processes = {admission['pid'], *admission['children'].values(), *(row['pid'] for row in registered)}
    cutoff = load(base/'whole-v3-emergency-quiescence/receipt.json')['second_observed_ns']
    command = ['/bin/ps', '-axo', 'pid=,pgid=,lstart=']
    environment = dict(os.environ, TZ='UTC', LC_ALL='C')
    started = time.time_ns()
    result = subprocess.run(command, env=environment, capture_output=True, timeout=10, check=True)
    ended = time.time_ns()
    require(len(result.stdout) <= 4*1024**2 and result.stderr == b'', 'bounded clean current process observation')
    raw = result.stdout.decode('ascii')
    overlap = inspect_birth_rows(raw, groups, processes, cutoff)
    return dict(command=command, environment_overrides={'TZ':'UTC','LC_ALL':'C'}, started_ns=started,
                ended_ns=ended, exit_code=result.returncode, deadline_seconds=10,
                cutoff_ns=cutoff, overlaps=overlap, raw_stdout=raw,
                stdout=dict(bytes=len(result.stdout),sha256=hashlib.sha256(result.stdout).hexdigest()))
