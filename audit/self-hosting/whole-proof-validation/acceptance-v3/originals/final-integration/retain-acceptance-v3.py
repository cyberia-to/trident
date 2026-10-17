"""Retain frozen checker preparation/reviews with exact decoded-byte readback."""
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent
BASE = ROOT.parent
REPO = ROOT/'trident'
SOURCE = BASE/'whole-proof-final-review-v3'
DEST = REPO/'audit/self-hosting/whole-proof-validation/acceptance-v3'
MANIFEST_SHA = '3bb3470f75daa39c263a0b36cc9b1b9f0a891b19669aaf6aa263fcebbad68e96'
REVIEW_SHA = '4e7c65de21545a6efc1cb0f1aeb898bfad81c8ecb37540d4c122a7a04211a752'
LAUNCH = BASE/'whole-final-checker-root-launch-review.json'
LAUNCH_SHA = '9e1fec7e10835702faf7f9f67c3a10544d7ac9a9de84737ad017f55ce530f134'
PRIVATE = BASE/'whole-proof-attacks-completion-v3/reclamation-failure-observation/stdout'
METADATA = {'.gitattributes', 'README.md', 'check_delivery.py'}
EXTRA = {'sources.json', 'pins.json', 'preparation.json', 'preparation-2.json',
         'preparation-3.json', 'independent-review.json', 'draft-source-inventory-1.json',
         'actual-prior-resources-1.json', 'actual-prior-resources-2.json',
         'actual-transition-review-1.json', 'actual-transition-review-2.json'}


def require(value, message):
    if not value:
        raise ValueError(message)


def identity(data):
    return dict(bytes=len(data), sha256=hashlib.sha256(data).hexdigest())


def read(path):
    require(path.is_file() and not path.is_symlink(), 'regular retained input: '+str(path))
    require(path.stat().st_size <= 16*1024**2, 'bounded evidence file')
    return path.read_bytes()


def tree(root):
    for path in root.rglob('*'):
        if '__pycache__' in path.parts or path.suffix == '.pyc':
            continue
        require(not path.is_symlink(), 'symlink in frozen evidence tree')
        if path.is_file():
            yield path


def selection(manifest):
    paths = {Path(name) for name in manifest} | {SOURCE/name for name in EXTRA}
    for path in SOURCE.iterdir():
        if path.is_dir() and (path.name == 'review-snapshots' or path.name.startswith('offline-tests-')):
            paths.update(tree(path))
    for name in ('whole-final-checker-independent-review', 'whole-final-checker-root-review'):
        paths.update(tree(BASE/name))
    paths.update((LAUNCH, Path(__file__).resolve()))
    require(PRIVATE not in paths, 'private full-command diagnostic stays local')
    require(all(p.is_relative_to(BASE) for p in paths), 'owned original locations')
    require(not any({'orchestration', 'watcher-launch', '__pycache__'} & set(p.parts) for p in paths),
            'live watcher outputs excluded')
    return paths


def main():
    require(DEST.is_dir() and {p.name for p in DEST.iterdir()} == METADATA,
            'fresh delivery containing only authored metadata')
    manifest_path = SOURCE/'sources.json'
    require(identity(read(manifest_path))['sha256'] == MANIFEST_SHA, 'exact frozen 13-source manifest')
    manifest = json.loads(read(manifest_path))
    require(len(manifest) == 13 and all(identity(read(Path(p))) == v for p,v in manifest.items()),
            'complete exact checker source closure')
    review = json.loads(read(SOURCE/'independent-review.json'))
    require(identity(read(SOURCE/'independent-review.json'))['sha256'] == REVIEW_SHA
            and review['status'] == 'passed-source-review' and review['sources'] == manifest,
            'exact independent checker review')
    launch = json.loads(read(LAUNCH))
    require(identity(read(LAUNCH))['sha256'] == LAUNCH_SHA and launch['sources'] == manifest
            and launch['review_sha256'] == REVIEW_SHA and launch['status'] == 'passed-source-launch-review',
            'exact root launch review')
    selected = selection(manifest)
    before = {p: identity(read(p)) for p in selected}
    require(sum(v['bytes'] for v in before.values()) <= 32*1024**2, 'bounded frozen evidence selection')
    rows = {}
    for path in sorted(selected):
        data = read(path)
        require(identity(data) == before[path], 'source changed during retention')
        name = path.relative_to(BASE).as_posix()
        stored = Path('originals')/name
        encoding = 'identity' if path.suffix in ('.py', '.json', '.md', '.rs', '.toml', '.lock') else 'gzip'
        if encoding == 'gzip':
            stored = Path(str(stored)+'.gz')
        payload = gzip.compress(data, mtime=0) if encoding == 'gzip' else data
        destination = DEST/stored
        destination.parent.mkdir(parents=True, exist_ok=True)
        with destination.open('xb') as out:
            out.write(payload)
        reread = read(destination)
        decoded = gzip.decompress(reread) if encoding == 'gzip' else reread
        require(identity(reread) == identity(payload) and decoded == data, 'complete stored/decoded readback')
        rows[name] = dict(original_path=str(path), original=before[path], stored_path=stored.as_posix(),
                          stored=identity(payload), encoding=encoding)
    require(selection(manifest) == selected and all(identity(read(p)) == value for p,value in before.items()),
            'all original bytes and frozen membership unchanged')
    record = dict(schema='trident/frozen-final-checker-retention/v1', status='retained-and-readback-checked',
        created_ns=time.time_ns(), command=[sys.executable, *sys.argv], cwd=str(Path.cwd()),
        repository_revision=subprocess.check_output(['/usr/bin/git', '-C', str(REPO), 'rev-parse', 'HEAD'], text=True).strip(),
        scope='Frozen final checker, preparation, historical source snapshots, offline tests and source reviews only. '
              'Live watcher/orchestration outputs excluded. No final SH8 acceptance or durable whole-proof claim.',
        source=identity(read(Path(__file__))), original_base=str(BASE), source_manifest=identity(read(manifest_path)),
        independent_review=identity(read(SOURCE/'independent-review.json')), root_launch_review=identity(read(LAUNCH)),
        original_inputs_unchanged=True, files=rows,
        delivery_files={name: identity(read(DEST/name)) for name in sorted(METADATA)},
        totals=dict(files=len(rows), original_bytes=sum(r['original']['bytes'] for r in rows.values()),
                    stored_bytes=sum(r['stored']['bytes'] for r in rows.values())),
        exclusions=['whole-proof-final-review-v3/orchestration/**', 'whole-proof-final-review-v3/watcher-launch/**',
                    '**/__pycache__/**', '**/*.pyc', str(PRIVATE)])
    with (DEST/'files.json').open('x') as out:
        json.dump(record, out, indent=2)
        out.write('\n')
    print(json.dumps(dict(**record['totals'], manifest=identity(read(DEST/'files.json')))))


if __name__ == '__main__':
    main()
