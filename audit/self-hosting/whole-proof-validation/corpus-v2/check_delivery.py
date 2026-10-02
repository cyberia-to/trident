"""Check the complete failed original corpus and the reviewed replacement tools."""
import ast
import hashlib
import json
from pathlib import Path
import tarfile

ROOT = Path(__file__).resolve().parent


def identity(data):
    return dict(bytes=len(data), sha256=hashlib.sha256(data).hexdigest())


archive = json.loads((ROOT / 'archive.json').read_text())
index = json.loads((ROOT / 'original-files.json').read_text())
assert identity((ROOT / 'original-corpus-v1-failed.tar.gz').read_bytes()) == archive['archive']
with tarfile.open(ROOT / 'original-corpus-v1-failed.tar.gz', 'r:gz') as container:
    members = container.getmembers()
    assert len(members) == len(index) == archive['files']
    assert {m.name for m in members} == set(index)
    assert sum(m.size for m in members) == archive['logical_bytes']
    for member in members:
        assert member.isfile() and not Path(member.name).is_absolute() and '..' not in Path(member.name).parts
        assert identity(container.extractfile(member).read()) == index[member.name]
    def load(name):
        return json.load(container.extractfile(name))
    failed = load('c3/receipt.json')
    assert failed['status'] == 'failed' and len(failed['commands']) == 6
    assert [row['exit_code'] for row in failed['commands']] == [0, 0, 0, 0, 0, 1]
    assert failed['fixture_revision'] == 'e57f2b4c6c6b7128e1cbbf80a77a0ccfcc8d33d7'
    last = load('c3/corpora/c3-check-generated-compiler-profile.json')
    assert last['status'] == 'failed' and last['commands'][0]['command'] == ['git', 'rev-parse', 'HEAD']
    assert last['failure'] == "FileNotFoundError: [Errno 2] No such file or directory: 'git'"
prepared = json.loads((ROOT / 'prepared-files.json').read_text())
by_original = {}
for name, expected in prepared.items():
    path = ROOT / name
    assert identity(path.read_bytes()) == {key: expected[key] for key in ('bytes', 'sha256')}
    by_original[expected['original']] = path
    if path.suffix == '.py':
        ast.parse(path.read_text())
scope = ROOT / 'prepared/whole-proof-corpus-v2'
review = json.loads((scope / 'independent-review.json').read_text())
assert review['status'] == 'passed-source-review'
assert review['sources'] == json.loads((scope / 'sources.json').read_text())
for original, expected in review['sources'].items():
    assert identity(by_original[original].read_bytes()) == expected
print(json.dumps(dict(status='passed', original_failed_files=len(index),
    original_logical_bytes=archive['logical_bytes'], reviewed_prepared_files=len(prepared),
    scope='Exact original failure and reviewed replacement tools; new corpus result remains pending.')))
