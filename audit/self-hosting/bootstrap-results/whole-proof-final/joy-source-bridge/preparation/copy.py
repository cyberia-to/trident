"""Stage the explicitly selected source bridge; never write to original paths."""
import hashlib
import json
import os
from pathlib import Path
import stat
import time

ROOT = Path(__file__).resolve().parent
PACKET = ROOT / 'packet'
MAX_READ = 16 * 1024 * 1024
MAX_FILE = 8 * 1024 * 1024
MAX_OUTPUT = 4 * 1024 * 1024
started = time.monotonic()
read_bytes = 0
written_bytes = 0


def identity(data):
    return {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}


def guard():
    assert time.monotonic() - started < 20, '20-second staging bound'
    assert read_bytes <= MAX_READ and written_bytes <= MAX_OUTPUT, 'byte bound'


def read(path, expected=None):
    global read_bytes
    guard()
    path = Path(path)
    before = path.lstat()
    assert stat.S_ISREG(before.st_mode) and before.st_size <= MAX_FILE
    with path.open('rb') as f:
        assert os.fstat(f.fileno()) == before
        data = f.read(MAX_FILE + 1)
        assert os.fstat(f.fileno()) == before
    assert path.lstat() == before
    read_bytes += len(data)
    guard()
    if expected is not None:
        assert identity(data) == expected, str(path)
    return data


def write(path, data):
    global written_bytes
    guard()
    written_bytes += len(data)
    guard()
    with path.open('xb') as f:
        f.write(data)


def encoded(value):
    return (json.dumps(value, indent=2, sort_keys=True) + '\n').encode()


def main():
    plan_bytes = read(ROOT / 'inputs.json')
    plan = json.loads(plan_bytes)
    assert plan['schema'] == 'trident/joy-source-bridge-delivery/v1'
    assert len(plan['originals']) == 98 and len(plan['references']) == 2
    source = read(__file__)
    template = read(ROOT / 'README.template.md', plan['readme'])
    verifier = read(plan['verifier']['path'], {k: plan['verifier'][k] for k in ('bytes', 'sha256')})
    assert identity(verifier)['sha256'] == '7084b3e31fff02d78533e48998ac2994afc1cbd2744a6586fe9335315ceae446'
    PACKET.mkdir()
    (PACKET / 'objects').mkdir()
    rows = []
    originals = set()
    for row in plan['originals']:
        assert set(row) == {'original', 'bytes', 'sha256'} and row['original'] not in originals
        originals.add(row['original'])
        data = read(row['original'], {k: row[k] for k in ('bytes', 'sha256')})
        stored = 'objects/' + row['sha256']
        if not (PACKET / stored).exists():
            write(PACKET / stored, data)
        else:
            assert read(PACKET / stored) == data
        rows.append({**row, 'stored': stored})
    for row in plan['references']:
        read(row['original'], {k: row[k] for k in ('bytes', 'sha256')})
    write(PACKET / 'retained-files.json', encoded(rows))
    write(PACKET / 'references.json', encoded(plan['references']))
    write(PACKET / 'README.md', template)
    write(PACKET / 'verify-retained.py', verifier)
    # The one original raw Git diff contains blank context lines with a space.
    attrs = ('* -text\nobjects/ad057c18c0ef4e86fec6d6405f7cbcf87203132d38bdb64800a158095b7fe246 '
             'whitespace=-blank-at-eol\n').encode()
    write(PACKET / '.gitattributes', attrs)
    assert read(ROOT / 'inputs.json') == plan_bytes
    assert read(__file__) == source
    assert read(ROOT / 'README.template.md') == template
    for row in plan['originals']:
        read(row['original'], {k: row[k] for k in ('bytes', 'sha256')})
    files = {str(p.relative_to(PACKET)): identity(read(p))
             for p in sorted(PACKET.rglob('*')) if p.is_file()}
    write(ROOT / 'copy-inventory.json', encoded(files))
    receipt = {'status': 'passed-local-archive-staging', 'scope': 'Exact-byte copying only; verification is separate.',
               'argv': ['python3', '-B', '-W', 'error', str(Path(__file__))],
               'source': identity(source), 'inputs': identity(plan_bytes),
               'original_files': len(rows), 'unique_objects': len({r['stored'] for r in rows}),
               'original_bytes': sum(r['bytes'] for r in rows), 'packet_files': len(files),
               'packet_bytes': sum(r['bytes'] for r in files.values()),
               'references_only': len(plan['references']),
               'copy_inventory': identity((ROOT / 'copy-inventory.json').read_bytes()),
               'read_bytes': read_bytes, 'written_bytes_before_receipt': written_bytes,
               'elapsed_seconds': time.monotonic() - started,
               'caps': {'wall_seconds': 20, 'read_bytes': MAX_READ, 'single_file_bytes': MAX_FILE,
                        'output_bytes': MAX_OUTPUT}}
    write(ROOT / 'receipt.json', encoded(receipt))
    print(json.dumps(receipt))


if __name__ == '__main__':
    main()
