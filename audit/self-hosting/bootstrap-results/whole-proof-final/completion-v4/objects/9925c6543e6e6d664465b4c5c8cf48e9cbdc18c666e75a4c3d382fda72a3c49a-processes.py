"""Two later birth-aware observations; no signals and no old status rewrite."""
import calendar
import datetime
import hashlib
import os
from pathlib import Path
import re
import subprocess
import time

from safe_files import durable_bytes, require

MARGIN = 2_000_000_000
MONTHS = {m: i for i, m in enumerate(
    ('Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'), 1)}


def birth_ns(text):
    match = re.fullmatch(r'(Mon|Tue|Wed|Thu|Fri|Sat|Sun) ([A-Z][a-z]{2}) +([0-9]{1,2}) '
                         r'([0-9]{2}):([0-9]{2}):([0-9]{2}) ([0-9]{4})', text)
    require(match is not None, 'invalid birth timestamp')
    week, month, day, hour, minute, second, year = match.groups()
    require(month in MONTHS, 'unknown month')
    d = datetime.datetime(int(year), MONTHS[month], int(day), int(hour), int(minute),
                          int(second), tzinfo=datetime.timezone.utc)
    require(('Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun')[d.weekday()] == week,
            'inconsistent birth weekday')
    return calendar.timegm(d.utctimetuple()) * 1_000_000_000


def classify(raw, owners, cutoff_ns, observed_ns):
    require(type(cutoff_ns) is int and 0 < cutoff_ns <= observed_ns, 'invalid quiescence cutoff')
    require(owners and all(type(p) is int and p > 0 for p in owners), 'invalid owner identity set')
    seen, reused = set(), []
    for line in raw.decode('ascii', errors='strict').splitlines():
        p = line.strip().split(maxsplit=3)
        require(len(p) == 4 and all(x.isascii() and x.isdecimal() for x in p[:3]),
                'malformed complete process snapshot')
        pid, ppid, pgid = map(int, p[:3])
        require(pid > 0 and pid not in seen, 'duplicate/invalid PID')
        seen.add(pid)
        born = birth_ns(p[3])
        require(born <= observed_ns, 'future process birth')
        if {pid, ppid, pgid} & set(owners):
            require(born > cutoff_ns + MARGIN, 'live old owner/descendant or ambiguous reused identity')
            reused.append({'pid': pid, 'ppid': ppid, 'pgid': pgid, 'birth_ns': born})
    require(seen, 'empty process snapshot')
    return {'no_live_prior_owner': True, 'snapshot_rows': len(seen), 'reused': reused,
            'cutoff_ns': cutoff_ns, 'observed_ns': observed_ns, 'rounding_margin_ns': MARGIN}


def observe(private, label, owners, cutoff_ns):
    command = ['/bin/ps', '-axo', 'pid=,ppid=,pgid=,lstart=']
    started = time.time_ns()
    r = subprocess.run(command, capture_output=True, timeout=15,
                       env=dict(os.environ, LC_ALL='C', LANG='C', TZ='UTC'))
    ended = time.time_ns()
    refs = {}
    for suffix, data in [('stdout', r.stdout), ('stderr', r.stderr)]:
        path = Path(private) / (label + '.' + suffix)
        durable_bytes(path, data)
        refs[suffix] = {'local_only_path': str(path), 'bytes': len(data),
                        'sha256': hashlib.sha256(data).hexdigest()}
    require(r.returncode == 0, 'fresh process inventory command failed')
    result = classify(r.stdout, owners, cutoff_ns, ended)
    return dict(result, started_ns=started, command=command, returncode=r.returncode,
                raw_private=refs, signals_sent=[])
