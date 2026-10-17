"""Check retained raw bytes and this observed run; invokes no compiler or runtime."""
import hashlib
import json
from pathlib import Path
import sys
import tarfile

ROOT = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent


def identity(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def require(value, label):
    if not value:
        raise ValueError(label)


receipt = json.loads((ROOT / 'receipt.json').read_bytes())
archive = ROOT / receipt['archive']['path']
require(identity(archive.read_bytes()) == {k: receipt['archive'][k] for k in ('bytes', 'sha256')}, 'archive identity')
rows = [row for page in sorted(ROOT.glob('files-*.json')) for row in json.loads(page.read_bytes())]
expected = {row['path']: row for row in rows}
require(len(rows) == len(expected) == receipt['members'], 'manifest completeness')
contents = {}
with tarfile.open(archive, 'r:gz') as tar:
    for member in tar:
        name = member.name
        require(name in expected and name not in contents, 'member set')
        require(member.isfile() or member.islnk(), 'ordinary file or deduplicated hardlink')
        require(member.mtime == member.uid == member.gid == 0 and member.mode == 0o644, 'deterministic metadata')
        if member.islnk():
            require(member.linkname in contents, 'prior hardlink target')
            raw = contents[member.linkname]
        else:
            require(member.size == expected[name]['bytes'], 'member size')
            raw = tar.extractfile(member).read()
        require(identity(raw) == {k: expected[name][k] for k in ('bytes', 'sha256')}, 'raw member identity: ' + name)
        contents[name] = raw
require(contents.keys() == expected.keys(), 'complete raw tree')


def load(path):
    return json.loads(contents[path])


def bound(path, row):
    require(identity(contents[path]) == {k: row[k] for k in ('bytes', 'sha256')}, 'receipt binding: ' + path)


prefix = 'preparation/final/'
measurement = load(prefix + 'measurement.json')
require(measurement['status'] == 'passed' and measurement['guest_runs'] == 0, 'transport-only preparation')
require([r['exit_code'] for r in measurement['commands']] == [1, 0, 0, 0, 0], 'five original preparation routes')
require(not any(measurement['unavailable_tools'].values()), 'empty toolchain PATH')
for command in measurement['commands']:
    for stream in ('stdout', 'stderr'):
        row = command[stream]
        bound(prefix + Path(row['path']).name, row)
for suffix in ['prepared', 'prepared пробел']:
    start = prefix + suffix + '/'
    prep = load(start + 'receipt.json')
    require(prep['status'] == 'prepared' and prep['kit_status'] == 'rehearsal', 'explicit rehearsal')
    require(len(prep['sources']) == 94 and sum(row['bytes'] for row in prep['sources'].values()) == 370544, 'complete frozen source')
    for name, row in prep['files'].items():
        bound(start + name, row)
    for row in prep['sources'].values():
        bound(start + row['copy'], row)
for name in ('compiler.dag', 'job.dag', 'package.json'):
    require(contents[prefix + 'prepared/' + name] == contents[prefix + 'prepared пробел/' + name], 'relocation: ' + name)

run = load('execution/receipt.json')
require(run['status'] == 'passed' and run['guest_runs'] == 1, 'one completed guest run')
require(run['binary_end'] == {k: run['binary'][k] for k in ('bytes', 'sha256')}, 'unchanged Joy')
require(run['inputs_start'] == run['inputs_end'], 'unchanged prepared input')
require(not any(run['unavailable_tools'].values()), 'execution without toolchain PATH')
for stream in ('stdout', 'stderr'):
    bound('execution/' + stream, run['commands'][0][stream])
require(run['commands'][0]['exit_code'] == 0 and len(run['commands']) == 1, 'single successful guest command')
compiler = contents[prefix + 'prepared/compiler.dag']
require(contents['execution/c3.dag'] == compiler, 'exact C3/C2 bytes')
current = load('execution/stdout')
baseline = load('execution/baseline.json')['execution']
admission = load('execution/admission.json')['package']
require(current['ok'] is True and current['execution']['compiler_job']['status'] == 'success', 'guest success')
require(current['execution']['program_particle'] == admission['compiler_particle'], 'compiler particle')
require(current['execution']['input_particle'] == admission['job_particle'], 'JOB particle')
require({k: v for k, v in current['execution'].items() if k != 'elapsed_micros'} ==
        {k: v for k, v in baseline['execution'].items() if k != 'elapsed_micros'}, 'every non-time execution field')
require(current['published_particle'] == baseline['published_particle'], 'published particle')
require(load('preparation/corrected/gates.json')['status'] == 'passed', 'corrected original gates')
require(load('preparation/portability/receipt.json')['status'] == 'passed', 'later local guard repetition')
require(load('preparation/portability/source-integrity.json')['protected_files_equal_base'] is True, 'unchanged source inputs')
print(json.dumps(dict(status='passed', members=len(contents), raw_bytes=sum(len(b) for b in contents.values()),
                     compiler=identity(compiler), job=identity(contents[prefix + 'prepared/job.dag']),
                     exact_c3_c2=True, non_time_execution_fields_equal=True,
                     scope='Retained historical rehearsal evidence; no new execution or matrix/proof acceptance'), sort_keys=True))
