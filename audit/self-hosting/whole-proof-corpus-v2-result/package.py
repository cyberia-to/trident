#!/usr/bin/env python3
"""Snapshot the completed corpus scopes without modifying their evidence."""
import argparse
import gzip
import hashlib
import io
import json
from pathlib import Path
import stat
import tarfile

ROOT = Path(__file__).resolve().parent


def identity(data):
    return {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}


def inventory(directory):
    result = {}
    for path in sorted(directory.rglob('*')):
        mode = path.lstat().st_mode
        if stat.S_ISDIR(mode):
            continue
        if not stat.S_ISREG(mode):
            raise ValueError(f'non-regular evidence: {path}')
        result[str(path.relative_to(directory))] = identity(path.read_bytes())
    return result


def write_json(path, value):
    with path.open('x') as stream:
        stream.write(json.dumps(value, indent=2, sort_keys=True) + '\n')


def package(source, destination):
    scope = source / 'whole-proof-corpus-v2'
    original_index = inventory(scope)
    for label in ('c2', 'c3', 'orchestration'):
        if json.loads((scope / label / 'receipt.json').read_text())['status'] != 'passed':
            raise ValueError(f'incomplete scope: {label}')
    paths = {f'corpus/{name}': scope / name for name in original_index}
    for generation in (1, 2):
        fresh = source / f'whole-proof/attempts/c{generation}-fresh-verification-1'
        for name in ('receipt.json', 'stdout', 'stderr', 'resources.jsonl', 'compiler.dag'):
            paths[f'fresh/c{generation}/{name}'] = fresh / name
        producer = source / f'whole-proof/attempts/c{generation}-selfbuild-1'
        for name in ('receipt.json', 'stdout', 'stderr'):
            paths[f'producer/c{generation}/{name}'] = producer / name
    paths.update({
        'inputs/joy': source / 'production-install/installed/bin/joy',
        'inputs/git': Path('/usr/bin/git'),
        'inputs/compiler_vectors.json': source / 'corpus-path-fix/joy/cli/tests/compiler_vectors.json',
        'inputs/joy-postinstall.json': source / 'production-install/postinstall/receipt.json',
    })
    prepared = ROOT.parent / 'whole-proof-validation/corpus-v2'
    for name, row in json.loads((prepared / 'prepared-files.json').read_text()).items():
        # Keep the reviewed copies at their original relative names; validate both sources.
        copy = prepared / name
        if identity(copy.read_bytes()) != {k: row[k] for k in ('bytes', 'sha256')}:
            raise ValueError(f'changed reviewed copy: {name}')
        if identity(Path(row['original']).read_bytes()) != identity(copy.read_bytes()):
            raise ValueError(f'changed reviewed source: {name}')
        paths[f'reviewed/{name}'] = copy
    index = {}
    archive_path = destination / 'successful-corpus.tar.gz'
    with archive_path.open('xb') as raw:
        with gzip.GzipFile(filename='', fileobj=raw, mode='wb', mtime=0, compresslevel=9) as compressed:
            with tarfile.open(fileobj=compressed, mode='w', format=tarfile.PAX_FORMAT) as archive:
                for name, path in sorted(paths.items()):
                    if not stat.S_ISREG(path.lstat().st_mode):
                        raise ValueError(f'non-regular source: {path}')
                    data = path.read_bytes()
                    index[name] = dict(identity(data), original=str(path))
                    member = tarfile.TarInfo(name)
                    member.size = len(data)
                    member.mode = 0o644
                    archive.addfile(member, io.BytesIO(data))
    for name, path in paths.items():
        if identity(path.read_bytes()) != {k: index[name][k] for k in ('bytes', 'sha256')}:
            raise ValueError(f'source changed during snapshot: {path}')
    if inventory(scope) != original_index:
        raise ValueError('corpus membership changed during snapshot')
    write_json(destination / 'files.json', index)
    manifest = {
        'schema': 'trident/whole-proof-corpus-delivery/v1',
        'scope': 'Measured C2/C3 corpus results and exact supporting bytes; SH8 acceptance pending.',
        'archive': identity(archive_path.read_bytes()),
        'inventory': identity((destination / 'files.json').read_bytes()),
        'files': len(index),
        'logical_bytes': sum(row['bytes'] for row in index.values()),
        'complete_original_scope': str(scope),
        'complete_original_files': len(original_index),
        'complete_original_bytes': sum(row['bytes'] for row in original_index.values()),
        'fixture_revision': '2f1a575a6fbc0704c268f1ae21667830a3c996df',
        'joy_revision': '6e0ec4d8440e2521df08f442d64f54e667044716',
        'prepared_inventory': identity((prepared / 'prepared-files.json').read_bytes()),
        'prior_failure_archive': json.loads((prepared / 'archive.json').read_text())['archive'],
    }
    write_json(destination / 'archive.json', manifest)
    return manifest


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path, help='the immutable completed R2 evidence family')
    parser.add_argument('destination', type=Path, help='an existing directory without package files')
    arguments = parser.parse_args()
    print(json.dumps(package(arguments.source, arguments.destination), indent=2))
