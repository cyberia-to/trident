"""Fresh JOB1 -> guest imports -> RES1/ART1 -> installed Joy acceptance."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile


def cases():
    base = {
        'a.value': '// ж\nmodule a.value pub const X:Field=3 pub const I:U32=7 pub const P:Field=18446744069414584321',
        'z.bridge': 'module z.bridge use a.value pub const X:Field=value.X pub const I:U32=a.value.I pub const P:Field=value.P',
    }
    def positive(name, entry, deps, value):
        return dict(case=name, sources={**deps, 'sample': entry}, value=value)
    for name, body, value in [('full-short', 'bridge.X+z.bridge.X', 6),
            ('u32', 'as_field(bridge.I)', 7), ('foreign-index', '[7][bridge.P]', 7),
            ('foreign-condition', 'if bridge.P{return 7}', 7)]:
        yield positive(name, f'program sample use z.bridge fn main()->Field{{{body}}}', base, value)
    yield positive('local-alias', 'program sample use z.bridge const X:Field=bridge.X const Y:Field=X fn main()->Field{Y}', base, 3)
    deps = {'a.same': 'module a.same pub const X:Field=3 pub const Y:Field=4',
            'z.same': 'module z.same pub const X:Field=7 const Y:Field=9'}
    for name, uses, value in [('last-public', 'use a.same use z.same', 74),
            ('reversed', 'use z.same use a.same', 34),
            ('repeated', 'use a.same use z.same use a.same', 34)]:
        yield positive(name, f'program sample {uses} fn main()->Field{{same.X*10+same.Y}}', deps, value)
    yield positive('constant-root', 'program sample use a.same use z.same const same:Field=99 fn main()->Field{same.X}', deps, 7)
    yield positive('lexical-root', 'program sample use a.same struct S{X:Field} fn main()->Field{let same=S{X:9} same.X}', deps, 9)
    yield positive('final-binding', 'program sample use dep fn main()->Field{dep.Y}',
        {'dep': 'module dep const X:Field=3 pub const X:Field=7 pub const Y:Field=X'}, 7)
    member = 'member_' + 'a' * 280
    yield positive('long-commented', f'program sample use a fn main()->Field{{a //ж\n . {member}}}',
        {'a': f'// different byte offsets\nmodule a pub const {member}:Field=9'}, 9)
    case = positive('unused-invalid', 'program sample use a fn main()->Field{a.X}',
        {'a': 'module a pub const X:Field=7', 'b': b'\xff'}, 7)
    case['seed_omit'] = ['b']
    yield case

    def negative(name, entry, deps, owner, span, code, seed_reject=True):
        return dict(case=name, sources={**deps, 'sample': entry}, owner=owner,
            span=span, code=code, seed_reject=seed_reject)
    for name, body in [('transitive-short', 'value.X'), ('transitive-full', 'a.value.X')]:
        yield negative(name, f'program sample use z.bridge fn main()->Field{{{body}}}', base, 'sample', body, 5)
    for name, declaration in [('unused-invalid-declaration', 'const BAD:Field=missing pub const X:Field=7'),
            ('replaced-invalid', 'pub const X:Field=missing pub const X:Field=7')]:
        yield negative(name, 'program sample use dep fn main()->Field{dep.X}',
            {'dep': f'module dep {declaration}'}, 'dep', 'missing', 5)
    for name, declaration in [('withdrawn', 'pub const X:Field=3 const X:Field=7'),
            ('private', 'const X:Field=7')]:
        yield negative(name, 'program sample use dep fn main()->Field{dep.X}',
            {'dep': f'module dep {declaration}'}, 'sample', 'dep.X', 5)
    yield negative('type-mismatch', 'program sample use z.bridge const X:U32=bridge.X fn main()->Field{7}', base, 'sample', 'bridge.X', 5)
    for name, body in [('call', 'bridge.X()'), ('constructor', 'bridge.X{}')]:
        yield negative(name, f'program sample use z.bridge fn main()->Field{{{body}}}', base, 'sample', 'bridge.X', 5 if name == 'call' else 6)
    yield negative('cycle', 'program sample use a fn main()->Field{7}', {'a': 'module a use z', 'z': 'module z use a'}, 'z', 'use a', 4)
    yield negative('self-cycle', 'program sample use sample fn main()->Field{7}', {}, 'sample', 'use sample', 4)
    yield negative('missing', 'program sample use a fn main()->Field{7}', {'a': 'module a use z'}, 'a', 'use z', 3)
    yield negative('wrong-owner', 'program sample use a fn main()->Field{7}', {'a': 'module wrong'}, 'a', 'wrong', 3)
    yield negative('wrong-kind', 'program sample use a fn main()->Field{7}', {'a': 'program a fn main()->Field{7}'}, 'a', 'program', 3)
    yield negative('dependency-utf8', 'program sample use a fn main()->Field{7}', {'a': b'module a\xff'}, 'a', b'\xff', 1)
    for name, declaration in [('function', 'fn f()->Field{7}'),
            ('attribute', '#[pure] fn f()->Field{7}')]:
        yield positive(name, 'program sample use a fn main()->Field{7}', {'a': f'module a {declaration}'}, 7)
    for name, declaration, span in [('struct', 'struct S{x:Field}', 'struct')]:
        yield negative(name, 'program sample use a fn main()->Field{7}', {'a': f'module a {declaration}'}, 'a', span, 6, False)


def main(case_provider=cases):
    parser = argparse.ArgumentParser()
    parser.add_argument('--joy', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--case', action='append', help='run only the named acceptance case (repeatable)')
    args = parser.parse_args()
    selected = list(case_provider())
    if args.case:
        requested = set(args.case)
        assert requested <= {case['case'] for case in selected}, requested
        selected = [case for case in selected if case['case'] in requested]
    repo = Path(__file__).resolve().parents[2]
    binary = args.joy.resolve()
    sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
    binary_sha = sha(binary)
    spec = importlib.util.spec_from_file_location('native', Path(__file__).with_name('run-native-compiler.py'))
    native = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(native)
    commands, observations = [], []
    identities = {}
    host = ['--budget', '100000000', '--frames', '65536', '--time-ms', '60000', '--arena-nodes', '786432']

    def run(arguments, expected=0):
        command = [str(binary), *map(str, arguments)]
        result = subprocess.run(command, cwd=repo, capture_output=True, text=True)
        record = dict(command=command, exit_code=result.returncode, stdout=result.stdout, stderr=result.stderr)
        commands.append(record)
        assert result.returncode == expected, record
        return json.loads(result.stdout) if result.stdout.startswith('{') else None

    with tempfile.TemporaryDirectory(prefix='trident-guest-constants-') as temp:
        root = Path(temp)
        zero = root / 'zero.dag'
        zero.write_bytes(bytes.fromhex(json.loads((repo.parent / 'joy/cli/tests/compiler_vectors.json').read_text())['files']['zero']))
        compiler = root / 'compiler.dag'
        run(['build', repo / 'compiler/nox/main.tri', '--emit', 'artifact', '--artifact-profile', 'compiler-job', '-o', compiler])
        compiler_info = dict(sha256=sha(compiler), particle=compiler.read_bytes()[8:40].hex(), dag_entries=int.from_bytes(compiler.read_bytes()[40:44], 'little'))
        for case in selected:
            directory = root / case['case']
            directory.mkdir()
            sources = dict(sorted((k, v.encode() if isinstance(v, str) else v) for k, v in case['sources'].items()))
            files = []
            for index, (name, source) in enumerate(sources.items()):
                (directory / f'{index}.tri').write_bytes(source)
                files.append(dict(logical_path=name, file=f'{index}.tri', origin_name='acceptance', origin_version='1'))
            manifest = dict(version=1, entry_module='sample', entry_function='main', modules=files,
                options=dict(target=0, input_profile=0, output_profile=0, optimization=0, cfg_flags=['explicit']),
                limits=dict(source_bytes=4096, modules=128, diagnostics=16, sequence_length=4096,
                    validation_visits=1000000, artifact_bytes=16777216, artifact_nodes=196608, artifact_depth=4096,
                    reductions=100000000, arena_nodes=786432, evaluator_frames=65536))
            path = directory / 'package.json'
            path.write_text(json.dumps(manifest))
            job, program = directory / 'job.dag', directory / 'program.dag'
            admission = run(['pack-job', '--manifest', path, '--compiler', compiler, '-o', job, *host])
            report = run(['run-artifact', compiler, '--input', job, '--emit', 'program' if 'value' in case else 'result', '-o', program, *host])
            result = report['execution']['compiler_job']
            observation = dict(case=case['case'], sources={k:v.hex() for k,v in sources.items()}, admission=admission, compiler_execution=report)
            seed_root = directory / 'seed'
            seed_root.mkdir()
            for name, source in sources.items():
                if name in case.get('seed_omit', []):
                    continue
                target = seed_root / (name.replace('.', '/') + '.tri')
                target.parent.mkdir(parents=True, exist_ok=True)
                if name == 'sample':
                    source = source.replace(b'fn main()->Field', b'fn result()->Field', 1) + b' fn main(input:Noun)->Noun{nox_noun_atom(result())}'
                target.write_bytes(source)
            oracle = directory / 'seed.dag'
            seed_cmd = ['build', seed_root / 'sample.tri', '--emit', 'artifact', '--artifact-profile', 'raw', '-o', oracle]
            if 'value' in case:
                assert result['status'] == 'success', (case['case'], result)
                output = directory / 'value.dag'
                execution = run(['run-artifact', program, '--input', zero, '-o', output, *host])
                assert native.decode(output) == case['value'], case['case']
                run(seed_cmd)
                expected = directory / 'seed-value.dag'
                run(['run-artifact', oracle, '--input', zero, '-o', expected, *host])
                assert output.read_bytes() == expected.read_bytes(), case['case']
                if 'identity' in case:
                    previous = identities.setdefault(case['identity'], program.read_bytes())
                    assert previous == program.read_bytes(), (case['case'], 'artifact identity drift')
                observation.update(expected=case['value'], program_sha256=sha(program), runtime_execution=execution, complete_seed_output_equal=True)
            else:
                assert result['status'] == 'compile_error' and len(result['diagnostics']) == 1, (case['case'], result)
                diagnostic = result['diagnostics'][0]
                assert diagnostic['code'] == case['code'] and list(sources)[diagnostic['module_index']] == case['owner'], (case['case'], diagnostic)
                span = case['span'].encode() if isinstance(case['span'], str) else case['span']
                assert sources[case['owner']][diagnostic['start_byte']:diagnostic['end_byte']] == span, (case['case'], diagnostic)
                if case['code'] == 4:
                    assert diagnostic['message'] == 'cyclic import', diagnostic
                protected = directory / 'protected.dag'
                protected.write_bytes(b'previous output')
                run(['run-artifact', compiler, '--input', job, '--emit', 'program', '-o', protected, '--force', *host], 1)
                assert protected.read_bytes() == b'previous output'
                if case['seed_reject']:
                    run(seed_cmd, 1)
                observation.update(diagnostics=result['diagnostics'], previous_output_preserved=True)
            observations.append(observation)
    assert sha(binary) == binary_sha, 'installed Joy changed during acceptance'
    args.output.write_text(json.dumps(dict(binary_sha256=binary_sha, compiler=compiler_info, commands=commands, observations=observations), indent=2) + '\n')
    print(json.dumps(dict(commands=len(commands), observations=len(observations), compiler=compiler_info)))


if __name__ == '__main__':
    main()
