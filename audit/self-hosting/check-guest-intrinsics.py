"""Installed C1: exact intrinsic identities, owner ABIs and reachable lowering.

The shared import runner compares complete guest/seed outputs, original owner
spans and rejection output preservation. Full VM sources, trapping argument
order, namespace byte boundaries and sparse IDs also have native Rust tests.
"""
import importlib.util
import json
from pathlib import Path


def cases():
    def positive(name, body, declarations, value, structured=False):
        entry = 'fn main(input:Noun)->Noun' if structured else 'fn main()->Field'
        return dict(case=name, sources={
            'ext.ops': 'module ext.ops ' + declarations,
            'sample': f'program sample use ext.ops {entry}{{{body}}}'},
            value=value, structured=structured)

    def negative(name, body, declarations, owner, span, code=5, seed_reject=True):
        return dict(case=name, sources={
            'ext.ops': 'module ext.ops ' + declarations,
            'sample': f'program sample use ext.ops fn main()->Field{{{body}}}'},
            owner=owner, span=span, code=code, seed_reject=seed_reject)

    subtract = '#[intrinsic(sub)] pub fn pick(a:Field,b:Field)->Field'
    for name, body in [('short-alias', 'ops.pick(9,2)'),
                       ('full-alias', 'ext.ops.pick(9,2)'),
                       ('lexical-root', 'let ops=99 ops.pick(9,2)')]:
        yield positive(name, body, subtract, 7)
    yield positive('ordinary-replaces-intrinsic', 'ops.pick(9,2)',
        subtract + ' pub fn pick(a:Field,b:Field)->Field{a+b}', 11)
    yield positive('intrinsic-replaces-ordinary', 'ops.pick(9,2)',
        'pub fn pick(a:Field)->Field{a+1} ' + subtract, 7)
    yield positive('private-intrinsic-wrapper', 'ops.wrapper()',
        '#[intrinsic(sub)] fn private(a:Field,b:Field)->Field '
        'pub fn wrapper()->Field{private(9,2)}', 7)
    yield positive('final-attribute-wins', 'ops.pick(9,2)',
        '#[intrinsic(unknown)] #[pure] ' + subtract, 7)
    yield positive('checked-conversions', 'ops.field(ops.uint(4294967295))',
        '#[intrinsic(as_u32)] pub fn uint(a:Field)->U32 '
        '#[intrinsic(as_field)] pub fn field(a:U32)->Field', 4294967295)
    nouns = ('#[intrinsic(nox_noun_atom)] pub fn atom(a:Field)->Noun '
        '#[intrinsic(nox_noun_pair)] pub fn pair(a:Noun,b:Noun)->Noun '
        '#[intrinsic(nox_noun_head)] pub fn head(a:Noun)->Noun '
        '#[intrinsic(nox_noun_tail)] pub fn tail(a:Noun)->Noun '
        '#[intrinsic(nox_noun_as_field)] pub fn field(a:Noun)->Field '
        '#[intrinsic(nox_noun_eq)] pub fn eq(a:Noun,b:Noun)->Bool')
    yield positive('complete-noun-values',
        'let n=ops.pair(ops.atom(7),ops.pair(ops.atom(9),input)) '
        'if ops.eq(ops.head(n),ops.atom(7)){ops.pair(ops.tail(n),ops.atom(ops.field(ops.head(n))))}else{input}',
        nouns, ((9, 0), 7), structured=True)
    # Independent expected identity comes from the checked-in canonical atom-0
    # DAG header used as runtime input, including all four field limbs.
    vectors = json.loads((Path(__file__).resolve().parents[3] /
                          'joy/cli/tests/compiler_vectors.json').read_text())
    zero = bytes.fromhex(vectors['files']['zero'])
    assert zero[:8] == b'NOXDAG01'
    words = [int.from_bytes(zero[i:i+8], 'little') for i in range(8, 40, 8)]
    yield positive('complete-digest-identity',
        'let d=ops.identity(input) ops.pair(ops.pair(ops.atom(d[0]),ops.atom(d[1])),'
        'ops.pair(ops.atom(d[2]),ops.atom(d[3])))',
        nouns + ' #[intrinsic(nox_noun_identity)] pub fn identity(a:Noun)->Digest',
        ((words[0], words[1]), (words[2], words[3])), structured=True)
    guards = ('#[intrinsic(assert)] pub fn stop(a:Bool) '
              '#[intrinsic(assert_eq)] pub fn equal(a:Field,b:Field)')
    yield positive('unit-assertions', 'ops.stop(true) ops.equal(7,7) 7', guards, 7)
    yield positive('ordinary-assert-keeps-body', 'ops.stop(false) 7',
        guards + ' pub fn stop(a:Bool){}', 7)
    for name, parameters, returned, call in [
        ('field_add', 'a:Field,b:Field', 'Field', '7,9'),
        ('field_mul', 'a:Field,b:Field', 'Field', '7,9'),
        ('neg', 'a:Field', 'Field', '7'),
        ('inv', 'a:Field', 'Field', '7'),
        ('split', 'a:Field', '(U32,U32)', '7'),
    ]:
        declaration = f'#[intrinsic({name})] pub fn f({parameters})->{returned}'
        yield positive('unused-known-' + name, '7', declaration, 7)
        body = f'ops.f({call})' if returned == 'Field' else f'let(a,b)=ops.f({call}) as_field(a)'
        yield negative('reachable-known-' + name, body, declaration, 'sample',
                       f'ops.f({call})', 6, False)
    for name, declarations in [
        ('private', '#[intrinsic(sub)] fn pick(a:Field,b:Field)->Field'),
        ('withdrawn', subtract + ' #[intrinsic(sub)] fn pick(a:Field,b:Field)->Field'),
    ]:
        yield negative(name, 'ops.pick(9,2)', declarations, 'sample', 'ops.pick')
    yield negative('call-arity', 'ops.pick(9)', subtract, 'sample', 'ops.pick(9)')
    yield negative('call-type', 'ops.pick(true,2)', subtract, 'sample', 'true')
    for name, declaration, attribute in [
        ('wrong-parameter', '#[intrinsic(sub)] pub fn f(a:Field,b:Bool)->Field', '#[intrinsic(sub)]'),
        ('wrong-return', '#[intrinsic(nox_noun_pair)] pub fn f(a:Noun,b:Noun)->Field', '#[intrinsic(nox_noun_pair)]'),
        ('wrong-tuple', '#[intrinsic(split)] pub fn f(a:Field)->(Field,U32)', '#[intrinsic(split)]'),
        ('invalid-replaced', '#[intrinsic(sub)] fn f(a:Bool,b:Field)->Field fn f()->Field{7}', '#[intrinsic(sub)]'),
    ]:
        yield negative(name, '7', declaration, 'ext.ops', attribute)
    yield negative('last-attribute-unknown', '7',
        '#[intrinsic(sub)] #[intrinsic(unknown)] pub fn f(a:Field,b:Field)->Field',
        'ext.ops', '#[intrinsic(unknown)]', 6, False)
    yield negative('intrinsic-with-body', '7', subtract + '{a+b}',
        'ext.ops', '#[intrinsic(sub)]', 6, False)
    yield negative('generic-intrinsic', '7',
        '#[intrinsic(sub)] pub fn f<N>(a:Field,b:Field)->Field',
        'ext.ops', '#[intrinsic(sub)]', 6, False)
    yield negative('unused-bad-body', '7',
        subtract + ' fn unused()->Field{pick(true,2)}', 'ext.ops', 'true')
    for name, source, body, value, owner, span in [
        ('transitive-wrapper', 'pub fn wrapper()->Field{ops.pick(9,2)}', 'bridge.wrapper()', 7, None, None),
        ('transitive-hidden', 'pub fn wrapper()->Field{ops.pick(9,2)}', 'ops.pick(9,2)', None, 'sample', 'ops.pick'),
        ('reachable-wrapper-known', 'pub fn wrapper()->Field{ops.inverse(7)}', 'bridge.wrapper()', None, 'bridge', 'ops.inverse(7)'),
    ]:
        declarations = subtract if name != 'reachable-wrapper-known' else '#[intrinsic(inv)] pub fn inverse(a:Field)->Field'
        row = dict(case=name, sources={'ext.ops': 'module ext.ops ' + declarations,
            'bridge': 'module bridge use ext.ops ' + source,
            'sample': f'program sample use bridge fn main()->Field{{{body}}}'})
        if value is not None:
            row['value'] = value
        else:
            known = name == 'reachable-wrapper-known'
            row.update(owner=owner, span=span, code=6 if known else 5, seed_reject=not known)
        yield row


if __name__ == '__main__':
    spec = importlib.util.spec_from_file_location('imports_acceptance',
        Path(__file__).with_name('check-guest-constant-linking.py'))
    acceptance = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(acceptance)
    acceptance.main(cases)
