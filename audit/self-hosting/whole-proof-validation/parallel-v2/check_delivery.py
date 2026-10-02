"""Verify exact archived v2 tools and their actual scoped validation receipts."""
import ast
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[3]


def identity(path):
    data = path.read_bytes()
    return dict(bytes=len(data), sha256=hashlib.sha256(data).hexdigest())


def load(path):
    return json.loads(path.read_text())


manifest = load(ROOT / 'files.json')
by_original = {}
for name, expected in manifest.items():
    path = ROOT / name
    assert identity(path) == {key: expected[key] for key in ('bytes', 'sha256')}, name
    by_original[expected['original']] = path
    if path.suffix == '.py':
        ast.parse(path.read_text())
shared = ROOT / 'retained/whole-proof-attacks-parallel-v2'
review = load(shared / 'independent-review.json')
assert review['status'] == 'passed-source-review'
assert review['sources'] == load(shared / 'sources.json')
for original, expected in review['sources'].items():
    assert identity(by_original[original]) == expected
unit = ROOT / 'retained/whole-tooling-delivery-review'
unit_receipt = load(unit / 'parallel-v2-test-receipt.json')
assert unit_receipt['status'] == 'passed' and unit_receipt['tests'] == 8
for name, expected in unit_receipt['files'].items():
    assert identity(unit / name) == expected
assert 'Ran 8 tests' in (unit / 'parallel-v2-tests-5.stderr').read_text()
assert (unit / 'parallel-v2-tests-5.stderr').read_text().endswith('\nOK\n')
impact = load(ROOT / 'delivery/test-inheritance.json')
trees = [subprocess.check_output(['git', 'ls-tree', '-rz', '--full-tree', revision, '--', *impact['paths']], cwd=REPO)
         for revision in (impact['tested_base'], impact['delivery_base'])]
assert trees[0] == trees[1]
assert len(trees[0]) == impact['tree_bytes']
assert hashlib.sha256(trees[0]).hexdigest() == impact['tree_sha256']
subprocess.run(['git', 'diff', '--exit-code', impact['delivery_base'], '--', *impact['paths']], cwd=REPO, check=True)
previous = ROOT / 'delivery/inherited-rust-workspace'
assert identity(previous / 'receipt.json') == impact['rust_test_receipt']
for directory in (previous, ROOT / 'delivery/cargo-check'):
    receipt = load(directory / 'receipt.json')
    assert receipt['status'] == 'passed' and receipt['exit_code'] == 0 and not receipt['warning_lines']
    for name, expected in receipt['files'].items():
        assert identity(directory / name) == expected
counts = [tuple(map(int, row)) for row in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', (previous / 'stdout').read_text())]
assert tuple(sum(row[i] for row in counts) for i in range(3)) == (1231, 0, 5)
for document in (ROOT / 'README.md', ROOT.parent / 'README.md'):
    for target in re.findall(r'\[[^\]]*\]\(([^)]+)\)', document.read_text()):
        if '://' not in target and not target.startswith('#'):
            assert (document.parent / target.split('#')[0]).exists(), target
subprocess.run(['git', 'diff', '--check'], cwd=REPO, check=True)
print(json.dumps(dict(status='passed', archived_files=len(manifest), actual_python_tests=8,
                     inherited_rust_tests=[1231, 0, 5], fresh_cargo_check=True,
                     scope='Prepared tools and exact retained validation; full proof matrix remains pending.')))
