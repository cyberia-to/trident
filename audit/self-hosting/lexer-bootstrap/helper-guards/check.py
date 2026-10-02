"""Verify audit helpers accept normal Python and reject disabled assertions."""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

if sys.flags.optimize:
    raise RuntimeError('guard checks require unoptimized Python')
A = Path(__file__).resolve().parent
REPO = A.parents[3]
HELPERS = ('collect.py', 'c2-corpus/verify.py', 'c3-corpus/verify.py')


def sha(data):
    return hashlib.sha256(data).hexdigest()


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def unchanged_inputs():
    return {p.relative_to(A.parent).as_posix(): sha(p.read_bytes())
            for p in A.parent.rglob('*') if p.is_file() and A not in p.parents}


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', required=True, type=Path)
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=False)
before = unchanged_inputs()
rows = []
for helper in HELPERS:
    path = A.parent / helper
    for mode in ('normal', 'flag-O', 'environment-OO'):
        env = dict(os.environ, PYTHONDONTWRITEBYTECODE='1')
        env.pop('PYTHONOPTIMIZE', None)
        if mode == 'environment-OO':
            env['PYTHONOPTIMIZE'] = '2'
        argv = [sys.executable] + (['-O'] if mode == 'flag-O' else []) + [str(path)]
        result = subprocess.run(argv, cwd=REPO, env=env, capture_output=True, timeout=60)
        row = dict(helper=helper, mode=mode, command=argv, cwd=str(REPO),
                   environment={'PYTHONDONTWRITEBYTECODE': '1',
                                'PYTHONOPTIMIZE': env.get('PYTHONOPTIMIZE')},
                   exit_code=result.returncode, helper_sha256=sha(path.read_bytes()))
        for label, raw in (('stdout', result.stdout), ('stderr', result.stderr)):
            name = f'{len(rows)}.{label}.gz'
            stored = gzip.compress(raw, mtime=0)
            (args.output / name).write_bytes(stored)
            row[label] = dict(path=name, bytes=len(raw), sha256=sha(raw),
                              compressed_sha256=sha(stored), compressed_bytes=len(stored))
        if mode == 'normal':
            row['passed'] = result.returncode == 0 and json.loads(result.stdout)['status'] == 'passed'
        else:
            row['passed'] = (result.returncode != 0 and result.stdout == b'' and
                            b'archive verification requires unoptimized Python' in result.stderr)
        rows.append(row)
after = unchanged_inputs()
report = dict(schema='trident/archive-helper-guards/v1',
    status='passed' if before == after and all(row['passed'] for row in rows) else 'failed',
    base_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=REPO, text=True).strip(),
    checker_sha256=sha(Path(__file__).read_bytes()), commands=rows,
    all_retained_inputs_byte_identical=before == after, retained_input_sha256=before,
    scope='Audit helper invocation modes only; no compiler execution or new bootstrap acceptance')
(args.output / 'receipt.json').write_text(json.dumps(report, indent=2) + '\n')
require(report['status'] == 'passed', 'helper invocation boundary failed; see receipt')
print(json.dumps(dict(status='passed', checks=len(rows), retained_inputs_unchanged=True)))
