#!/usr/bin/env python3
"""Retain and restore exact source-size artifacts; never replace differing files."""
import argparse
import gzip
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import tarfile

ROOT = Path(__file__).resolve().parent
ARCHIVE = ROOT / 'artifacts.tar.gz'
INDEX = ROOT / 'artifacts.json'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def safe(name):
    path = PurePosixPath(name)
    require(not path.is_absolute() and '..' not in path.parts and str(path) == name, 'unsafe archive path')
    target = ROOT / path
    require(target.resolve().is_relative_to(ROOT), 'archive path escapes root through a symlink')
    return target


def verify(restore=False):
    index = json.loads(INDEX.read_text())
    require(digest(ARCHIVE.read_bytes()) == index['archive_sha256'], 'archive hash mismatch')
    seen = set()
    with tarfile.open(ARCHIVE, 'r:gz') as archive:
        for entry in archive:
            require(entry.isfile() and entry.name not in seen and entry.name in index['files'], 'unexpected archive member')
            seen.add(entry.name)
            with archive.extractfile(entry) as stream:
                data = stream.read()
            row = index['files'][entry.name]
            require(len(data) == row['bytes'] and digest(data) == row['sha256'], 'archived file bytes changed')
            target = safe(entry.name)
            if restore:
                target.parent.mkdir(parents=True, exist_ok=True)
                require(not target.is_symlink(), 'restore target is a symlink')
                if not target.exists():
                    with target.open('xb') as output:
                        output.write(data)
            require(target.is_file() and target.read_bytes() == data, 'retained file differs or is absent')
    require(seen == set(index['files']), 'missing archive members')
    return len(seen)


def create():
    require(not ARCHIVE.exists() and not INDEX.exists(), 'archive/index already exists')
    paths = []
    receipts = {}
    for name in ['receipt.json', 'negative-receipt.json']:
        receipt = ROOT / name
        document = json.loads(receipt.read_text())
        directory = Path(document['artifact_directory']).resolve()
        require(directory.parent == ROOT, 'only own generated directories may be archived')
        receipts[name] = digest(receipt.read_bytes())
        paths.extend(sorted(path for path in directory.rglob('*') if path.is_file()))
    files = {}
    with ARCHIVE.open('xb') as raw:
        with gzip.GzipFile(fileobj=raw, mode='wb', filename='', mtime=0) as compressed:
            with tarfile.open(fileobj=compressed, mode='w|') as archive:
                for path in sorted(paths):
                    require(not path.is_symlink(), 'source symlink')
                    name = str(path.relative_to(ROOT))
                    data = path.read_bytes()
                    require(name not in files, 'duplicate source path')
                    files[name] = dict(bytes=len(data), sha256=digest(data))
                    entry = tarfile.TarInfo(name)
                    entry.size, entry.mode, entry.mtime = len(data), 0o644, 0
                    archive.addfile(entry, io.BytesIO(data))
    index = dict(schema='trident/source-scale-files/v1', command='python3 audit/self-hosting/source-scale-compacting/archive.py',
                 script_sha256=digest(Path(__file__).read_bytes()), archive_sha256=digest(ARCHIVE.read_bytes()),
                 archive_bytes=ARCHIVE.stat().st_size, receipt_hashes=receipts, files=files)
    with INDEX.open('x') as output:
        json.dump(index, output, indent=2)
        output.write('\n')
    return verify()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument('--restore', action='store_true')
    mode.add_argument('--verify', action='store_true')
    args = parser.parse_args()
    count = verify(args.restore) if args.restore or args.verify else create()
    print(json.dumps(dict(status='passed', files=count, restore=args.restore)))
