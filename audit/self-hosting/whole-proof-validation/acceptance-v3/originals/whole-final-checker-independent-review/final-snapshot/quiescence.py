"""Independently classify the recorded UTC process snapshot after empty shutdown."""
import datetime
import re
from common import require, identity, load, same


def birth(value):
    require(re.fullmatch(r'[A-Z][a-z]{2} [A-Z][a-z]{2} +[0-9]{1,2} [0-9]{2}:[0-9]{2}:[0-9]{2} [0-9]{4}',value)
            is not None, 'exact C-locale process birth grammar')
    parts = value.split()
    require(len(parts) == 5, 'process birth shape')
    weekday, month, day, clock, year = parts
    months = ('Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec')
    require(month in months and weekday in ('Mon','Tue','Wed','Thu','Fri','Sat','Sun'), 'English UTC process birth')
    hours = clock.split(':')
    require(len(hours) == 3 and all(s.isascii() and s.isdecimal() for s in [day,year,*hours]), 'numeric process birth')
    point = datetime.datetime(int(year),months.index(month)+1,int(day),*map(int,hours),tzinfo=datetime.timezone.utc)
    require(('Mon','Tue','Wed','Thu','Fri','Sat','Sun')[point.weekday()] == weekday, 'birth weekday consistent')
    delta = point-datetime.datetime(1970,1,1,tzinfo=datetime.timezone.utc)
    return (delta.days*86400+delta.seconds)*1_000_000_000


def classify(raw, groups, shutdown, observed):
    require(type(shutdown) is int and shutdown > 0 and type(observed) is int and observed >= shutdown,
            'ordered exact shutdown/observation times')
    require(groups and all(type(g) is int and g > 0 for g in groups), 'positive prior owner identifiers')
    seen, overlaps = set(), []
    for line in raw.decode('ascii').splitlines():
        parts = line.strip().split(maxsplit=2)
        require(len(parts) == 3 and all(p.isascii() and p.isdecimal() for p in parts[:2]), 'complete raw process row')
        pid, pgid = map(int,parts[:2])
        require(pid not in seen, 'unique process PID')
        seen.add(pid)
        started = birth(parts[2])
        if pid in groups or pgid in groups:
            require(shutdown + 2_000_000_000 < started <= observed, 'prior or ambiguous process birth refuses reclamation')
            overlaps.append(dict(pid=pid,pgid=pgid,birth_utc=parts[2],birth_ns=started,raw=line))
    require(bool(seen), 'nonempty process inventory')
    return dict(snapshot_rows=len(seen), overlapping_rows=overlaps, reused_rows=overlaps,
                rounding_margin_ns=2_000_000_000, shutdown_ns=shutdown, observed_ns=observed,
                no_live_prior_owners=True)


def review(base, shared, recorded):
    old = base/'whole-proof-attacks-parallel-v2'
    admission, shutdown = load(old/'admission.json'),load(old/'shutdown.json')
    require(shutdown['all_owned_groups_empty'] is True and shutdown['remaining'] == [], 'recorded empty-group shutdown')
    groups = {admission['pid'],*admission['children'].values()}
    owner_files = {str(old/name):identity(old/name) for name in ('admission.json','shutdown.json')}
    for path in (old/'native-processes').glob('*.json'):
        native, retired = load(path),path.with_suffix('.retired')
        require(native['coordinator'] == admission['pid'] and 0 < int(retired.read_text().strip()) <= shutdown['time_ns'],
                'original native group retired before empty shutdown')
        groups.add(native['pgid'])
        owner_files.update({str(p):identity(p) for p in (path,retired)})
    require(recorded['original_owner_records'] == owner_files, 'complete original process ownership evidence')
    directory = shared/'reclamation-action-v2'
    require((directory/'processes.stdout').stat().st_size <= 4*1024**2
            and (directory/'processes.stderr').stat().st_size == 0, 'bounded successful process inventory')
    same(directory/'processes.stdout',recorded['stdout'])
    same(directory/'processes.stderr',recorded['stderr'])
    expected = dict(command=['/bin/ps','-axo','pid=,pgid=,lstart='],
                    environment={'TZ':'UTC','LC_ALL':'C','LANG':'C'}, checked_groups=sorted(groups),
                    original_owner_records=owner_files, stdout=identity(directory/'processes.stdout'),
                    stderr=identity(directory/'processes.stderr'),
                    **classify((directory/'processes.stdout').read_bytes(),groups,shutdown['time_ns'],recorded['observed_ns']))
    require(recorded == expected, 'independently replayed timestamp-aware process classification')
    return expected
