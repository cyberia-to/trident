#!/usr/bin/env python3
"""Actual provided-C2 source-size cases; no build, replacement or fallback."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[3]
HOST = ['--arena-nodes', '1000000000', '--budget', '20000000000', '--frames', '65536',
        '--time-ms', '3600000', '--validation-visits', '16777216',
        '--resident-nodes', '3145728', '--collection-work', '10000000000']
LIMITS = dict(modules=128, diagnostics=16, sequence_length=65536, validation_visits=16777216,
              artifact_bytes=16777216, artifact_nodes=196608, artifact_depth=4096,
              reductions=20000000000, arena_nodes=1000000000, evaluator_frames=65536)
PROGRAM_HOST = ['--budget', '1000000', '--arena-nodes', '196608', '--frames', '65536',
                '--time-ms', '30000']


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def padded(size):
    prefix, suffix = b'program sample //', b'\nfn main()->Field{13}'
    require(size >= len(prefix) + len(suffix), 'padding target too small')
    return prefix + b' ' * (size - len(prefix) - len(suffix)) + suffix


def cases():
    prefix = b'program sample //' + b' ' * 4096 + b'\n'
    return [('baseline', b'program sample fn main()->Field{13}', 0),
            ('valid4096', padded(4096), 0), ('valid65536', padded(65536), 0),
            ('diagnostic-after-comment', prefix + b'fn main()->Field{missing}', 5),
            ('exact65536', b'\xff' + bytes(65535), 1),
            ('excess65537', b'\xff' + bytes(65536), 7)]


def atomic_output(path):
    """Independent expected-value reader after Joy validated/published the DAG."""
    data = path.read_bytes()
    require(len(data) == 85 and data[:8] == b'NOXDAG01', 'expected a single canonical atom DAG')
    require(int.from_bytes(data[40:44], 'little') == 1 and data[8:40] == data[44:76]
            and data[76] == 8, 'expected one root atom entry')
    value = int.from_bytes(data[77:85], 'little')
    require(value < 18446744069414584321, 'noncanonical atom')
    return value


def bind(args):
    require(sha(args.producer_receipt) == args.producer_receipt_sha256, 'producer receipt changed')
    producer = json.loads(args.producer_receipt.read_text())
    require(producer['status'] == 'compiler-returned' and producer['published_kind'] == 'program',
            'producer did not publish a compiler')
    require(producer['execution']['execution']['compiler_job']['status'] == 'success', 'producer failed')
    require(producer['execution']['ok'] is True, 'producer execution failed')
    require(sha(args.compiler) == args.compiler_sha256 == producer['result_sha256'],
            'provided compiler is not actual C2')
    source = Path(producer['artifact_directory']) / 'result.dag'
    require(args.compiler.read_bytes() == source.read_bytes(), 'provided C2 differs from producer artifact')
    require(args.compiler.read_bytes()[8:40].hex() ==
            producer['execution']['published_particle'] ==
            producer['execution']['execution']['compiler_job']['compiled_particle'], 'C2 particle differs')
    require(sha(args.joy) == args.joy_sha256 == producer['binary_sha256'] == producer['binary_sha256_end'],
            'Joy changed from full-source run')
    require(producer['host_flags'] == HOST, 'compiler profile differs from full-source run')
    return producer


def require_guest_rejection(row):
    require(row['exit_code'] == 1 and 'guest compilation failed:' in row['stderr'],
            'forced publication failed for a reason other than guest compilation')


def record_end_hash(report, key, path):
    try:
        report[key] = sha(path)
    except OSError as error:
        report[key] = None
        report['status'] = 'failed'
        report.setdefault('identity_errors', {})[key] = str(error)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ['joy', 'compiler', 'producer-receipt', 'output']:
        parser.add_argument('--' + key, type=Path, required=True)
    for key in ['joy-sha256', 'compiler-sha256', 'producer-receipt-sha256']:
        parser.add_argument('--' + key, required=True)
    parser.add_argument('--case', action='append', choices=[name for name, _, _ in cases()])
    args = parser.parse_args()
    selected = args.case or [name for name, _, _ in cases()]
    if 'baseline' not in selected or len(selected) != len(set(selected)):
        parser.error('case selection must include baseline and contain no duplicates')
    if args.output.exists() or args.output.is_symlink():
        parser.error('choose a fresh receipt path; existing evidence is preserved')
    for key in ['joy', 'compiler', 'producer_receipt', 'output']:
        setattr(args, key, getattr(args, key).resolve())
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('x') as output:
        report = dict(schema='trident/source-scale-compacting/v1', status='running',
            scope='provided-C2 source-size corpus only; full C2 corpus/fixed point/platform acceptance remain open',
            invocation=sys.argv, script_sha256=sha(Path(__file__)), binary=str(args.joy),
            compiler=str(args.compiler), compiler_sha256_start=sha(args.compiler),
            binary_sha256_start=sha(args.joy), producer_receipt=str(args.producer_receipt),
            producer_receipt_sha256=args.producer_receipt_sha256, host_flags=HOST,
            program_host_flags=PROGRAM_HOST, limits=LIMITS, selected_cases=selected,
            complete_corpus=len(selected) == len(cases()), commands=[], observations=[])

        def flush():
            output.seek(0)
            output.truncate()
            json.dump(report, output, indent=2)
            output.write('\n')
            output.flush()

        def run(arguments, expected=0):
            require(sha(args.compiler) == args.compiler_sha256 and sha(args.joy) == args.joy_sha256,
                    'tool/compiler changed before command')
            row = dict(command=[str(args.joy), *map(str, arguments)], cwd=str(ROOT),
                       expected_exit=expected, exit_code=None, stdout='', stderr='', status='running')
            report['commands'].append(row)
            flush()
            started = time.monotonic_ns()
            result = subprocess.run(row['command'], cwd=ROOT, capture_output=True, text=True,
                                    check=False, timeout=3660)
            row.update(exit_code=result.returncode, stdout=result.stdout, stderr=result.stderr,
                       status='completed', elapsed_nanoseconds=time.monotonic_ns() - started)
            flush()
            require(sha(args.compiler) == args.compiler_sha256 and sha(args.joy) == args.joy_sha256,
                    'tool/compiler changed during command')
            require(result.returncode == expected, 'unexpected Joy exit; see retained command')
            return json.loads(result.stdout) if result.stdout.lstrip().startswith('{') else None

        try:
            producer = bind(args)
            root = Path(tempfile.mkdtemp(prefix=args.output.stem + '-files-', dir=args.output.parent))
            report['artifact_directory'] = str(root)
            report['compiler_particle'] = producer['execution']['published_particle']
            vectors_path = ROOT.parent / 'joy/cli/tests/compiler_vectors.json'
            vectors = json.loads(vectors_path.read_text())
            zero = root / 'zero.dag'
            zero.write_bytes(bytes.fromhex(vectors['files']['zero']))
            require(atomic_output(zero) == 0, 'independent zero input')
            report['input_fixture'] = dict(source=str(vectors_path), source_sha256=sha(vectors_path),
                                          retained=str(zero), sha256=sha(zero))
            baseline = None
            for name, source, code in cases():
                if name not in selected:
                    continue
                directory = root / name
                directory.mkdir()
                source_file = directory / 'sample.tri'
                source_file.write_bytes(source)
                limits = dict(LIMITS, source_bytes=len(source))
                manifest = dict(version=1, entry_module='sample', entry_function='main',
                    modules=[dict(logical_path='sample', file='sample.tri', origin_name='source-capacity', origin_version='1')],
                    options=dict(target=0, input_profile=0, output_profile=0, optimization=0, cfg_flags=[]), limits=limits)
                package = directory / 'package.json'
                package.write_text(json.dumps(manifest, indent=2) + '\n')
                job, result, program = [directory / name for name in ['job.dag', 'result.dag', 'program.dag']]
                row = dict(case=name, status='running', source_bytes=len(source), source_sha256=sha(source_file),
                           source_file=str(source_file), manifest=manifest, expected_diagnostic=code)
                report['observations'].append(row)
                admission = run(['pack-job', '--compiler', args.compiler, '--manifest', package, '-o', job, *HOST])
                require(admission['ok'] is True and admission['package']['compiler_particle'] == report['compiler_particle'],
                        'provided-C2 profile/admission binding')
                row.update(admission=admission, job_sha256=sha(job))
                if code:
                    execution = run(['run-artifact', args.compiler, '--input', job, '--emit', 'result', '-o', result, *HOST])
                else:
                    execution = run(['run-artifact', args.compiler, '--input', job, '--emit', 'program', '-o', program, *HOST])
                ran = execution['execution']
                compiled = ran['compiler_job']
                require(execution['ok'] is True and ran['trace_mode'] == 'none' and
                        ran['program_particle'] == report['compiler_particle'] and
                        ran['input_particle'] == admission['package']['job_particle'], 'actual compiler execution binding')
                require(compiled['limits'] == limits and compiled['options'] == manifest['options'], 'returned options/limits')
                require(0 < ran['charged_reductions'] <= LIMITS['reductions'], 'successful compiler gas')
                row['compiler_execution'] = execution
                if code:
                    diagnostics = compiled['diagnostics']
                    require(compiled['status'] == 'compile_error' and compiled['compiled_particle'] is None,
                            'expected guest compile rejection')
                    require(len(diagnostics) == 1 and diagnostics[0]['code'] == code, 'wrong diagnostic')
                    diagnostic = diagnostics[0]
                    require(diagnostic['module_index'] == 0 and 0 <= diagnostic['start_byte'] <=
                            diagnostic['end_byte'] <= len(source), 'diagnostic source bounds')
                    if name == 'diagnostic-after-comment':
                        start, end = diagnostic['start_byte'], diagnostic['end_byte']
                        require(start > 4096 and source[start:end] == b'missing', 'long diagnostic original span')
                    if name == 'excess65537':
                        require(diagnostic['start_byte'] == diagnostic['end_byte'] == 0, 'excess capacity span')
                    require(baseline is not None, 'baseline unavailable for output protection')
                    program.write_bytes(baseline)
                    run(['run-artifact', args.compiler, '--input', job, '--emit', 'program', '-o', program, '--force', *HOST], 1)
                    require_guest_rejection(report['commands'][-1])
                    require(program.read_bytes() == baseline, 'negative overwrote previous program')
                    row.update(diagnostics=diagnostics, result_sha256=sha(result), previous_program_preserved=True)
                else:
                    require(compiled['status'] == 'success' and compiled['diagnostics'] == [], 'compilation failed')
                    require(execution['published_particle'] == compiled['compiled_particle'] == program.read_bytes()[8:40].hex(),
                            'published program particle')
                    if baseline is None:
                        baseline = program.read_bytes()
                    require(program.read_bytes() == baseline, 'source padding changed artifact bytes')
                    value = directory / 'value.dag'
                    runtime = run(['run-artifact', program, '--input', zero, '-o', value, *PROGRAM_HOST])
                    require(runtime['ok'] is True and runtime['execution']['program_particle'] ==
                            compiled['compiled_particle'] and atomic_output(value) == 13, 'independent execution result is not 13')
                    row.update(program_sha256=sha(program), runtime_execution=runtime, expected=13,
                               baseline_artifact_bytes_equal=True, output_sha256=sha(value))
                row['status'] = 'passed'
                flush()
            bind(args)
            report['status'] = 'passed'
        except BaseException as error:
            report.update(status='failed', error=dict(kind=type(error).__name__, message=str(error)))
        finally:
            record_end_hash(report, 'compiler_sha256_end', args.compiler)
            record_end_hash(report, 'binary_sha256_end', args.joy)
            flush()
    print(json.dumps(dict(status=report['status'], receipt=str(args.output))))
    return 0 if report['status'] == 'passed' else 1


if __name__ == '__main__':
    sys.exit(main())
