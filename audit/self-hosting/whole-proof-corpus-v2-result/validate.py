#!/usr/bin/env python3
"""Retain exact validation commands and source impact for this audit-only unit."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[2]
SCOPE = str(ROOT.relative_to(REPO)) + '/'
BASE = '8cb43f652501bf7689211bae793ac1757eb125b7'
PYTHON = str(Path(sys.executable).resolve())


def identity(data):
    return {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}


def run(name, argv):
    start = time.time_ns()
    completed = subprocess.run(argv, cwd=REPO, capture_output=True, timeout=120)
    for channel in ('stdout', 'stderr'):
        with (ROOT / f'validation-{name}.{channel}').open('xb') as stream:
            stream.write(getattr(completed, channel))
    return {'name': name, 'argv': argv, 'cwd': str(REPO), 'started_ns': start,
            'ended_ns': time.time_ns(), 'exit_code': completed.returncode,
            'stdout': identity(completed.stdout), 'stderr': identity(completed.stderr)}


sources = {p.name: identity(p.read_bytes()) for p in sorted(ROOT.iterdir())
           if p.suffix == '.py' or p.name == 'README.md'}
commands = [
    ('tests', [PYTHON, '-B', '-W', 'error', '-m', 'unittest', 'discover', '-s', str(ROOT), '-p', 'test_delivery.py', '-v']),
    ('replay', [PYTHON, '-B', '-W', 'error', str(ROOT / 'check_delivery.py'), '--originals']),
    ('prior-failure', [PYTHON, '-B', '-W', 'error', str(ROOT.parent / 'whole-proof-validation/corpus-v2/check_delivery.py')]),
    ('head', ['git', 'rev-parse', 'HEAD']),
    ('tracked-diff', ['git', 'diff', '--exit-code', 'HEAD', '--']),
    ('whitespace', ['git', 'diff', '--check']),
    ('status', ['git', 'status', '--porcelain=v1', '--untracked-files=all']),
]
repack = """import json, sys, tempfile
from pathlib import Path
sys.path.insert(0, sys.argv[1])
import package
root = Path(sys.argv[1])
old = json.loads((root / 'archive.json').read_bytes())
source = Path(old['complete_original_scope']).parent
with tempfile.TemporaryDirectory(prefix='repack-', dir=root) as temporary:
    new = package.package(source, Path(temporary))
    if new != old:
        raise ValueError('deterministic package mismatch')
    print(json.dumps({'status': 'passed', 'archive': new['archive'], 'files': new['files']}))
"""
commands.append(('repack', [PYTHON, '-B', '-W', 'error', '-c', repack, str(ROOT)]))
results = [run(name, argv) for name, argv in commands]
head = (ROOT / 'validation-head.stdout').read_text().strip()
status = (ROOT / 'validation-status.stdout').read_text().splitlines()
unchanged = sources == {name: identity((ROOT / name).read_bytes()) for name in sources}
passed = all(row['exit_code'] == 0 for row in results) and head == BASE and unchanged
passed = passed and bool(status) and all(row.startswith('?? ' + SCOPE) for row in status)
receipt = {
    'schema': 'trident/whole-proof-corpus-delivery-validation/v1',
    'status': 'passed' if passed else 'failed',
    'repository_base': head, 'expected_base': BASE,
    'python': {'executable': PYTHON, 'version': sys.version, 'identity': identity(Path(PYTHON).read_bytes())},
    'commands': results, 'sources': sources, 'sources_unchanged': unchanged,
    'source_impact': 'Only the new audit/self-hosting/whole-proof-corpus-v2-result/ directory. No tracked source changed.',
    'runtime_gate_scope': 'No new Rust/runtime gate is claimed by this audit-only evidence delivery.',
    'preserved_development_failures': ['check-first.stderr', 'check-second.stderr', 'check-third.stderr', 'tests.stderr', 'replay.stderr'],
    'packaging_command': ['python3', '-B', '-W', 'error', SCOPE + 'package.py',
        str(Path(json.loads((ROOT / 'archive.json').read_text())['complete_original_scope']).parent), SCOPE.rstrip('/')],
    'archive': json.loads((ROOT / 'archive.json').read_bytes()),
}
with (ROOT / 'validation.json').open('x') as stream:
    stream.write(json.dumps(receipt, indent=2, sort_keys=True) + '\n')
print(json.dumps({'status': receipt['status'], 'commands': len(results), 'sources': len(sources), 'base': head}))
raise SystemExit(0 if passed else 1)
