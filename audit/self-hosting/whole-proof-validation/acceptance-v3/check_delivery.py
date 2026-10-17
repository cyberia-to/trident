"""Replay all retained stored and decoded identities; optional original-file check."""
import argparse
import gzip
import hashlib
import io
import json
from pathlib import Path


def require(value, message):
    if not value:
        raise ValueError(message)


def identity(data):
    return dict(bytes=len(data), sha256=hashlib.sha256(data).hexdigest())


def read(path, maximum=16*1024**2):
    require(path.is_file() and not path.is_symlink() and path.stat().st_size <= maximum,
            'bounded regular file: '+str(path))
    return path.read_bytes()


def check(root, originals=False):
    manifest = json.loads(read(root/'files.json'))
    require(manifest['schema'] == 'trident/frozen-final-checker-retention/v1'
            and manifest['status'] == 'retained-and-readback-checked'
            and manifest['original_inputs_unchanged'] is True, 'retention state')
    wanted = {'files.json'}
    for name, expected in manifest['delivery_files'].items():
        require(Path(name).name == name, 'delivery metadata filename')
        wanted.add(name)
        require(identity(read(root/name)) == expected, 'delivery metadata identity')
    original_bytes = stored_bytes = 0
    for name, row in manifest['files'].items():
        require(type(row['original']['bytes']) is int and 0 <= row['original']['bytes'] <= 16*1024**2,
                'bounded decoded file')
        source = Path(manifest['original_base'])/name
        require(not Path(name).is_absolute() and '..' not in Path(name).parts
                and str(source) == row['original_path'], 'explicit original location')
        relative = Path(row['stored_path'])
        require(not relative.is_absolute() and '..' not in relative.parts
                and relative.parts[0] == 'originals', 'safe stored path')
        require(relative.as_posix() not in wanted, 'unique stored path')
        wanted.add(relative.as_posix())
        data = read(root/relative)
        require(identity(data) == row['stored'], 'stored identity: '+name)
        require(row['encoding'] in ('identity', 'gzip'), 'known retained encoding')
        if row['encoding'] == 'gzip':
            with gzip.GzipFile(fileobj=io.BytesIO(data), mode='rb') as stream:
                decoded = stream.read(row['original']['bytes']+1)
                require(len(decoded) == row['original']['bytes'] and stream.read(1) == b'', 'decoded byte bound')
        else:
            decoded = data
        require(identity(decoded) == row['original'], 'full decoded identity: '+name)
        if originals:
            require(identity(read(source)) == row['original'], 'unchanged original: '+name)
        original_bytes += len(decoded)
        stored_bytes += len(data)
        require(original_bytes <= 32*1024**2, 'bounded decoded delivery')
    actual = set()
    for path in root.rglob('*'):
        require(not path.is_symlink(), 'delivery contains a symlink')
        if path.is_file():
            actual.add(path.relative_to(root).as_posix())
    require(actual == wanted, 'exact delivery membership')
    totals = dict(files=len(manifest['files']), original_bytes=original_bytes, stored_bytes=stored_bytes)
    require(totals == manifest['totals'], 'retained counts and byte totals')
    return dict(status='passed-complete-decoded-readback', **totals,
                originals_checked=originals, manifest=identity(read(root/'files.json')))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    args = parser.parse_args()
    print(json.dumps(check(Path(__file__).resolve().parent, args.originals), indent=2))
