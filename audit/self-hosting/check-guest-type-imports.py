"""Installed C1 nominal imports preserve complete values and owner visibility."""
import importlib.util
from pathlib import Path


def cases():
    def positive(name, uses, body, deps, value, declarations='', structured=False):
        signature = 'fn main(input:Noun)->Noun' if structured else 'fn main()->Field'
        return dict(case=name, sources={**deps, 'sample': f'program sample {uses} {declarations} {signature}{{{body}}}'},
                    value=value, structured=structured)

    def negative(name, uses, body, deps, owner, span, declarations=''):
        return dict(case=name, sources={**deps, 'sample': f'program sample {uses} {declarations} fn main()->Field{{{body}}}'},
                    owner=owner, span=span, code=5, seed_reject=True)

    dep = {'a.model': '// ж😀 distinct spans\nmodule a.model pub struct Item{pub x:Field,pub y:Field} pub fn sum(s:Item)->Field{s.x+s.y}'}
    for name, body in [
        ('full-short', 'let s:a.model.Item=model.Item{y:9,x:7} model.sum(s)'),
        ('lexical-constructor-root', 'let model=99 let s:model.Item=model.Item{y:9,x:7} model.sum(s)'),
        ('commented-path', 'let s:a //ж\n . model . Item=a.model.Item{x:7,y:9} model.sum(s)'),
        ('mutable-snapshot', 'let mut s=model.Item{x:7,y:9} let old=s s.x=3 old.x+s.y'),
    ]:
        yield positive(name, 'use a.model', body, dep, 16)
    opaque = {'dep': 'module dep struct Hidden{pub x:Field,secret:Field} pub fn make()->Hidden{Hidden{x:7,secret:9}} pub fn secret(s:Hidden)->Field{s.secret}'}
    yield positive('opaque-return', 'use dep', 'let s=dep.make() s.x+dep.secret(s)', opaque, 16)
    yield negative('opaque-private-field', 'use dep', 'dep.make().secret', opaque, 'sample', 'secret')
    yield negative('opaque-private-type', 'use dep', 'let s:dep.Hidden=dep.make() s.x', opaque, 'sample', 'dep.Hidden')
    yield negative('opaque-private-constructor', 'use dep', 'dep.Hidden{x:7,secret:9}.x', opaque, 'sample', 'dep.Hidden')
    for name, declarations in [('final-public', 'struct S{pub x:Field} pub struct S{pub x:Field}'),
                              ('repeated-public', 'pub struct S{pub x:Field} pub struct S{pub x:Field}')]:
        yield positive(name, 'use dep', 'dep.S{x:7}.x', {'dep': 'module dep '+declarations}, 7)
    hidden = {'dep': 'module dep pub struct S{pub x:Field} struct S{pub x:Field} pub fn make()->S{S{x:7}}'}
    yield positive('final-private-opaque', 'use dep', 'dep.make().x', hidden, 7)
    yield negative('withdrawn-constructor', 'use dep', 'dep.S{x:7}.x', hidden, 'sample', 'dep.S')
    aliases = {'a.same': 'module a.same pub struct S{pub x:Field} pub struct Keep{pub x:Field} pub fn take(s:S)->Field{s.x}',
               'z.same': 'module z.same pub struct S{pub x:Field} struct Keep{pub x:Field} pub fn take(s:S)->Field{s.x+10}'}
    for name, uses, expected in [('alias-last','use a.same use z.same',21),
                                 ('alias-reversed','use z.same use a.same',11),
                                 ('alias-repeated','use a.same use z.same use a.same',11)]:
        yield positive(name, uses, 'let s:same.S=same.S{x:7} same.take(s)+same.Keep{x:4}.x', aliases, expected)
    yield negative('different-owners', 'use a.same use z.same', 'a.same.take(z.same.S{x:7})', aliases, 'sample', 'z.same.S{x:7}')
    bridge = {'base': 'module base pub struct S{pub x:Field} pub fn make()->S{S{x:7}}',
              'bridge': 'module bridge use base pub fn make()->base.S{base.make()}'}
    yield positive('transitive-opaque-value', 'use bridge', 'bridge.make().x', bridge, 7)
    yield negative('transitive-type-hidden', 'use bridge', 'let s:base.S=bridge.make() s.x', bridge, 'sample', 'base.S')
    yield negative('transitive-constructor-hidden', 'use bridge', 'base.S{x:7}.x', bridge, 'sample', 'base.S')
    aggregate = {'dep': 'module dep pub struct Box{pub n:Noun,pub a:[Field;2],pub t:(Field,Bool)} pub fn echo(x:Box)->Box{x} pub fn array(x:[Field;2])->[Field;2]{x} pub fn tuple(x:(Field,Bool))->(Field,Bool){x} pub fn noun(x:Noun)->Noun{x}'}
    yield positive('complete-aggregate-values', 'use dep',
        'let b=dep.echo(dep.Box{n:dep.noun(nox_noun_pair(nox_noun_atom(7),nox_noun_atom(9))),a:dep.array([3,5]),t:dep.tuple((11,true))}) let(x,flag)=b.t if flag{nox_noun_pair(b.n,nox_noun_pair(nox_noun_atom(b.a[0]),nox_noun_pair(nox_noun_atom(b.a[1]),nox_noun_atom(x))))}else{input}',
        aggregate, ((7,9),(3,(5,11))), structured=True)
    for name, body, span in [
        ('changed-layout','pub struct S{pub x:Field} pub struct S{pub x:Bool}','S'),
        ('changed-privacy','pub struct S{pub x:Field} pub struct S{x:Field}','S'),
        ('forward-type','pub struct S{x:Later} struct Later{x:Field}','Later'),
    ]:
        yield negative(name, 'use dep', '7', {'dep': 'module dep '+body}, 'dep', span)
    long = 'Type_' + 'x'*280
    yield positive('long-type-name','use dep',f'dep.{long}{{x:7}}.x',{'dep': f'module dep pub struct {long}{{pub x:Field}}'},7)


if __name__ == '__main__':
    spec = importlib.util.spec_from_file_location('imports_acceptance', Path(__file__).with_name('check-guest-constant-linking.py'))
    acceptance = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(acceptance)
    acceptance.main(cases)
