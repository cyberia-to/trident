"""Birth-bound group observation and bounded cleanup, retaining every error."""
import os
import signal
import subprocess
import time


def snapshot():
    raw = subprocess.check_output(
        ['/bin/ps', '-axo', 'pid=,ppid=,pgid=,rss=,stat=,lstart='], text=True,
        timeout=15, env=dict(os.environ, LC_ALL='C', LANG='C', TZ='UTC'))
    result = []
    for line in raw.splitlines():
        columns = line.split(None, 5)
        if len(columns) != 6:
            raise ValueError('complete process identity required')
        pid, parent, group, rss = map(int, columns[:4])
        result.append(dict(pid=pid, ppid=parent, pgid=group, rss_kib=rss,
                           state=columns[4], started=columns[5]))
    return result


def bind(pid):
    rows = [row for row in snapshot() if row['pid'] == pid]
    if len(rows) != 1:
        raise ValueError('new child birth unavailable')
    row = rows[0]
    if row['pgid'] != pid:
        raise ValueError('child must lead its own group')
    return dict(pid=pid, pgid=pid, started=row['started'],
                members={str(pid): row['started']})


def observe(record, rows=None):
    """Never adopt an unknown or reused group solely by its numeric identifier."""
    rows = snapshot() if rows is None else rows
    group = [row for row in rows if row['pgid'] == record['pgid']]
    if not group:
        return dict(status='empty', members=[])
    known = record.setdefault('members', {str(record['pid']): record['started']})
    if not any(known.get(str(row['pid'])) == row['started'] for row in group):
        return dict(status='unbound', members=group)
    for row in group:
        known[str(row['pid'])] = row['started']
    return dict(status='owned', members=group)


def stop_group(record, child=None, grace=10, final_wait=5):
    """An EPERM is retained; only an observed empty group permits success."""
    events = []

    def probe():
        if child is not None:
            child.poll()  # Reap a dead direct child before judging its group.
        result = observe(record)
        events.append(dict(time_ns=time.time_ns(), observation=result))
        return result

    def send(sig):
        result = probe()
        if result['status'] != 'owned':
            return result
        try:
            os.killpg(record['pgid'], sig)
            events.append(dict(time_ns=time.time_ns(), signal=int(sig)))
        except OSError as error:
            events.append(dict(time_ns=time.time_ns(), signal=int(sig),
                               error=repr(error), errno=error.errno))
        return result

    result = send(signal.SIGTERM)
    end = time.monotonic() + grace
    while result['status'] == 'owned' and time.monotonic() < end:
        time.sleep(0.2)
        result = probe()
    if result['status'] == 'owned':
        result = send(signal.SIGKILL)
    end = time.monotonic() + final_wait
    while result['status'] == 'owned' and time.monotonic() < end:
        time.sleep(0.2)
        result = probe()
    return dict(status='passed' if result['status'] == 'empty' else 'failed',
                binding=record, final=result, events=events)


def stop_child(child, record=None):
    """A successful Popen must never be misreported as a command not started."""
    if child is None:
        return dict(status='not-started')
    report = dict(status='failed', pid=child.pid, binding_missing=record is None, events=[])
    try:
        if record is not None:
            return stop_group(record, child)
        if child.poll() is None:
            # A live, unreaped direct child cannot have its PID reused. Still
            # require its current parent and group before acquiring a birth.
            rows = [r for r in snapshot() if r['pid'] == child.pid]
            if len(rows) != 1 or rows[0]['ppid'] != os.getpid() or rows[0]['pgid'] != child.pid:
                raise ValueError('cannot bind the existing direct child group')
            row = rows[0]
            record = dict(pid=child.pid, pgid=child.pid, started=row['started'],
                          members={str(child.pid): row['started']})
            result = stop_group(record, child)
            result['binding_recovered_from_live_direct_child'] = True
            return result
        report['direct_child_exit_code'] = child.returncode
        report['group_status'] = 'unverified-without-original-binding'
    except BaseException as error:
        report['events'].append(dict(error=repr(error)))
        # Popen owns this unreaped direct child; never infer group ownership.
        try:
            if child.poll() is None:
                child.terminate()
                report['events'].append(dict(direct_child_signal='SIGTERM'))
                try:
                    child.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    child.kill()
                    report['events'].append(dict(direct_child_signal='SIGKILL'))
                    child.wait(timeout=5)
            report['direct_child_exit_code'] = child.returncode
        except BaseException as cleanup_error:
            report['events'].append(dict(cleanup_error=repr(cleanup_error)))
    return report
