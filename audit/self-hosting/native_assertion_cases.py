"""Fresh JOB1 assertion semantics, runtime traps and protected publication."""


def check(root, repo, run, package, execute, decode, record, observations, commands, zero, prior_program):
    def source(body, declarations=''):
        return f'program sample {declarations} fn main(input:Noun)->Noun{{{body}}}'.encode()

    positives = [
        ('assert-success', source('assert(true) nox_noun_atom(7)'), 7),
        ('assert-eq-success', source('assert_eq(7,7) nox_noun_atom(7)'), 7),
        ('assert-eq-normalized', source('assert_eq(18446744069414584321,0) nox_noun_atom(7)'), 7),
        ('assert-unit-local', source('let mut a=assert(true) let b=a a=assert_eq(0,0) b nox_noun_atom(7)'), 7),
        ('assert-unit-tuple', source('let(a,b)=(assert(true),assert_eq(0,0)) a b nox_noun_atom(7)'), 7),
        ('assert-untaken', source('if false{assert(false)}else{nox_noun_atom(7)}'), 7),
        ('assert-shadow-function', source('nox_noun_atom(assert(false))', 'fn assert(c:Bool)->Field{7}'), 7),
        ('assert-final-function', source('nox_noun_atom(assert(5))', 'fn assert(c:Bool)->Field{1} fn assert(c:Field)->Field{c+2}'), 7),
        ('assert-eq-shadow-function', source('nox_noun_atom(assert_eq(false,false))', 'fn assert_eq(a:Bool,b:Bool)->Field{if a==b{7}else{9}}'), 7),
        ('assert-local-callable', source('let assert=7 let assert_eq=9 assert(true) assert_eq(0,0) nox_noun_atom(assert+assert_eq)'), 16),
        ('assert-constant-callable', source('assert(true) nox_noun_atom(assert)', 'const assert:Field=9'), 9),
        ('assert-loop-coverage', source('nox_noun_atom(f(false))', 'fn f(c:Bool)->Field{for i in 0..1{if c{assert(false)}else{return 7}}}'), 7),
    ]
    for name, content, expected in positives:
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
        ('assert-field-type', source('assert(0) input')),
        ('assert-missing-argument', source('assert() input')),
        ('assert-extra-argument', source('assert(true,false) input')),
        ('assert-eq-bool-type', source('assert_eq(true,true) input')),
        ('assert-eq-u32-type', source('assert_eq(as_u32(0),as_u32(0)) input')),
        ('assert-eq-missing-argument', source('assert_eq(0) input')),
        ('assert-direct-unreachable', source('assert(false) input')),
        ('assert-computed-no-coverage', source('assert(1==0)')),
        ('assert-eq-no-coverage', source('return assert_eq(0,1)')),
        ('assert-let-no-coverage', source('let a=assert(false)')),
        ('assert-empty-loop-no-coverage', source('for i in 0..0{assert(false)}')),
        ('assert-ordinary-no-coverage', source('assert(false)', 'fn assert(c:Bool){}')),
    ]
    for name, content in negatives:
        directory, job = package(name, content, {'arena_nodes': 786432})
        compiled = execute(job, directory / 'result.dag', emit='result')
        result = compiled['execution']['compiler_job']
        assert result['status'] == 'compile_error' and len(result['diagnostics']) == 1, (name, result)
        assert result['diagnostics'][0]['code'] == 5, (name, result)
        protected = directory / 'protected.dag'
        protected.write_bytes(prior_program)
        execute(job, protected, expected=1, force=True)
        assert protected.read_bytes() == prior_program
        observations.append({'case': name, 'source_hex': content.hex(), 'diagnostics': result['diagnostics'],
                             'compiler_execution': compiled, 'previous_program_preserved': True})

    inverse = 'as_field(as_u32(4294967296))'
    axis = 'nox_noun_as_field(nox_noun_head(nox_noun_atom(0)))'
    failures = [
        ('assert-false-tail', source('assert(false)'), 'InvZero'),
        ('assert-false-return', source('return assert((false))'), 'InvZero'),
        ('assert-false-loop', source('for i in 0..1{assert(false)}'), 'InvZero'),
        ('assert-defensive-branch', source('if true{assert(false)} input'), 'InvZero'),
        ('assert-eq-failure', source('assert_eq(0,1) input'), 'InvZero'),
        ('assert-computed-failure', source('assert(1==0) input'), 'InvZero'),
        ('assert-initializer-failure', source('let a=assert(false) input'), 'InvZero'),
        ('assert-nonterminal-loop-tail', source('for i in 0..1{if true{assert(false)}else{false}}'), 'InvZero'),
        ('assert-eq-left-trap', source(f'assert_eq({inverse},{axis}) input'), 'InvZero'),
        ('assert-eq-right-trap', source(f'assert_eq({axis},{inverse}) input'), 'AxisError'),
    ]
    for name, content, error in failures:
        directory, job = package(name, content, {'arena_nodes': 786432})
        program = directory / 'program.dag'
        compiled = execute(job, program)
        assert compiled['execution']['compiler_job']['status'] == 'success', (name, compiled)
        output = directory / 'output.dag'
        output.write_bytes(zero.read_bytes())
        run(['run-artifact', program, '--input', zero, '-o', output, '--force'], expected=1)
        assert commands[-1]['stderr'] == f'error: execution failed: {error}\n'
        assert output.read_bytes() == zero.read_bytes()
        observations.append({'case': name, 'source_hex': content.hex(), 'compiler_execution': compiled,
                             'program_execution_error': commands[-1]['stderr'], 'previous_output_preserved': True})
