"""Installed C1 callable imports, owner diagnostics and deterministic ART1."""
import importlib.util
from pathlib import Path


def cases():
    def positive(name, entry, deps, value, identity=None):
        case = dict(case=name, sources={**deps, 'sample': entry}, value=value)
        if identity:
            case['identity'] = identity
        return case

    def negative(name, entry, deps, owner, span, code=5, seed_reject=True):
        return dict(case=name, sources={**deps, 'sample': entry}, owner=owner,
                    span=span, code=code, seed_reject=seed_reject)

    deps = {'a.value': 'module a.value pub const N:Field=7 pub fn number()->Field{helper()} fn helper()->Field{N}',
            'z.bridge': '// different offsets\nmodule z.bridge use a.value pub fn get()->Field{value.number()}',
            'unreached': 'module wrong fn bad()->Field{missing}'}
    for name, expression, expected in [('private-helper', 'bridge.get()', 7),
            ('full-short', 'bridge.get()+z.bridge.get()', 14),
            ('lexical-root', 'let bridge=99 bridge.get()', 7)]:
        # The host seed takes a filesystem closure and never opens unreached.
        yield positive(name, f'program sample use z.bridge fn main()->Field{{{expression}}}', deps, expected)
    yield positive('scalar-abi', 'program sample use dep fn main()->Field{dep.ignore(false) if dep.yes(as_u32(7)){dep.field(3)}else{99}}',
        {'dep': 'module dep pub fn ignore(b:Bool){} pub fn yes(v:U32)->Bool{v==as_u32(7)} pub fn field(x:Field)->Field{x*10+as_field(uint())} fn uint()->U32{as_u32(2)}'}, 32)
    deps = {'a.same': 'module a.same pub fn f()->Field{3} pub fn keep()->Field{4}',
            'z.same': '// ж😀\nmodule z.same pub fn f()->Field{7} pub fn keep()->Field{9} fn keep()->Field{8}'}
    for name, uses, value in [('alias-last', 'use a.same use z.same', 74),
            ('alias-reversed', 'use z.same use a.same', 34),
            ('alias-repeated', 'use a.same use z.same use a.same', 34)]:
        yield positive(name, f'program sample {uses} fn main()->Field{{same.f()*10+same.keep()}}', deps, value)
    yield positive('same-function-constant', 'program sample use dep fn main()->Field{dep.f()+dep.f}',
        {'dep': 'module dep pub const f:Field=3 pub fn f()->Field{7}'}, 10)
    yield positive('ordinary-assert', 'program sample use dep fn main()->Field{dep.assert(false) 7}',
        {'dep': 'module dep pub fn assert(b:Bool){}'}, 7)
    yield positive('replacement-signature', 'program sample use dep fn main()->Field{dep.f(as_u32(7))}',
        {'dep': 'module dep pub fn f(x:Field)->Bool{true} pub fn f(x:U32)->Field{as_field(x)}'}, 7)
    long = 'member_' + 'a' * 280
    yield positive('long-commented-call', f'program sample use dep fn main()->Field{{dep // ж\n . {long}()}}',
        {'dep': f'// offset\nmodule dep pub fn {long}()->Field{{7}}'}, 7)
    for name, dep, entry in [
            ('stable-original', 'module dep pub fn go()->Field{helper()} fn helper()->Field{7}', 'program sample use dep fn main()->Field{dep.go()}'),
            ('stable-reordered', 'module dep fn helper()->Field{7} pub fn go()->Field{helper()}', 'program sample use dep fn main()->Field{dep.go()}'),
            ('stable-unused', 'module dep fn unused()->Field{9} fn helper()->Field{7} pub fn go()->Field{helper()}', 'program sample use dep fn unused()->Field{3} fn main()->Field{dep.go()}')]:
        yield positive(name, entry, {'dep': dep}, 7, 'stable')
    yield positive('owner-prefix-order', 'program sample use a use a.b fn main()->Field{a.z()*10+b.f()}',
        {'a': 'module a pub fn z()->Field{3}', 'a.b': 'module a.b pub fn f()->Field{7}'}, 37)
    for name, body in [('transitive-short', 'value.number()'), ('transitive-full', 'a.value.number()'), ('bare-ambient', 'get()')]:
        yield negative(name, f'program sample use z.bridge fn main()->Field{{{body}}}',
            {'a.value': 'module a.value pub fn number()->Field{7}', 'z.bridge': 'module z.bridge use a.value pub fn get()->Field{value.number()}'},
            'sample', body[:-2])
    for name, declaration in [('private', 'fn f()->Field{7}'), ('withdrawn', 'pub fn f()->Field{3} fn f()->Field{7}')]:
        yield negative(name, 'program sample use dep fn main()->Field{dep.f()}', {'dep': f'module dep {declaration}'}, 'sample', 'dep.f')
    for name, declaration in [('unused-bad', 'fn bad()->Field{missing} pub fn f()->Field{7}'),
            ('replaced-bad', 'pub fn f()->Field{missing} pub fn f()->Field{7}')]:
        yield negative(name, 'program sample use dep fn main()->Field{dep.f()}', {'dep': f'module dep {declaration}'}, 'dep', 'missing')
    yield negative('unused-recursion', 'program sample use dep fn main()->Field{7}',
        {'dep': 'module dep fn rec()->Field{rec()}'}, 'dep', 'rec')
    yield negative('argument-type', 'program sample use dep fn main()->Field{dep.f(true)}',
        {'dep': 'module dep pub fn f(x:Field)->Field{x}'}, 'sample', 'true')
    yield negative('argument-count', 'program sample use dep fn main()->Field{dep.f()}',
        {'dep': 'module dep pub fn f(x:Field)->Field{x}'}, 'sample', 'dep.f()')
    yield negative('ordinary-assert-not-bottom', 'program sample use dep fn main()->Field{dep.assert(false)}',
        {'dep': 'module dep pub fn assert(b:Bool){}'}, 'sample', 'dep.assert(false)')
    yield negative('pure-qualified-io-name', 'program sample use dep #[pure] fn main()->Field{dep.pub_read_custom()}',
        {'dep': 'module dep pub fn pub_read_custom()->Field{7}'}, 'sample', 'dep.pub_read_custom')
    for name, declaration, span in [('noun-signature', 'pub fn f(x:Noun)->Field{7}', 'Noun'),
            ('tuple-signature', 'pub fn f(x:(Field,Bool))->Field{7}', '('),
            ('struct', 'pub struct S{x:Field}', 'struct'),
            ('noun-return', 'pub fn f()->Noun{nox_noun_atom(0)}', 'Noun'),
            ('digest-return', 'pub fn f()->Digest{nox_noun_identity(nox_noun_atom(0))}', 'Digest')]:
        yield negative(name, 'program sample use dep fn main()->Field{7}', {'dep': f'module dep {declaration}'}, 'dep', span, 6, False)


if __name__ == '__main__':
    spec = importlib.util.spec_from_file_location('imports_acceptance', Path(__file__).with_name('check-guest-constant-linking.py'))
    acceptance = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(acceptance)
    acceptance.main(cases)
