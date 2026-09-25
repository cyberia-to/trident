"""Execute compiler-checked direct TIR through the pinned Trisha backend and VM."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile


def rust_ops(debug):
    # Controlled Rust Debug fixtures: preserve escaped strings, make owned
    # String and Vec constructors explicit. No project source is executed here.
    pieces = re.split(r'("(?:[^"\\]|\\.)*")', debug)
    return ''.join(piece + '.to_string()' if i % 2 else piece.replace('[', 'vec![')
                   for i, piece in enumerate(pieces))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[2]
    root = repo.parent
    env = os.environ.copy()
    env.setdefault('CARGO_TARGET_DIR', str(root / 'target'))
    commands = []

    def run(command, cwd=repo):
        p = subprocess.run(command, cwd=cwd, env=env, capture_output=True, text=True)
        row = dict(command=list(map(str, command)), cwd=str(cwd), exit_code=p.returncode,
                   stdout=p.stdout, stderr=p.stderr)
        commands.append(row)
        assert p.returncode == 0, row
        assert not re.search(r'^warning:', p.stderr, re.M), row
        return p

    with tempfile.TemporaryDirectory(prefix='trident-callable-tir-') as directory:
        temporary = Path(directory)
        exported = temporary / 'checked.json'
        generated = temporary / 'fixtures.rs'
        env['TRIDENT_CALLABLE_TIR_FIXTURES'] = str(exported)
        run(['cargo', 'test', '--release', '--locked', '--offline', '--lib',
             'typecheck::tests::callable_execution'])
        fixtures = json.loads(exported.read_text())
        generated.write_text("fn fixtures() -> Vec<(&'static str, Vec<TIROp>, Vec<u64>)> { vec![\n" +
                             ',\n'.join(f'({json.dumps(f["case"])}, {rust_ops(f["tir"])}, vec!{json.dumps(f["expected"])})'
                                         for f in fixtures) + '\n] }\n')
        env['CALLABLE_TIR_FIXTURES_RS'] = str(generated)
        manifest = '[package]\nname="callable-tir-audit"\nversion="0.0.0"\nedition="2021"\n[workspace]\n'
        manifest += '[[bin]]\nname="callable-tir-audit"\npath=' + json.dumps(str(Path(__file__).with_name('callable-tir.rs'))) + '\n'
        manifest += '[dependencies]\n'
        for name, path in [('trident-lang', repo), ('trisha-rs', root / 'trisha/rs')]:
            manifest += name + '={path=' + json.dumps(str(path)) + '}\n'
        manifest += 'triton-vm="=7.0.0"\nserde_json="1"\n[patch.crates-io]\n'
        for name in ['triton-vm', 'twenty-first', 'triton-air', 'triton-isa', 'triton-constraint-circuit',
                     'triton-constraint-builder', 'tasm-lib', 'tasm-object-derive']:
            manifest += name + '={path=' + json.dumps(str(root / 'trisha/.vendor' / name)) + '}\n'
        (temporary / 'Cargo.toml').write_text(manifest)
        shutil.copyfile(root / 'trisha/Cargo.lock', temporary / 'Cargo.lock')
        run(['cargo', 'generate-lockfile', '--offline'], temporary)
        execution = run(['cargo', 'run', '--release', '--locked', '--offline'], temporary)
        observations = [json.loads(line) for line in execution.stdout.splitlines()]
        assert len(observations) == len(fixtures)
        assert all(o['actual'] == o['expected'] for o in observations)
        receipt = {'schema': 'trident/direct-callable-tir/v1', 'kind': 'local-development',
                   'environment': {k: env[k] for k in ['CARGO_TARGET_DIR', 'TRIDENT_CALLABLE_TIR_FIXTURES', 'CALLABLE_TIR_FIXTURES_RS']},
                   'commands': commands, 'manifest': manifest,
                   'lockfile_sha256': hashlib.sha256((temporary / 'Cargo.lock').read_bytes()).hexdigest(),
                   'lockfile': (temporary / 'Cargo.lock').read_text(),
                   'fixtures': fixtures, 'observations': observations}
    args.output.write_text(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps({'direct_tir_executions': len(observations)}))


if __name__ == '__main__':
    main()
