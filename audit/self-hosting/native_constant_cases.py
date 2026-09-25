"""Fresh JOB1 acceptance for typed constants resolved inside the nox compiler."""


def check(root, repo, run, package, execute, decode, record, observations, commands, zero, prior_program):
    def source(declarations, body='nox_noun_atom(A)'):
        return f'program sample {declarations} fn main(input:Noun)->Noun{{{body}}}'.encode()

    cases = [
        ('constant-forward', source('const A:Field=B const B:Field=((7))'), 7),
        ('constant-after-entry', source('') + b' const A:Field=7', 7),
        ('constant-final', source('const A:Field=B const B:Field=2 const B:Field=7'), 7),
        ('constant-final-self-reference', source('const A:Field=A const A:Field=7'), 7),
        ('constant-final-type', source('pub const A:U32=7 const A:Field=4294967296'), 4294967296),
        ('constant-u32', source('const A:U32=B const B:U32=((0004294967295))', 'nox_noun_atom(as_field(A & as_u32(7)))'), 7),
        ('constant-field-p', source('const A:Field=18446744069414584321'), 0),
        ('constant-field-p-plus-one', source('const A:Field=18446744069414584322'), 1),
        ('constant-field-max', source('const A:Field=18446744073709551615'), 4294967294),
        ('constant-local-shadow', source('const A:Field=7', 'let mut A=A A=9 nox_noun_atom(A)'), 9),
        ('constant-parameter-shadow', source('const A:Field=7 fn f(A:Field)->Field{A}', 'nox_noun_atom(f(3))'), 3),
        ('constant-tuple-shadow', source('const A:Field=7', 'let(A,b)=(3,4) nox_noun_atom(A+b)'), 7),
        ('constant-index-normalized', source('const A:Field=18446744069414584321', 'nox_noun_atom([7][A])'), 7),
        ('constant-index-u32', source('const A:U32=0', 'nox_noun_atom([7][A])'), 7),
        ('constant-coverage', source('const A:Field=18446744069414584321', 'if A{return nox_noun_atom(7)}'), 7),
        ('constant-final-function', source('const A:Field=2 fn f()->Field{A} const A:Field=7', 'nox_noun_atom(f()+A)'), 14),
    ]
    name = 'common_prefix_' + 'a' * 256
    cases.append(('constant-long-names', source(f'const {name}a:Field=3 const {name}b:Field=7', f'nox_noun_atom({name}a*10+{name}b)'), 37))
    declarations = ' '.join(f'const C{i}:Field=C{i+1}' for i in range(16)) + ' const C16:Field=7'
    cases.append(('constant-alias-chain', source(declarations, 'nox_noun_atom(C0+C16)'), 14))
    cases.append(('constant-groups64', source('const A:Field=' + '(' * 64 + '7' + ')' * 64), 7))
    for name, content, expected in cases:
        directory, job = package(name, content, {'arena_nodes': 786432})
        program = directory / 'program.dag'
        compiled = execute(job, program)
        assert compiled['execution']['compiler_job']['status'] == 'success', (name, compiled)
        output = directory / 'output.dag'
        executed = run(['run-artifact', program, '--input', zero, '-o', output])
        assert decode(output) == expected, (name, decode(output), expected)
        assert executed['execution']['program_particle'] == compiled['published_particle']
        observations.append({'case': name, 'source_hex': content.hex(), 'expected': expected,
                             'compiler_execution': compiled, 'program_execution': executed,
                             'program_bytes': len(program.read_bytes()), 'complete_output_checked': True})

    negatives = [
        ('constant-unknown', source('const A:Field=missing', 'input'), 5),
        ('constant-replaced-unknown', source('const A:Field=missing const A:Field=7', 'input'), 5),
        ('constant-cycle', source('const A:Field=B const B:Field=A', 'input'), 5),
        ('constant-alias-type', source('const A:Field=B const B:U32=7', 'input'), 5),
        ('constant-u32-overflow', source('const A:U32=4294967296 const A:U32=7', 'input'), 5),
        ('constant-u32-before-reduction', source('const A:U32=18446744069414584321', 'input'), 5),
        ('constant-u64-overflow', source('const A:Field=18446744073709551616', 'input'), 1),
        ('constant-initializer-arithmetic', source('const A:Field=((7+2))', 'input'), 5),
        ('constant-initializer-call', source('const A:Field=f() fn f()->Field{7}', 'input'), 5),
        ('constant-missing-close', source('const A:Field=(7', 'input'), 2),
        ('constant-missing-colon', source('const A Field=7', 'input'), 2),
        ('constant-qualified', source('const A:Field=other.A', 'input'), 6),
        ('constant-assignment', source('const A:Field=7', 'A=9 input'), 5),
        ('constant-shadow-coverage', source('const A:Field=0', 'let A=1 if A{return input}'), 5),
        ('constant-unselected-type', source('const A:Field=0', 'if A{return input}else{false}'), 5),
        ('constant-extent-unsupported', source('const A:Field=1 fn f(a:[Field;A]){}', 'input'), 6),
        ('constant-loop-unsupported', source('const A:Field=1', 'for i in 0..A{} input'), 6),
        ('constant-groups65', source('const A:Field=' + '(' * 65 + '7' + ')' * 65), 7),
    ]
    for name, content, code in negatives:
        directory, job = package(name, content, {'arena_nodes': 786432})
        compiled = execute(job, directory / 'result.dag', emit='result')
        result = compiled['execution']['compiler_job']
        assert result['status'] == 'compile_error' and len(result['diagnostics']) == 1, (name, result)
        assert result['diagnostics'][0]['code'] == code, (name, result)
        protected = directory / 'protected.dag'
        protected.write_bytes(prior_program)
        execute(job, protected, expected=1, force=True)
        assert protected.read_bytes() == prior_program
        observations.append({'case': name, 'source_hex': content.hex(), 'diagnostics': result['diagnostics'],
                             'compiler_execution': compiled, 'previous_program_preserved': True})

    content = source('const A:Field=1', 'nox_noun_atom([7][A])')
    directory, job = package('constant-runtime-oob', content, {'arena_nodes': 786432})
    program = directory / 'program.dag'
    compiled = execute(job, program)
    assert compiled['execution']['compiler_job']['status'] == 'success'
    output = directory / 'output.dag'
    output.write_bytes(zero.read_bytes())
    run(['run-artifact', program, '--input', zero, '-o', output, '--force'], expected=1)
    assert commands[-1]['stderr'] == 'error: execution failed: InvZero\n'
    assert output.read_bytes() == zero.read_bytes()
    observations.append({'case': 'constant-runtime-oob', 'source_hex': content.hex(), 'compiler_execution': compiled,
                         'program_execution_error': commands[-1]['stderr'], 'previous_output_preserved': True})
