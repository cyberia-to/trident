"""Restore exact retained CI bytes into a fresh directory and replay their checks."""
import argparse
import gzip
import hashlib
import io
import json
import stat
from pathlib import Path, PurePosixPath
import subprocess
import sys
import tarfile

ROOT = Path(__file__).resolve().parent
ARCHIVE_SHA = 'd4ff213725b9c9605eecb4d2aba5ecdfa7786847ebbca16b5c7b24b3ac3102bb'


def require(value, message):
    if not value:
        raise ValueError(message)


def identity(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def bounded(path, maximum):
    require(stat.S_ISREG(path.lstat().st_mode) and path.stat().st_size <= maximum,
            'bounded ordinary input: ' + str(path))
    with path.open('rb') as stream:
        raw = stream.read(maximum + 1)
    require(len(raw) <= maximum, 'input grew beyond bound')
    return raw


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    output = args.output.absolute()
    require(not output.exists() and not output.is_symlink(), 'fresh output required')
    for parent in (output.parent, *output.parent.parents):
        require(parent.is_dir() and not parent.is_symlink(), 'ordinary existing output parent')
        require(not parent.samefile(ROOT), 'output overlaps retained inputs')
    require(not output.resolve().is_relative_to(ROOT.resolve()) and
            not ROOT.resolve().is_relative_to(output.resolve()), 'separate output required')
    index = json.loads(bounded(ROOT / 'retention.json', 256 << 10))
    archive = bounded(ROOT / 'evidence.tar.gz', 8 << 20)
    require(identity(archive) == index['archive'] and
            identity(archive)['sha256'] == ARCHIVE_SHA, 'exact retained archive')
    with gzip.GzipFile(fileobj=io.BytesIO(archive)) as stream:
        raw = stream.read((16 << 20) + 1)
    require(len(raw) <= 16 << 20 and identity(raw) == index['tar'], 'bounded original tar')
    files = {}
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:') as tar:
        for member in tar:
            path = PurePosixPath(member.name)
            require(member.isfile() and not path.is_absolute() and path.as_posix() == member.name and
                    all(p not in ('', '.', '..') and ':' not in p and '\\' not in p for p in path.parts),
                    'ordinary canonical member')
            require(member.name not in files and len(files) < 256 and 0 <= member.size <= 8 << 20,
                    'bounded unique member')
            require(member.uid == member.gid == member.mtime == 0 and member.mode == 0o644,
                    'retained tar metadata')
            files[member.name] = tar.extractfile(member).read()
    require({name: identity(data) for name, data in files.items()} == index['files'] and
            len(files) == index['members'] == 198 and sum(map(len, files.values())) == index['raw_bytes'],
            'complete exact retained tree')
    output.mkdir()
    for name, data in files.items():
        path = output / name
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(data)
        require(path.read_bytes() == data, 'restored bytes')
    command = [sys.executable, '-B', '-W', 'error', str(output / 'verify.py'), 'root-replay.json']
    result = subprocess.run(command, capture_output=True)
    (output / 'root-replay.stdout').write_bytes(result.stdout)
    (output / 'root-replay.stderr').write_bytes(result.stderr)
    (output / 'root-command.json').write_text(json.dumps(dict(argv=command, cwd=str(Path.cwd()),
                                                            exit_code=result.returncode), indent=2) + '\n')
    require(result.returncode == 0, 'original CI replay failed; logs retained')
    verdict = json.loads((output / 'root-replay.json').read_bytes())
    require(verdict['status'] == 'passed', 'complete retained CI verdict')
    print(json.dumps(dict(status='passed', members=len(files), targets=verdict['native_targets'],
                         passed=verdict['passed_case_executions'], skipped=verdict['skipped_case_executions'],
                         scope='Retained native source-transport results; no new tests or compiler execution')))


if __name__ == '__main__':
    main()
