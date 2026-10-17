"""Run existing source-transport guards on native Python and retain both modes."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
TESTS = ROOT / 'scripts/test_prepare_selfhost_source.py'


def require(value, message):
    if not value:
        raise ValueError(message)


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n', encoding='utf-8')


def identity(path):
    raw = path.read_bytes()
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def worker(output):
    spec = importlib.util.spec_from_file_location('source_transport_tests', TESTS)
    tests = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(tests)
    suite = unittest.defaultTestLoader.loadTestsFromModule(tests)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    write(output, dict(tests=result.testsRun, failures=len(result.failures), errors=len(result.errors),
                       expected_failures=len(result.expectedFailures), unexpected_successes=len(result.unexpectedSuccesses),
                       skips=[dict(test=str(test), reason=reason) for test, reason in result.skipped],
                       successful=result.wasSuccessful(), optimize=sys.flags.optimize))
    return 0 if result.wasSuccessful() and result.testsRun == 21 else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--target')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--worker', action='store_true')
    args = parser.parse_args()
    if args.worker:
        return worker(args.output)
    args.output.mkdir(parents=True, exist_ok=False)
    machine = platform.machine().lower()
    arch = {'arm64': 'aarch64', 'aarch64': 'aarch64', 'amd64': 'x86_64', 'x86_64': 'x86_64'}.get(machine)
    suffix = {'darwin': 'apple-darwin', 'linux': 'unknown-linux-gnu', 'win32': 'pc-windows-msvc'}.get(sys.platform)
    report = dict(schema='trident/native-source-transport/v1', status='running', target=args.target, machine=machine,
                  native_target=f'{arch}-{suffix}', python=sys.version, executable=sys.executable,
                  commands=[], ci_origin=None)
    try:
        revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
        report.update(revision=revision, implementation=identity(Path(__file__)), tests=identity(TESTS),
                      preparer=identity(ROOT / 'scripts/prepare-selfhost-source.py'))
        if os.environ.get('GITHUB_ACTIONS') == 'true':
            report['ci_origin'] = {key: os.environ.get('GITHUB_' + key.upper())
                                   for key in ('repository', 'run_id', 'run_attempt', 'event_name', 'workflow_ref')}
            report['ci_origin']['head_sha'] = os.environ.get('TRANSPORT_HEAD_SHA')
            require(all(report['ci_origin'].values()) and report['ci_origin']['head_sha'] == revision,
                    'exact CI run and source identity')
        require(report['native_target'] == args.target, 'native host and Python architecture')
        child_env = dict(os.environ)
        child_env.pop('PYTHONOPTIMIZE', None)
        for name, flags in (('ordinary', []), ('optimized', ['-O'])):
            argv = [sys.executable, '-B', '-W', 'error', *flags, str(Path(__file__).resolve()),
                    '--worker', '--output', str((args.output / (name + '.json')).resolve())]
            with (args.output / (name + '.stdout')).open('xb') as stdout, (args.output / (name + '.stderr')).open('xb') as stderr:
                result = subprocess.run(argv, cwd=ROOT, env=child_env, stdout=stdout, stderr=stderr)
            report['commands'].append(dict(argv=argv, cwd=str(ROOT), exit_code=result.returncode))
        require(all(row['exit_code'] == 0 for row in report['commands']), 'transport guards failed; original logs retained')
        for name, optimize in (('ordinary', 0), ('optimized', 1)):
            outcome = json.loads((args.output / (name + '.json')).read_bytes())
            require(outcome['tests'] == 21 and outcome['optimize'] == optimize and outcome['successful'] is True
                    and all(outcome[k] == 0 for k in ('failures', 'errors', 'expected_failures', 'unexpected_successes')),
                    'complete test result and distinct interpreter modes')
        report['status'] = 'passed'
    except BaseException as error:
        report.update(status='failed', error=f'{type(error).__name__}: {error}')
        raise
    finally:
        report['files'] = {p.name: identity(p) for p in sorted(args.output.iterdir()) if p.is_file()}
        write(args.output / 'receipt.json', report)
    print(json.dumps(dict(status=report['status'], target=args.target, revision=revision)))
    return 0


if __name__ == '__main__':
    sys.exit(main())
