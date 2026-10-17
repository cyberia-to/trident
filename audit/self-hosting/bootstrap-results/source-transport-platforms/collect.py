"""Retain all original source-transport CI files, including their original index."""
from pathlib import Path
import gzip
import hashlib
import io
import json
import tarfile

R = Path('/Users/master/cyber/.worktrees/selfhost-0.4-full-bootstrap')
IN = R / 'measurements/source-transport-ci-readonly'
OUT = R / 'portable-native-smoke/trident-transport-ci/audit/self-hosting/bootstrap-results/source-transport-platforms'

def identity(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())

original = json.loads((IN / 'files.json').read_bytes())
files = {}
for row in original:
    raw = (IN / row['path']).read_bytes()
    assert identity(raw) == {k: row[k] for k in ('bytes', 'sha256')}
    files[row['path']] = raw
files['files.json'] = (IN / 'files.json').read_bytes()
assert len(files) == 198
assert {p.relative_to(IN).as_posix() for p in IN.rglob('*') if p.is_file()} == set(files)
buffer = io.BytesIO()
with tarfile.open(fileobj=buffer, mode='w', format=tarfile.USTAR_FORMAT) as tar:
    for name, raw in sorted(files.items()):
        member = tarfile.TarInfo(name)
        member.size, member.mode, member.mtime = len(raw), 0o644, 0
        member.uid = member.gid = 0
        tar.addfile(member, io.BytesIO(raw))
archive = gzip.compress(buffer.getvalue(), mtime=0)
OUT.mkdir(parents=True)
(OUT / 'evidence.tar.gz').write_bytes(archive)
with tarfile.open(fileobj=io.BytesIO(archive), mode='r:gz') as tar:
    for member in tar:
        assert tar.extractfile(member).read() == (IN / member.name).read_bytes() == files[member.name]
receipt = dict(schema='trident/native-source-transport-retention/v1', status='retained',
               run_id=36400752982, run_attempt=1, revision='85ab2d3f33d4e5d5e057600782aeb99bed2dfeb2',
               archive=identity(archive), tar=identity(buffer.getvalue()), members=len(files),
               raw_bytes=sum(map(len, files.values())), files={n: identity(b) for n, b in sorted(files.items())},
               original_index=identity(files['files.json']), original_directory=str(IN),
               collector=identity(Path(__file__).read_bytes()),
               scope='Six native Python source-preparation jobs, separate from compiler execution and SH6')
(OUT / 'retention.json').write_text(json.dumps(receipt, indent=2) + '\n')
(OUT / 'collect.py').write_bytes(Path(__file__).read_bytes())
print(json.dumps({k: receipt[k] for k in ('members', 'raw_bytes', 'archive', 'tar')}))
