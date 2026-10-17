"""Retain build-only example evidence against the frozen S1 source inventory."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def require(value, message):
    if not value:
        raise ValueError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('worktree', 'output', 'inventory', 'checker'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    root, output = args.worktree.resolve(), args.output.resolve()
    inventory, checker = args.inventory.resolve(), args.checker.resolve()
    output.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ, CARGO_TARGET_DIR='../target-lexer')
    report = dict(schema='trident/example-build/v1', status='running',
                  scope='Build-only example targets; no example execution or additional test passes',
                  cwd=str(root), start=datetime.now(timezone.utc).isoformat(),
                  script_sha256=sha(__file__), commands=[], test_count_delta=0,
                  inventory=dict(path=str(inventory), sha256=sha(inventory)),
                  checker=dict(path=str(checker), sha256_start=sha(checker)))

    def flush():
        temporary = output / 'receipt.next.json'
        temporary.write_text(json.dumps(report, indent=2) + '\n')
        temporary.replace(output / 'receipt.json')

    def run(argv, name, timeout=300):
        row = dict(argv=list(map(str, argv)), cwd=str(root),
                   env={'CARGO_TARGET_DIR': env['CARGO_TARGET_DIR']},
                   log=name, status='running', exit_code=None)
        report['commands'].append(row)
        flush()
        started = time.monotonic_ns()
        with (output / name).open('xb') as stream:
            try:
                result = subprocess.run(row['argv'], cwd=root, env=env, stdout=stream,
                                        stderr=subprocess.STDOUT, timeout=timeout, check=False)
                row.update(status='completed', exit_code=result.returncode)
            finally:
                row['elapsed_nanoseconds'] = time.monotonic_ns() - started
                row['sha256'] = sha(output / name)
                row['bytes'] = (output / name).stat().st_size
                flush()
        require(row['exit_code'] == 0, 'command failed: ' + name)
        return (output / name).read_bytes()

    def sources(document):
        values = {}
        for name, module in document['modules'].items():
            path = Path(module['path'])
            require(not path.is_absolute() and '..' not in path.parts, 'unsafe inventory path')
            data = (root / path).read_bytes()
            require(len(data) == module['source_bytes'], 'inventory source length differs')
            values[name] = dict(path=path.as_posix(), bytes=len(data),
                                sha256=hashlib.sha256(data).hexdigest(),
                                source_blake3=module['source_blake3'])
        return values

    try:
        report['execution_revision'] = run(['git', 'rev-parse', 'HEAD'], 'revision.log').decode().strip()
        require(not run(['git', 'status', '--porcelain', '--untracked-files=all'], 'status-start.log').strip(),
                'worktree is not clean')
        report['cargo_version'] = run(['cargo', '-Vv'], 'cargo-version.log').decode()
        report['rust_version'] = run(['rustc', '-vV'], 'rust-version.log').decode()
        tools = {}
        for name in ('cargo', 'rustc'):
            path = Path(run(['rustup', 'which', name], name + '-path.log').decode().strip())
            tools[name] = dict(path=str(path), sha256_start=sha(path))
        report['tools'] = tools
        report['build_environment'] = {name: env.get(name) for name in (
            'RUSTUP_TOOLCHAIN', 'RUSTC', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER',
            'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'CARGO_BUILD_TARGET')}
        document = json.loads(inventory.read_text())
        require(document['module_count'] == 94 and len(document['modules']) == 94, 'closure module count')
        before = sources(document)
        indexed = run(['git', 'ls-files', '-z'], 'tracked-paths.log').decode().split('\0')
        inputs = {path: sha(root / path) for path in indexed if path and (
            Path(path).suffix in ('.rs', '.tri') or Path(path).name in ('Cargo.toml', 'Cargo.lock') or
            path.startswith('.cargo/'))}
        captured = dict(inventory_sha256=sha(inventory), sources=before, tracked_build_inputs=inputs)
        (output / 'inputs.json').write_text(json.dumps(captured, indent=2) + '\n')
        report['inputs_sha256'] = sha(output / 'inputs.json')
        run([checker, '--root', root, '--entry', 'compiler/nox/main.tri',
             '--output', inventory, '--check'], 'inventory-check.log')
        metadata = json.loads(run(['cargo', 'metadata', '--format-version', '1', '--no-deps',
                                   '--locked', '--offline'], 'cargo-metadata.json'))
        package = next(p for p in metadata['packages'] if Path(p['manifest_path']) == root / 'Cargo.toml')
        examples = [target for target in package['targets'] if target['kind'] == ['example']]
        require(len(examples) == 21 and all(not t.get('required-features') for t in examples),
                'unexpected example target set or feature-gated target')
        report['examples'] = [dict(name=t['name'], path=Path(t['src_path']).relative_to(root).as_posix())
                              for t in examples]
        raw = run(['cargo', 'build', '--release', '--locked', '--offline', '--examples'], 'build.log', 1800)
        text = re.sub(r'\x1b\[[0-9;]*m', '', raw.decode())
        report['rust_warnings'] = len(re.findall(r'^\s*warning(?:\[.*?\])?:', text, re.M))
        require(report['rust_warnings'] == 0, 'Rust build warning')
        require(sources(document) == before and sha(inventory) == report['inventory']['sha256'],
                'source or inventory changed')
        require(all(sha(root / path) == identity for path, identity in inputs.items()), 'build input changed')
        report['checker']['sha256_end'] = sha(checker)
        require(report['checker']['sha256_end'] == report['checker']['sha256_start'], 'checker changed')
        for tool in tools.values():
            tool['sha256_end'] = sha(tool['path'])
            require(tool['sha256_end'] == tool['sha256_start'], 'build tool changed')
        require(not run(['git', 'status', '--porcelain', '--untracked-files=all'], 'status-end.log').strip(),
                'worktree changed')
        report.update(status='passed', source_inputs_unchanged=True, example_count=len(examples))
    except BaseException as error:
        report.update(status='failed', error=f'{type(error).__name__}: {error}')
    report['end'] = datetime.now(timezone.utc).isoformat()
    flush()
    print(json.dumps({k: report.get(k) for k in ('status', 'example_count', 'rust_warnings', 'error')}))
    return 0 if report['status'] == 'passed' else 1


if __name__ == '__main__':
    raise SystemExit(main())
