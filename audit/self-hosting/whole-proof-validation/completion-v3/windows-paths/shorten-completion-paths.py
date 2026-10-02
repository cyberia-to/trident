"""Shorten stored audit paths while retaining all original bytes and locations."""
import copy
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import sys


def identity(data):
    return dict(bytes=len(data), sha256=hashlib.sha256(data).hexdigest())


repo = Path(sys.argv[1]).resolve()
base = Path(__file__).resolve().parent
revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=repo, text=True).strip()
assert revision == 'fdeda5fe903c10df4a49b1b247f20f5fa2ae3bdf'
root = repo / 'audit/self-hosting/whole-proof-validation/completion-v3'
record = root / 'files.json'
raw = record.read_bytes()
before = json.loads(raw)
after = copy.deepcopy(before)
evidence = root / 'windows-paths'
evidence.mkdir()
(evidence / 'original-files.json').write_bytes(raw)
(evidence / 'shorten-completion-paths.py').write_bytes(Path(__file__).read_bytes())
moves = []
stored_files = 0
for name, row in after['files'].items():
    if 'stored_path' not in row:
        continue
    path = root / row['stored_path']
    assert path.is_file() and not path.is_symlink()
    data = path.read_bytes()
    assert identity(data) == row['stored']
    decoded = gzip.decompress(data) if row['encoding'] == 'gzip' else data
    assert identity(decoded) == row['original']
    stored_files += 1
    if len(path.relative_to(repo).as_posix().encode('utf-16-le')) // 2 > 200:
        suffix = '.gz' if row['encoding'] == 'gzip' else path.suffix
        short = 'objects/' + hashlib.sha256(name.encode()).hexdigest() + suffix
        target = root / short
        target.parent.mkdir(exist_ok=True)
        assert not target.exists()
        path.rename(target)
        assert target.read_bytes() == data
        moves.append(dict(original=name, before=row['stored_path'], after=short,
                          stored=row['stored'], decoded=row['original']))
        row['stored_path'] = short
assert len(moves) == 36 and stored_files == 327
after['storage_migration'] = dict(
    original_manifest=identity(raw), original_manifest_path='windows-paths/original-files.json',
    source=identity(Path(__file__).read_bytes()), source_path='windows-paths/shorten-completion-paths.py',
    scope='Stored path changes only; original bytes, encoding, provenance and identities unchanged.',
    moved_files=len(moves))
record.write_text(json.dumps(after, indent=2) + '\n')
for name, row in after['files'].items():
    old = before['files'][name]
    assert {k: v for k, v in row.items() if k != 'stored_path'} == {k: v for k, v in old.items() if k != 'stored_path'}
    if 'stored_path' in row:
        data = (root / row['stored_path']).read_bytes()
        assert identity(data) == row['stored']
        assert identity(gzip.decompress(data) if row['encoding'] == 'gzip' else data) == row['original']
receipt = dict(status='passed-byte-preserving-path-migration', revision=revision,
               command=[sys.executable, *sys.argv], cwd=str(Path.cwd()),
               source=identity(Path(__file__).read_bytes()), original_manifest=identity(raw),
               current_manifest=identity(record.read_bytes()), retained_files=stored_files,
               moves=moves, original_records_unchanged=True)
(evidence / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
print(json.dumps(dict(status=receipt['status'], retained_files=stored_files, moved_files=len(moves))))
