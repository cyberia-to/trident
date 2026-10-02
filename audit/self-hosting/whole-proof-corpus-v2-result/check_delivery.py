#!/usr/bin/env python3
"""Replay archive integrity and the measured corpus/verified-compiler bindings."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import tarfile

from check_relations import check_relations

ROOT = Path(__file__).resolve().parent


def require(condition, message):
    if not condition:
        raise ValueError(message)


def identity(data):
    return {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}


def expected(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def read_archive(path, index, manifest):
    require(path.stat().st_size <= 32 * 1024 * 1024, 'archive byte cap')
    require(identity(path.read_bytes()) == manifest['archive'], 'archive identity')
    require(len(index) == manifest['files'] <= 10000, 'file count')
    require(sum(row['bytes'] for row in index.values()) == manifest['logical_bytes'] <= 128 * 1024 * 1024,
            'logical byte cap')
    payload = {}
    with tarfile.open(path, 'r:gz') as archive:
        for member in archive:
            name = member.name
            parts = PurePosixPath(name)
            require(member.isfile() and not parts.is_absolute() and '..' not in parts.parts
                    and str(parts) == name and name not in payload, 'unsafe or duplicate archive member')
            require(name in index and 0 <= member.size == index[name]['bytes'] <= 16 * 1024 * 1024,
                    'member identity/byte cap')
            with archive.extractfile(member) as stream:
                data = stream.read(member.size + 1)
            require(identity(data) == expected(index[name]), f'member digest: {name}')
            payload[name] = data
    require(set(payload) == set(index), 'archive membership')
    return payload


def check(root=ROOT, originals=False):
    manifest = json.loads((root / 'archive.json').read_bytes())
    raw_index = (root / 'files.json').read_bytes()
    require(identity(raw_index) == manifest['inventory'], 'inventory identity')
    index = json.loads(raw_index)
    payload = read_archive(root / 'successful-corpus.tar.gz', index, manifest)
    prepared_root = root.parent / 'whole-proof-validation/corpus-v2'
    prepared_raw = (prepared_root / 'prepared-files.json').read_bytes()
    require(identity(prepared_raw) == manifest['prepared_inventory'], 'prepared source inventory')
    prepared = json.loads(prepared_raw)
    for name, row in prepared.items():
        require(identity(payload[f'reviewed/{name}']) == expected(row), f'reviewed copy: {name}')
    require(identity((prepared_root / 'original-corpus-v1-failed.tar.gz').read_bytes())
            == manifest['prior_failure_archive'], 'prior failed archive identity')
    result = check_relations(payload, index, manifest, prepared)
    if originals:
        for name, row in index.items():
            path = Path(row['original'])
            require(path.is_file() and not path.is_symlink(), f'original regular file: {name}')
            require(identity(path.read_bytes()) == expected(row), f'original changed: {name}')
        scope = Path(manifest['complete_original_scope'])
        paths = list(scope.rglob('*'))
        require(not any(path.is_symlink() for path in paths), 'original scope symlink')
        actual = {str(path.relative_to(scope)) for path in paths if path.is_file()}
        require(actual == {name.removeprefix('corpus/') for name in payload if name.startswith('corpus/')},
                'original complete membership')
    return dict(status='passed', archive=manifest['archive'], files=len(index),
                logical_bytes=manifest['logical_bytes'], original_comparison=originals, **result)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true', help='also rehash every original on this host')
    arguments = parser.parse_args()
    print(json.dumps(check(originals=arguments.originals), indent=2, sort_keys=True))
