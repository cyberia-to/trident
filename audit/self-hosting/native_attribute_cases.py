"""Fresh JOB1 attribute metadata, declaration-owned purity and diagnostics."""


def check(root, repo, run, package, execute, decode, record, observations, commands, zero, prior_program):
    def source(body='nox_noun_atom(7)', prefix='', declarations=''):
        return f'program sample {declarations} {prefix} fn main(input:Noun)->Noun{{{body}}}'.encode()

    functions = ' '.join(f'{"#[pure]" if i in [0, 7, 8] else ""} fn f{i}()->Field{{7}}' for i in range(9))
    positives = [
        ('attribute-plain', source(), 7),
        ('attribute-pure', source(prefix='#[pure]'), 7),
        ('attribute-repeat', source(prefix='#[pure] #[requires(false)] #[pure] #[ensures(false)]'), 7),
        ('attribute-empty', source(prefix='#[requires()] #[ensures()]'), 7),
        ('attribute-metadata', source(prefix='#[requires(missing(x) +)]'), 7),
        ('attribute-token-metadata', source(prefix='#[requires(Field XField module use _ > ^ /% [ { #)]'), 7),
        ('attribute-nested96', source(prefix='#[requires(' + '(' * 96 + 'unknown' + ')' * 96 + ')]'), 7),
        ('attribute-comment-pub', source(prefix='#[pure] // λ )\n #[requires(// ignored )\n)] pub'), 7),
        ('attribute-after-constant', source('nox_noun_atom(A)', '#[pure]', 'const A:Field=7'), 7),
        ('attribute-pure-reset', source('nox_noun_atom(pub_write_shadow())', declarations='#[pure] fn f()->Field{7} fn pub_write_shadow()->Field{7}'), 7),
        ('attribute-pure-helper', source('nox_noun_atom(helper())', '#[pure]', 'fn pub_write_shadow()->Field{7} fn helper()->Field{pub_write_shadow()}'), 7),
        ('attribute-replaced-impure', source('nox_noun_atom(f())', declarations='fn f()->Field{ram_read()} #[pure] fn f()->Field{7} fn ram_read()->Field{7}'), 7),
        ('attribute-replaced-pure', source('nox_noun_atom(f())', declarations='#[pure] fn f()->Field{7} fn f()->Field{ram_read()} fn ram_read()->Field{7}'), 7),
        ('attribute-near-name', source('nox_noun_atom(ram_read_x())', '#[pure]', 'fn ram_read_x()->Field{7}'), 7),
        ('attribute-pure-variable', source('let ram_read=7 nox_noun_atom(ram_read)', '#[pure]'), 7),
        ('attribute-pure-chunks', source('nox_noun_atom(f8())', declarations=functions), 7),
        ('attribute-pure-assert', source('assert(true) assert_eq(7,7) nox_noun_atom(7)', '#[pure]'), 7),
    ]
    identical = {name for name, _, _ in positives[:9]}
    plain_particle = None
    for name, content, expected in positives:
        directory, job = package(name, content, {'arena_nodes': 786432})
        program = directory / 'program.dag'
        compiled = execute(job, program)
        assert compiled['execution']['compiler_job']['status'] == 'success', (name, compiled)
        if name == 'attribute-plain':
            plain_particle = compiled['published_particle']
        if name in identical:
            assert compiled['published_particle'] == plain_particle, (name, compiled)
        output = directory / 'output.dag'
        executed = run(['run-artifact', program, '--input', zero, '-o', output])
        assert decode(output) == expected, (name, decode(output), expected)
        assert executed['execution']['program_particle'] == compiled['published_particle']
        observations.append({'case': name, 'source_hex': content.hex(), 'expected': expected,
                             'compiler_execution': compiled, 'program_execution': executed,
                             'program_bytes': len(program.read_bytes()), 'complete_output_checked': True,
                             'metadata_identity_checked': name in identical})

    negatives = [
        ('attribute-before-program', b'#[pure] program sample fn main()->Field{7}', 6),
        ('attribute-after-pub', source(prefix='pub #[pure]'), 6),
        ('attribute-in-body', source('#[pure] input'), 6),
        ('attribute-in-constant', source(declarations='const A:Field=#[pure] 7'), 6),
        ('attribute-on-constant', source(declarations='#[pure] const A:Field=7'), 2),
        ('attribute-on-struct', source(declarations='#[requires()] struct R{a:Field}'), 2),
        ('attribute-missing-close', source(prefix='#[pure'), 2),
        ('attribute-unbalanced', source(prefix='#[requires((x)]'), 2),
        ('attribute-extra-close', source(prefix='#[requires(x))]'), 2),
        ('attribute-unknown', source(prefix='#[requiresx()]'), 6),
        ('attribute-pure-args', source(prefix='#[pure()]'), 6),
        ('attribute-bare-contract', source(prefix='#[requires]'), 6),
        ('attribute-cfg', source(prefix='#[cfg(nox)]'), 6),
        ('attribute-intrinsic', source(prefix='#[intrinsic(assert)]'), 6),
        ('attribute-asm', source(prefix='#[requires(asm)]'), 6),
        ('attribute-invalid-token', source(prefix='#[requires(@)]'), 1),
        ('attribute-numeric-overflow', source(prefix='#[requires(18446744073709551616)]'), 1),
        ('attribute-nonascii', source(prefix='#[requires(λ)]'), 1),
        ('attribute-direct-io', source('nox_noun_atom(ram_read())', '#[pure]', 'fn ram_read()->Field{7}'), 5),
        ('attribute-replaced-error', source(declarations='#[pure] fn f()->Field{ram_read()} fn f()->Field{7} fn ram_read()->Field{7}'), 5),
        ('attribute-final-error', source(declarations='fn f()->Field{7} #[pure] fn f()->Field{ram_read()} fn ram_read()->Field{7}'), 5),
        ('attribute-pure-chunk-error', source(declarations=functions.replace('fn f8()->Field{7}', 'fn f8()->Field{ram_read()}') + ' fn ram_read()->Field{7}'), 5),
        ('attribute-untaken-error', source('if false{ram_read()} input', '#[pure]', 'fn ram_read()->Field{7}'), 5),
        ('attribute-empty-loop-error', source('for i in 0..0{ram_read()} input', '#[pure]', 'fn ram_read()->Field{7}'), 5),
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
