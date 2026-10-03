"""Copy the fixed small public evidence selection outside all live repositories."""
import hashlib
import json
from pathlib import Path
import stat
import time

ROOT = Path(__file__).resolve().parent
PACKET = ROOT / 'packet'
START = time.monotonic()
read_bytes = 0
written_bytes = 0


def identity(data):
    return {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}


def guard():
    assert time.monotonic() - START < 30
    assert read_bytes <= 16 * 1024**2 and written_bytes <= 4 * 1024**2


def read(path, expected=None):
    global read_bytes
    guard()
    path = Path(path)
    before = path.lstat()
    assert stat.S_ISREG(before.st_mode) and before.st_size <= 2 * 1024**2
    data = path.read_bytes()
    after = path.lstat()
    assert (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns) == (
        after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns)
    read_bytes += len(data)
    guard()
    if expected is not None:
        assert identity(data) == expected, path
    return data


def write(path, data):
    global written_bytes
    written_bytes += len(data)
    guard()
    with path.open('xb') as f:
        f.write(data)


def encoded(value):
    return (json.dumps(value, indent=2, sort_keys=True) + '\n').encode()


selection = read(ROOT / 'selection.json')
assert identity(selection)['sha256'] == '0eb52b01b7574894e09d2502943ebe12c6aa9ae8af3af0ad9ad54439f8531eb8'
plan = json.loads(selection)
assert len(plan['originals']) == 283 and len(plan['references_only']) == 21
source = read(__file__)
readme = read(ROOT / 'README.template.md')
v = plan['verifier']
verifier = read(v['path'], {k: v[k] for k in ('bytes', 'sha256')})
assert identity(verifier)['sha256'] == '7084b3e31fff02d78533e48998ac2994afc1cbd2744a6586fe9335315ceae446'
PACKET.mkdir()
(PACKET / 'objects').mkdir()
rows = []
stored = set()
whitespace = set()
originals = list(plan['originals']) + [
    {'original': str(ROOT / name), **identity(data)}
    for name, data in [('copy.py', source), ('selection.json', selection), ('README.template.md', readme)]
]
assert len({r['original'] for r in originals}) == len(originals)
for row in originals:
    data = read(row['original'], {k: row[k] for k in ('bytes', 'sha256')})
    object_path = 'objects/' + row['sha256']
    if object_path not in stored:
        write(PACKET / object_path, data)
        stored.add(object_path)
        if any(line.rstrip(b' \t') != line for line in data.splitlines()):
            whitespace.add(object_path)
    rows.append({**row, 'stored': object_path})
for row in plan['references_only']:
    read(row['original'], {k: row[k] for k in ('bytes', 'sha256')})
write(PACKET / 'retained-files.json', encoded(rows))
write(PACKET / 'references-only.json', encoded(plan['references_only']))
write(PACKET / 'README.md', readme)
write(PACKET / 'verify-retained.py', verifier)
write(PACKET / '.gitattributes', ('* -text\n' + ''.join(p + ' whitespace=-blank-at-eol\n' for p in sorted(whitespace))).encode())
assert read(ROOT / 'selection.json') == selection and read(__file__) == source
assert read(ROOT / 'README.template.md') == readme
for row in originals:
    read(row['original'], {k: row[k] for k in ('bytes', 'sha256')})
inventory = {str(p.relative_to(PACKET)): identity(read(p))
             for p in sorted(PACKET.rglob('*')) if p.is_file()}
write(ROOT / 'copy-inventory.json', encoded(inventory))
receipt = {'status': 'passed-bounded-public-archive-staging',
           'scope': 'Exact selected original copying only; independent archive verification follows.',
           'source': identity(source), 'selection': identity(selection),
           'original_files': len(rows), 'original_bytes': sum(r['bytes'] for r in rows),
           'unique_objects': len(stored), 'references_only': len(plan['references_only']),
           'packet_files': len(inventory), 'packet_bytes': sum(r['bytes'] for r in inventory.values()),
           'copy_inventory': identity((ROOT / 'copy-inventory.json').read_bytes()),
           'elapsed_seconds': time.monotonic() - START, 'read_bytes': read_bytes,
           'written_bytes_before_receipt': written_bytes, 'caps': plan['caps'],
           'whitespace_exception_objects': sorted(whitespace)}
write(ROOT / 'receipt.json', encoded(receipt))
print(json.dumps(receipt))
