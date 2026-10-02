"""Installed Joy: C1 emits a small compiler, which compiles fresh literal sources.

This exercises generated compiler profiles, not the C2/self-hosting milestone.
The output JSON is updated after every command, including failed commands.
Its persistent sibling directory retains exact sources, JOB/RES/ART files and
seed outputs. --compiler selects the producer ART1 unchanged; raw seed oracles
remain separate reference-only comparisons.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time
from native_compiler_selection import CompilerSelection

P = 18446744069414584321
ART1, RES1 = 0x41525431, 0x52455331
TEMPLATE = b'program p fn main()->Field{7}'
HOST = ['--budget', '100000000', '--frames', '65536', '--time-ms', '300000',
        '--arena-nodes', '3145728']
LIMITS = dict(source_bytes=4096, modules=128, diagnostics=16,
              sequence_length=4096, validation_visits=1000000,
              artifact_bytes=16777216, artifact_nodes=196608,
              artifact_depth=4096, reductions=100000000, arena_nodes=3145728,
              evaluator_frames=65536)


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def metadata_git(value):
    if value is None:
        found = shutil.which('git')
        require(found is not None, 'Git metadata requires --git or TRIDENT_AUDIT_GIT when git is absent from PATH')
        value = Path(found).absolute()
    path = Path(value)
    require(path.is_absolute(), '--git/TRIDENT_AUDIT_GIT must be an absolute executable path')
    path = path.resolve(strict=True)
    require(path.is_file() and os.access(path, os.X_OK), 'Git metadata path must name an executable file')
    return path


def decode(path):
    """Independent noun reader, after the producing Joy admitted the container."""
    data = path.read_bytes()
    require(data[:8] == b'NOXDAG01' and len(data) >= 44, 'DAG header')
    root, count = data[8:40], int.from_bytes(data[40:44], 'little')
    cursor, nodes, last = 44, {}, None
    for _ in range(count):
        require(cursor + 33 <= len(data), 'DAG entry header')
        particle, width = data[cursor:cursor + 32], data[cursor + 32]
        cursor += 33
        require(width in (8, 64) and cursor + width <= len(data), 'DAG payload')
        payload = data[cursor:cursor + width]
        cursor += width
        require(particle not in nodes, 'duplicate DAG entry')
        if width == 8:
            value = int.from_bytes(payload, 'little')
            require(value < P, 'noncanonical atom')
        else:
            value = nodes[payload[:32]], nodes[payload[32:]]
        nodes[particle], last = value, particle
    require(cursor == len(data) and last == root, 'DAG root/trailing bytes')
    return nodes[root]


def fields(value, tag, count):
    require(isinstance(value, tuple) and value[0] == tag, f'record {tag:#x}')
    tail, result = value[1], []
    for _ in range(count):
        require(isinstance(tail, tuple), 'record arity')
        result.append(tail[0])
        tail = tail[1]
    require(tail == 0, 'record terminator')
    return result


def profile(path, expected):
    machine, incoming, outgoing, formula = fields(decode(path), ART1, 4)
    require((machine, incoming, outgoing) == (0, expected, expected), 'ART1 profile')
    return formula


def check_result(path, job, program):
    identity, status, artifact = fields(decode(path), RES1, 3)
    raw = job.read_bytes()[8:40]
    words = [int.from_bytes(raw[i:i + 8], 'little') for i in range(0, 32, 8)]
    require(identity == ((words[0], words[1]), (words[2], words[3])), 'all four JOB identity limbs')
    require(status == 0 and artifact == decode(program), 'complete RES1 payload/ART1')


class Acceptance:
    def __init__(self, binary, output, repo, selection, git):
        self.binary, self.output, self.repo = binary, output, repo
        self.selection = selection
        self.git = git
        output.parent.mkdir(parents=True, exist_ok=True)
        self.root = Path(tempfile.mkdtemp(prefix=output.stem + '-files-', dir=output.parent))
        self.report = dict(schema='trident/generated-compiler-profile/v1', status='running',
                           scope='generated bounded literal compiler; C2/self-hosting remains open',
                           artifact_directory=str(self.root), binary=str(binary),
                           commands=[], observations=[], source_snapshots=[],
                           metadata_git=dict(path=str(git), sha256=sha(git)))
        self.flush()

    def flush(self):
        self.report.update(self.selection.describe())
        self.selection.write_report(self.report)

    def run(self, arguments, expected=0, executable=None, reference_only=False):
        self.selection.before(arguments, reference_only)
        command = [str(executable or self.binary), *map(str, arguments)]
        row = dict(command=command, cwd=str(self.repo), expected_exit=expected,
                   exit_code=None, stdout='', stderr='', reference_only=reference_only)
        self.report['commands'].append(row)
        self.flush()
        started = time.monotonic_ns()
        try:
            result = subprocess.run(command, cwd=self.repo, capture_output=True,
                                    text=True, timeout=360, check=False)
            row.update(exit_code=result.returncode, stdout=result.stdout, stderr=result.stderr)
        except (OSError, subprocess.TimeoutExpired) as error:
            row['process_error'] = str(error)
            for name in ('stdout', 'stderr'):
                value = getattr(error, name, None)
                if value:
                    row[name] = value.decode('utf-8', errors='replace') if isinstance(value, bytes) else value
            raise
        finally:
            row['elapsed_nanoseconds'] = time.monotonic_ns() - started
            self.flush()
        value = json.loads(result.stdout) if result.stdout.startswith('{') else None
        self.selection.after(arguments, value, expected)
        require(result.returncode == expected, row)
        return value if value is not None else row

    def observe(self, name, **values):
        self.report['observations'].append(dict(case=name, **values))
        self.flush()

    def check_metadata_git(self):
        require(sha(self.git) == self.report['metadata_git']['sha256'], 'Git metadata executable changed')

    def metadata(self):
        self.check_metadata_git()
        self.report['revision'] = self.run(['rev-parse', 'HEAD'], executable=self.git)['stdout'].strip()
        self.report['working_tree'] = self.run(['status', '--porcelain=v1'], executable=self.git)['stdout']
        self.check_metadata_git()

    def snapshot(self, path, category):
        relative = path.relative_to(self.repo)
        destination = self.root / 'source-snapshot' / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(path.read_bytes())
        self.report['source_snapshots'].append(dict(source=str(path), snapshot=str(destination),
            category=category, sha256=sha(destination), bytes=destination.stat().st_size))

    def package(self, name, compiler, sources, entry, requested=0, function='main'):
        directory = self.root / name
        directory.mkdir()
        modules = []
        for index, (owner, source) in enumerate(sorted(sources.items())):
            filename = f'{index}.tri'
            (directory / filename).write_bytes(source)
            modules.append(dict(logical_path=owner, file=filename,
                                origin_name='generated-profile-acceptance', origin_version='1'))
        manifest = dict(version=1, entry_module=entry, entry_function=function, modules=modules,
            options=dict(target=0, input_profile=requested, output_profile=requested,
                         optimization=0, cfg_flags=[]), limits=LIMITS)
        path = directory / 'package.json'
        path.write_text(json.dumps(manifest, indent=2) + '\n')
        job = directory / 'job.dag'
        self.run(['pack-job', '--compiler', compiler, '--manifest', path, '-o', job, *HOST])
        return directory, job

    def execute(self, compiler, job, output, emit='result', expected=0, force=False, reference_only=False):
        return self.run(['run-artifact', compiler, '--input', job, '--emit', emit,
                         '-o', output, *HOST, *(['--force'] if force else [])], expected, reference_only=reference_only)

    def compile(self, name, source, entry, requested):
        directory, job = self.package(name, self.compiler, {entry: source}, entry, requested)
        result, program = directory / 'result.dag', directory / 'program.dag'
        report = self.execute(self.compiler, job, result)
        require(report['execution']['compiler_job']['status'] == 'success', report)
        self.execute(self.compiler, job, program, 'program')
        profile(program, requested)
        check_result(result, job, program)
        self.observe(name, requested_profile=requested, program=str(program), job=str(job),
                     result=str(result), all_four_identity_limbs_checked=True)
        return program, job

    def rejected(self, name, compiler, job, message, emit='program'):
        output = self.root / (name + '-protected.dag')
        previous = b'previous output: ' + name.encode()
        output.write_bytes(previous)
        result = self.execute(compiler, job, output, emit, 1, True)
        require(message in result['stderr'] and not result['stdout'], result)
        require(output.read_bytes() == previous, 'failed execution replaced previous output')
        self.observe(name, rejection=message, previous_output_preserved=True)

    def capture_files(self):
        self.report['files'] = []
        for path in sorted(self.root.rglob('*')):
            if not path.is_file():
                continue
            data = path.read_bytes()
            item = dict(path=str(path.relative_to(self.root)), bytes=len(data),
                        sha256=hashlib.sha256(data).hexdigest())
            if data[:8] == b'NOXDAG01' and len(data) >= 44:
                item.update(particle=data[8:40].hex(), dag_entries=int.from_bytes(data[40:44], 'little'))
            self.report['files'].append(item)


def exercise(a):
    fixture = a.repo / 'tests/fixtures/native_generated_literal_compiler.tri'
    source = fixture.read_bytes()
    require(len(source) <= 4096, 'literal compiler source capacity')
    require(len(TEMPLATE) == 29 and TEMPLATE[27:28] == b'7', 'literal grammar fixture')
    a.snapshot(fixture, 'generated compiler source')
    a.snapshot(Path(__file__).resolve(), 'acceptance runner')
    pending, seen = ([] if a.selection.provided else [a.repo / 'compiler/nox/main.tri']), set()
    while pending:
        path = pending.pop()
        if path in seen:
            continue
        seen.add(path)
        a.snapshot(path, 'C1 disk source closure; installed binary pinned separately')
        for owner in re.findall(rb'^use\s+([A-Za-z_][A-Za-z_0-9.]*)', path.read_bytes(), re.M):
            pending.append(a.repo / 'lib' / (owner.decode().replace('.', '/') + '.tri'))
    a.metadata()
    a.compiler = a.selection.select(a.root / 'c1.dag', lambda path:
        a.run(['build', a.repo / 'compiler/nox/main.tri', '--emit', 'artifact',
               '--artifact-profile', 'compiler-job', '-o', path]))
    profile(a.compiler, 1)
    # Only the transport atom comes from existing codec vectors; no compiler or
    # generated program is imported from those fixtures.
    vectors = a.repo.parent / 'joy/cli/tests/compiler_vectors.json'
    zero = a.root / 'zero.dag'
    zero.write_bytes(bytes.fromhex(json.loads(vectors.read_text())['files']['zero']))
    require(decode(zero) == 0, 'zero input fixture')
    a.report['zero_fixture'] = dict(path=str(vectors), sha256=sha(vectors))
    generated, producer_job = a.compile('generate-compiler', source, 'literal_compiler', 1)

    programs = []
    for digit in (0, 3, 7, 9):
        fresh = TEMPLATE[:27] + str(digit).encode() + TEMPLATE[28:]
        directory, job = a.package(f'literal-{digit}', generated, {'p': fresh}, 'p')
        result, program = directory / 'result.dag', directory / 'program.dag'
        report = a.execute(generated, job, result)
        require(report['execution']['compiler_job']['status'] == 'success', report)
        a.execute(generated, job, program, 'program')
        profile(program, 0)
        check_result(result, job, program)
        output = directory / 'value.dag'
        a.execute(program, zero, output)
        require(decode(output) == digit, 'compiled literal value')
        seed_source = directory / 'seed.tri'
        seed_source.write_bytes(fresh.replace(b'fn main()', b'fn result()', 1)
            + b' fn main(input:Noun)->Noun{nox_noun_atom(result())}')
        seed, seed_output = directory / 'seed.dag', directory / 'seed-value.dag'
        a.run(['build', seed_source, '--emit', 'artifact', '--artifact-profile', 'raw', '-o', seed], reference_only=True)
        a.execute(seed, zero, seed_output, reference_only=True)
        require(output.read_bytes() == seed_output.read_bytes(), 'complete seed output differs')
        programs.append(program.read_bytes())
        a.observe(f'literal-{digit}', expected=digit, complete_seed_output_equal=True,
                  all_four_identity_limbs_checked=True, program=str(program), result=str(result))
    require(len(set(programs)) == 4, 'different source digits emitted identical programs')

    # Each failure runs a freshly packed, otherwise admitted JOB1.
    for name, bad, extra, requested in [
        ('wrong-prefix', TEMPLATE.replace(b'program p', b'program q'), {}, 0),
        ('wrong-suffix', TEMPLATE[:-1] + b']', {}, 0),
        ('wrong-digit', TEMPLATE[:27] + b'a}', {}, 0),
        ('wrong-length', TEMPLATE + b'\n', {}, 0),
        ('multiple-modules', TEMPLATE, {'z': b'module z'}, 0),
        ('mini-profile-guard', TEMPLATE, {}, 1),
    ]:
        _, job = a.package(name, generated, {'p': bad, **extra}, 'p', requested)
        a.rejected(name, generated, job, 'execution failed: InvZero')

    for imported in (False, True):
        name = 'scalar-profile-import' if imported else 'scalar-profile-local'
        text = b'program p ' + (b'use dep ' if imported else b'') + b'fn main()->Field{missing}'
        sources = {'p': text, **({'dep': b'module dep pub fn f()->Field{7}'} if imported else {})}
        directory, job = a.package(name, a.compiler, sources, 'p', 1)
        result = a.execute(a.compiler, job, directory / 'result.dag')
        outcome = result['execution']['compiler_job']
        require(outcome['status'] == 'compile_error' and len(outcome['diagnostics']) == 1, outcome)
        diagnostic = outcome['diagnostics'][0]
        start = text.index(b'main')
        require((diagnostic['code'], diagnostic['module_index'], diagnostic['start_byte'],
                 diagnostic['end_byte']) == (3, sorted(sources).index('p'), start, start + 4), diagnostic)
        a.rejected(name, a.compiler, job, 'guest compilation failed: code3')

    raw, _ = a.compile('generate-raw-copy', source, 'literal_compiler', 0)
    require(profile(raw, 0) == profile(generated, 1), 'profile changed executable formula')
    directory, job = a.package('raw-cannot-publish', generated, {'p': TEMPLATE}, 'p')
    a.rejected('raw-cannot-publish', raw, job, '--emit program requires compiler profile(1,1)')
    protected = directory / 'protected-job.dag'
    protected.write_bytes(b'previous packed job')
    error = a.run(['pack-job', '--compiler', raw, '--manifest', directory / 'package.json',
                   '-o', protected, '--force', *HOST], 1)
    require('pack-job requires compiler profile(1,1)' in error['stderr'], error)
    require(protected.read_bytes() == b'previous packed job', 'raw pack replaced previous output')
    a.observe('raw-pack-rejected', previous_output_preserved=True)
    a.rejected('wrong-compiler-binding', generated, producer_job, 'compiler identity mismatch')

    identity_source = b'program identity_compiler fn main(input:Noun)->Noun{input}'
    identity, _ = a.compile('generate-malformed-result-compiler', identity_source, 'identity_compiler', 1)
    _, job = a.package('malformed-result', identity, {'p': TEMPLATE}, 'p')
    a.rejected('malformed-result', identity, job, 'compiler result: record tag: expected 0x52455331')

    # An explicitly faulty source variant lets Joy check returned ART1 profiles,
    # independently of the miniature compiler's own requested-profile guard.
    variant = source
    for guard in [
        b'    assert_eq(nox_noun_as_field(nox_noun_head(profiles)), 0)\n',
        b'    assert_eq(nox_noun_as_field(nox_noun_head(nox_noun_tail(profiles))), 0)\n',
    ]:
        require(variant.count(guard) == 1, 'profile guard variant boundary')
        variant = variant.replace(guard, b'', 1)
    mismatched, _ = a.compile('generate-wrong-profile-compiler', variant, 'literal_compiler', 1)
    _, job = a.package('returned-profile-mismatch', mismatched, {'p': TEMPLATE}, 'p', 1)
    a.rejected('returned-profile-mismatch', mismatched, job, 'compiler result: generated artifact profile mismatch')
    require(not [p for p in a.root.rglob('*') if p.is_file() and p.name.startswith('.')], 'publication staging leak')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--joy', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--compiler', type=Path, help='use this producer ART1 unchanged; retain separate reference-only oracles')
    parser.add_argument('--git', type=Path, default=os.environ.get('TRIDENT_AUDIT_GIT'),
                        help='absolute Git metadata executable; defaults to TRIDENT_AUDIT_GIT, then PATH lookup')
    args = parser.parse_args()
    try:
        git = metadata_git(args.git)
    except (OSError, AssertionError) as failure:
        parser.error(str(failure))
    selection = CompilerSelection(args, parser)
    a = Acceptance(selection.binary, args.output.resolve(), Path(__file__).resolve().parents[2], selection, git)
    error = None
    try:
        a.report['binary_sha256_start'] = sha(a.binary)
        a.flush()
        exercise(a)
        a.report['status'] = 'passed'
    except BaseException as failure:
        error = failure
        a.report.update(status='failed', failure=f'{type(failure).__name__}: {failure}')
    finally:
        try:
            a.report['binary_sha256_end'] = sha(a.binary)
            require(a.report['binary_sha256_start'] == a.report['binary_sha256_end'], 'installed Joy changed')
            selection.check()
            a.check_metadata_git()
            for source in a.report['source_snapshots']:
                require(sha(Path(source['source'])) == source['sha256'], f"source changed: {source['source']}")
        except BaseException as failure:
            a.report.update(status='failed', final_verification_failure=f'{type(failure).__name__}: {failure}')
            error = error or failure
        try:
            a.capture_files()
        except BaseException as failure:
            a.report.update(status='failed', file_capture_failure=f'{type(failure).__name__}: {failure}')
            error = error or failure
        try:
            a.flush()
        finally:
            selection.close()
    print(json.dumps(dict(status=a.report['status'], receipt=str(a.output),
                          commands=len(a.report['commands']), observations=len(a.report['observations']))))
    if error is not None:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
