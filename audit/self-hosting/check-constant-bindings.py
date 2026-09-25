"""Execute qualified names and frozen import bindings through installed Joy."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile


def tree(values, capacity=None):
    if capacity is None:
        capacity = 1 << (max(len(values), 1) - 1).bit_length()
    if capacity == 1:
        return values[0] if values else 0
    half = capacity // 2
    return tree(values[:half], half), tree(values[half:], half)


def seq(values):
    return 0x53455131, (len(values), tree(values))


def byte_string(value):
    value = value.encode() if isinstance(value, str) else value
    words = [int.from_bytes(value[i:i + 4], 'little') for i in range(0, len(value), 4)]
    return 0x42595431, (len(value), tree(words))


def fields(values):
    value = values[-1]
    for item in reversed(values[:-1]):
        value = item, value
    return value


def expression(value):
    if isinstance(value, tuple):
        return f'nox_noun_pair({expression(value[0])},{expression(value[1])})'
    return f'nox_noun_atom({value})'


def cases():
    source = '// ж😀\nstd // prefix\n . config . VALUE + 7'
    first, member = source.encode().index(b'std'), source.encode().index(b'VALUE')
    end = member + 5
    yield 'comments', 'name', byte_string(source), fields([0, first, first + 3, first, end, member, end, 3, source.encode().index(b'+'), byte_string('std.config')])
    source = 'a' * 255 + '.' + 'B' * 300
    yield 'long-member', 'name', byte_string(source), fields([0, 0, 255, 0, 556, 256, 556, 0, 556, byte_string('a' * 255)])
    yield 'incomplete-path', 'name', byte_string('a.'), fields([2, 2, 2, 0, 1, 2, 2, 41, 1, byte_string('a')])
    modules = [
        ('alpha.same', 'padding A B 11 4294967295', [[8, 9, 0, 11, 0, 12, 14], [10, 11, 3, 4294967295, 0, 15, 25]]),
        ('beta.same', 'A C 22 33', [[0, 1, 0, 22, 1, 4, 6], [2, 3, 0, 33, 1, 7, 9]]),
        ('hidden.same', 'A 99', [[0, 1, 0, 99, 2, 2, 4]]),
    ]
    queries = ['same.A', 'same.B', 'same.C', 'alpha.same.A', 'beta.same.A', 'hidden.same.A', 'A', 'same.MISSING']
    expected = [[3, 1, 0, 22, 1, 4, 6], [2, 0, 3, 4294967295, 0, 15, 25], [4, 1, 0, 33, 1, 7, 9], [1, 0, 0, 11, 0, 12, 14], [3, 1, 0, 22, 1, 4, 6], None, None, None]

    def imports(name, modules, targets, queries, expected):
        items = [(byte_string(owner), (byte_string(source), seq([fields(row) for row in bindings]))) for owner, source, bindings in modules]
        return name, 'imports', (seq(items), (seq(targets), seq([byte_string(q) for q in queries]))), (0, seq([fields(row) if row is not None else 0 for row in expected]))

    yield imports('per-symbol-order', modules, [0, 1], queries, expected)
    repeated = expected.copy()
    repeated[0] = [1, 0, 0, 11, 0, 12, 14]
    yield imports('repeated-uses', modules, [0, 1, 0], queries, repeated)
    yield imports('no-ambient-bindings', modules, [], queries, [None] * len(queries))
    owner = 'a.' * 127 + 'b'
    modules = [(owner, 'X 7', [[0, 1, 0, 7, 0, 2, 3]])]
    value = [1, 0, 0, 7, 0, 2, 3]
    yield imports('owner255', modules, [0], [owner + '.X', 'b.X', 'c.X', 'a.b.X'], [value, value, None, None])
    long = 'X' * 300
    modules = [('original', 'N 18446744069414584321', [[0, 1, 0, 0, 0, 2, 22]]), ('middle', long + ' ALIAS', [[0, 300, 0, 0, 0, 2, 22], [301, 306, 0, 0, 0, 2, 22]])]
    yield imports('foreign-literal-origin', modules, [1], ['middle.' + long, 'middle.ALIAS', 'original.N', 'middle.' + 'X' * 299 + 'Y'], [[2, 1, 0, 0, 0, 2, 22], [3, 1, 0, 0, 0, 2, 22], None, None])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--joy', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[2]
    binary = args.joy.resolve()
    spec = importlib.util.spec_from_file_location('native', Path(__file__).with_name('run-native-compiler.py'))
    native = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(native)
    commands, observations = [], []
    sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    limits = ['--budget', '100000000', '--frames', '65536', '--arena-nodes', '786432', '--time-ms', '60000']

    def run(args):
        cmd = [str(binary), *map(str, args)]
        result = subprocess.run(cmd, cwd=repo, capture_output=True, text=True)
        row = {'command': cmd, 'exit_code': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr}
        commands.append(row)
        assert result.returncode == 0, row
        return json.loads(result.stdout) if result.stdout.startswith('{') else None

    with tempfile.TemporaryDirectory(prefix='native-constant-bindings-') as temporary:
        root = Path(temporary)
        artifacts = {}
        for name, source, profile in [('compiler', 'compiler/nox/main.tri', 'compiler-job'), ('name', 'tests/fixtures/native_qualified_name.tri', 'raw'), ('imports', 'tests/fixtures/native_constant_imports.tri', 'raw')]:
            output = root / (name + '.dag')
            run(['build', repo / source, '--emit', 'artifact', '--artifact-profile', profile, '-o', output])
            artifacts[name] = {'sha256': sha(output), 'particle': output.read_bytes()[8:40].hex()}
        zero = root / 'zero.dag'
        zero.write_bytes(bytes.fromhex(json.loads((repo.parent / 'joy/cli/tests/compiler_vectors.json').read_text())['files']['zero']))
        for name, component, value, expected in cases():
            source = root / (name + '.tri')
            source.write_text('program fixture_input fn main(input:Noun)->Noun{' + expression(value) + '}')
            producer, input_file, output = [root / (name + suffix) for suffix in ['-producer.dag', '-input.dag', '-output.dag']]
            run(['build', source, '--emit', 'artifact', '--artifact-profile', 'raw', '-o', producer])
            run(['run-artifact', producer, '--input', zero, '-o', input_file, *limits])
            assert native.decode(input_file) == value
            execution = run(['run-artifact', root / (component + '.dag'), '--input', input_file, '-o', output, *limits])
            actual = native.decode(output)
            assert actual == expected, (name, actual, expected)
            observations.append({'case': name, 'component': component, 'complete_input_checked': True, 'complete_output_checked': True, 'input_sha256': sha(input_file), 'output_sha256': sha(output), 'execution': execution})
    args.output.write_text(json.dumps({'schema': 'trident/constant-bindings-cli/v1', 'kind': 'local-component-development', 'binary_sha256': sha(binary), 'artifacts': artifacts, 'commands': commands, 'observations': observations}, indent=2) + '\n')
    print(json.dumps({'commands': len(commands), 'observations': len(observations)}))


if __name__ == '__main__':
    main()
