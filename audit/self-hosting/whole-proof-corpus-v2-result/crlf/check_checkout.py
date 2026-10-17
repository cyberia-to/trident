#!/usr/bin/env python3
"""Exercise real Git CRLF filters and replay the resulting audit checkout."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile

RESULT = 'audit/self-hosting/whole-proof-corpus-v2-result'
PREPARED = 'audit/self-hosting/whole-proof-validation/corpus-v2'
BASE = '5b68e00af9d0e41f8a181471a74e8b01f174f1d9'


def require(condition, message):
    if not condition:
        raise ValueError(message)


def identity(data):
    return dict(bytes=len(data), sha256=hashlib.sha256(data).hexdigest())


def git(repo, *args):
    return subprocess.check_output(['git', *args], cwd=repo)


def command(output, name, argv, cwd, expected=0, diagnostic=None):
    completed = subprocess.run(argv, cwd=cwd, capture_output=True, timeout=120)
    for channel in ('stdout', 'stderr'):
        with (output / f'{name}.{channel}').open('xb') as stream:
            stream.write(getattr(completed, channel))
    require(completed.returncode == expected, f'{name}: unexpected exit')
    if diagnostic:
        require(diagnostic.encode() in completed.stderr, f'{name}: missing expected diagnostic')
    return dict(name=name, argv=argv, cwd=str(cwd), exit_code=completed.returncode,
                stdout=identity(completed.stdout), stderr=identity(completed.stderr))


def tree_map(repo, revision):
    if revision == 'INDEX':
        rows = git(repo, 'ls-files', '--stage', '-z').split(b'\0')
        result = {}
        for row in filter(None, rows):
            metadata, name = row.split(b'\t', 1)
            mode, blob, stage = metadata.split()
            require(stage == b'0', 'unmerged index')
            result[name.decode()] = [mode.decode(), blob.decode()]
        return result
    rows = git(repo, 'ls-tree', '-rz', '--full-tree', revision).split(b'\0')
    return {name.decode(): [metadata.split()[0].decode(), metadata.split()[2].decode()]
            for metadata, name in (row.split(b'\t', 1) for row in filter(None, rows))}


def verify(repo, output, revision):
    output.mkdir(parents=True, exist_ok=False)
    prefix = ':' if revision == 'INDEX' else revision + ':'
    before = tree_map(repo, BASE)
    after = tree_map(repo, revision)
    changed = sorted(name for name in set(before) | set(after) if before.get(name) != after.get(name))
    require(changed and all(name == '.gitattributes' or name.startswith(RESULT + '/') for name in changed),
            'change exceeds reviewed audit/attributes scope')
    protected = lambda tree: {name: row for name, row in tree.items()
                              if name != '.gitattributes' and not name.startswith(RESULT + '/')}
    require(protected(before) == protected(after), 'tracked sources outside explicit metadata/audit delta changed')
    names = {name for name in after if name.startswith(RESULT + '/')}
    prepared_index = json.loads(git(repo, 'show', prefix + PREPARED + '/prepared-files.json'))
    names.update(PREPARED + '/' + name for name in prepared_index)
    names.update({PREPARED + '/prepared-files.json', PREPARED + '/original-corpus-v1-failed.tar.gz'})
    require(all(name in after for name in names), 'hash-bound input is untracked')
    bound_names = set(names)
    names.update(PREPARED + '/' + name for name in
                 ['archive.json', 'original-files.json', 'check_delivery.py', '.gitattributes'])
    records = []
    commands = []
    with tempfile.TemporaryDirectory(prefix='checkout-', dir=output) as temporary:
        checkout = Path(temporary)
        for name in sorted(names):
            blob = git(repo, 'show', prefix + name)
            argv = ['git', '-c', 'core.autocrlf=true', 'cat-file', '--filters', prefix + name]
            filtered = subprocess.run(argv, cwd=repo, capture_output=True, check=True)
            if name in bound_names:
                require(filtered.stdout == blob, f'CRLF changed hash-bound bytes: {name}')
            destination = checkout / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(filtered.stdout)
            records.append(dict(path=name, argv=argv, exit_code=filtered.returncode,
                                blob=identity(blob), filtered=identity(filtered.stdout),
                                stderr=filtered.stderr.decode(), hash_bound=name in bound_names))
        result_root = checkout / RESULT
        failure = json.loads((result_root / 'crlf/before.json').read_bytes())
        require(failure['base'] == BASE, 'pre-fix source pin')
        original_data = {}
        for row in failure['commands']:
            path = checkout / row['path']
            original_data[path] = path.read_bytes()
            bad = gzip.decompress((result_root / 'crlf' / row['retained_filtered']).read_bytes())
            require(identity(bad) == row['filtered'] and identity(path.read_bytes()) == row['blob'],
                    'pre-fix actual filtered bytes')
            path.write_bytes(bad)
        invoke = [sys.executable, '-B', '-W', 'error', str(result_root / 'check_delivery.py')]
        commands.append(command(output, 'before-both-inventories', invoke, checkout, 1, 'inventory identity'))
        first = checkout / RESULT / 'files.json'
        first.write_bytes(original_data[first])
        commands.append(command(output, 'before-prepared-inventory', invoke, checkout, 1, 'prepared source inventory'))
        for path, data in original_data.items():
            path.write_bytes(data)
        commands.append(command(output, 'filtered-result-checker', invoke, checkout))
        commands.append(command(output, 'filtered-prior-checker',
            [sys.executable, '-B', '-W', 'error', str(checkout / PREPARED / 'check_delivery.py')], checkout))
        commands.append(command(output, 'filtered-adversarial-tests',
            [sys.executable, '-B', '-W', 'error', '-m', 'unittest', 'test_delivery', '-v'], result_root))
    receipt = dict(status='passed', base=BASE, selected_tree=revision,
        selected_revision=git(repo, 'rev-parse', 'HEAD' if revision == 'INDEX' else revision).decode().strip(),
        repository=str(repo), changed_paths=changed,
        source_impact=dict(excluded_metadata=['.gitattributes'], excluded_audit=RESULT + '/',
            unchanged_tracked_paths=len(protected(after)),
            inventory=identity((json.dumps(protected(after), sort_keys=True) + '\n').encode()),
            scope='All tracked files outside the exact attributes/audit delta are byte-identical to the reviewed delivery.'),
        filter_commands=records, hash_bound_inputs=len(bound_names), materialized_inputs=len(names), commands=commands,
        scope='Actual Git Windows CRLF filter simulation on macOS, plus materialized checker/test replay; not a native Windows runtime test.')
    (output / 'receipt.json').write_text(json.dumps(receipt, indent=2, sort_keys=True) + '\n')
    return receipt


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--revision', default='HEAD', help='a commit or INDEX for the staged snapshot')
    args = parser.parse_args()
    result = verify(args.repo.resolve(), args.output.resolve(), args.revision)
    print(json.dumps(dict(status=result['status'], hash_bound_inputs=result['hash_bound_inputs'],
                          materialized_inputs=result['materialized_inputs'], commands=len(result['commands']))))
