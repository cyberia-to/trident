"""Independent read-only source and historical evidence replay, not acceptance."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time
import traceback

BASE = Path(__file__).resolve().parent.parent
SOURCE = BASE/'whole-proof-final-review-v3'


def require(value, message):
    if not value:
        raise ValueError(message)


def identity(path):
    with path.open('rb') as stream:
        digest = hashlib.file_digest(stream, 'sha256').hexdigest()
    return dict(bytes=path.stat().st_size, sha256=digest)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-manifest-sha256', required=True)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    args.output.mkdir()
    result = dict(status='running', started_ns=time.time_ns(), command=[sys.executable, *sys.argv],
        driver=identity(Path(__file__)),
        cwd=str(Path.cwd()), python=dict(path=sys.executable, version=sys.version,
        identity=identity(Path(sys.executable))), scope='Source review, offline synthetic tests, '
        'and prior failed-attempt/reclamation replay only. No live v3 evaluation, final '
        'acceptance checker, watcher, compiler, prover or verifier is launched.')
    try:
        manifest_path = SOURCE/'sources.json'
        require(identity(manifest_path)['sha256'] == args.source_manifest_sha256, 'manifest identity')
        manifest = json.loads(manifest_path.read_text())
        require(all(identity(Path(path)) == expected for path, expected in manifest.items()), 'source identities')
        result.update(source_manifest=identity(manifest_path), sources=manifest)
        command = [sys.executable, '-B', '-W', 'error', '-m', 'unittest', '-v',
                   'test_checker', 'test_when_ready']
        with (args.output/'tests.stdout').open('xb') as out, (args.output/'tests.stderr').open('xb') as err:
            tested = subprocess.run(command, cwd=SOURCE, stdout=out, stderr=err, timeout=90,
                env={'PATH': '', 'LANG': 'C', 'PYTHONDONTWRITEBYTECODE': '1'})
        result['tests'] = dict(command=command, cwd=str(SOURCE), exit_code=tested.returncode)
        require(tested.returncode == 0, 'offline tests failed')
        sys.path.insert(0, str(SOURCE))
        import common
        import history
        import cases
        pins = common.v3_pins(BASE)
        result['source_pins_checked'] = len(pins)
        import importlib.util
        binding_path = BASE/'whole-proof-attacks-completion-v3/binding_context.py'
        binding_spec = importlib.util.spec_from_file_location('reviewed_binding_context', binding_path)
        binding = importlib.util.module_from_spec(binding_spec)
        binding_spec.loader.exec_module(binding)
        result['actual_prepared_binding_contexts'] = []
        for g in (1, 2):
            root = BASE/f'whole-proof-attacks-v3-c{g}'
            prepared = common.load(root/'preparation.json')
            for name, variant in prepared['generations'][str(g)]['variants'].items():
                coordinates = prepared['compiler_coordinates'][str(variant['compiler_generation'])]
                compiler = root/'inputs/frozen'/f"c{variant['compiler_generation']}.dag"
                job = root/f'inputs/c{g}/{name}/job.dag'
                common.same(compiler, variant['compiler'])
                common.same(job, variant['job'])
                actual = cases.admitted_context(coordinates, variant['admission'], compiler, job)
                parsed = binding.coordinates(compiler)
                require(all(parsed[key] == value for key, value in coordinates.items()), 'prepared coordinates')
                expected = binding.derive(parsed, variant['admission'], variant['admission']['job_particle'])
                require(actual == expected, 'independent admitted context agreement')
                result['actual_prepared_binding_contexts'].append(dict(generation=g, variant=name, context=actual))
        result['original_resource_replay'] = history.prior_schedule(BASE)
        result['original_command_resource_replays'] = []
        for g in (1, 2):
            root = BASE/f'whole-proof-attacks-v2-c{g}'
            suite = common.load(root/f'whole-c{g}/receipt.json')
            paths = [root/f'attempts/whole-c{g}-index/receipt.json']
            for row in [suite['controls'][1], *suite['rejections']]:
                paths.append(root/row['verification_receipt'])
                if row['recipe']:
                    paths.append(root/row['recipe']['construction_receipt'])
            for path in paths:
                row = common.load(path)
                common.C.command_receipt(path.parent, row['expected_exit'])
                cases.command_samples(path.parent, row)
                result['original_command_resource_replays'].append(dict(path=str(path), identity=identity(path)))
        result['reclamation_replay'] = history.transition(BASE, BASE/'whole-proof-attacks-completion-v3')
        require(len(result['original_command_resource_replays']) == 32, 'complete 32-command resource set')
        require(identity(manifest_path)['sha256'] == args.source_manifest_sha256, 'manifest unchanged')
        require(all(identity(Path(path)) == expected for path, expected in manifest.items()), 'sources unchanged')
        result['sources_unchanged'] = True
        result['status'] = 'passed-readonly-source-and-historical-replay'
    except BaseException:
        result.update(status='failed', error=traceback.format_exc())
        raise
    finally:
        result['ended_ns'] = time.time_ns()
        result['files'] = {p.name: identity(p) for p in args.output.iterdir() if p.is_file()}
        with (args.output/'receipt.json').open('x') as out:
            json.dump(result, out, indent=2)
            out.write('\n')


if __name__ == '__main__':
    main()
