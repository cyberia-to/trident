use super::{nouns::data, support};
use data::{Noun, Noun::Atom};
use support::{schema, Compilation};

fn package(entry: &str, dependencies: &[(&str, &str)]) -> Vec<schema::Module> {
    let mut modules = vec![support::module(entry.as_bytes())];
    for (path, source) in dependencies {
        let mut module = support::module(source.as_bytes());
        module.path = (*path).into();
        modules.push(module);
    }
    modules.sort_by(|a, b| a.path.cmp(&b.path));
    modules
}

fn compile(modules: &[schema::Module]) -> Compilation {
    support::try_compile_only_package(modules, "sample", "main", support::options(), data::caps())
        .unwrap()
}

// Only the independent seed oracle sees these temporary files. Guest discovery
// receives separate MOD1 sources and performs all import/type resolution itself.
fn seed(modules: &[schema::Module]) -> Result<Vec<u8>, String> {
    let dir = tempfile::tempdir().unwrap();
    for module in modules {
        let path = dir.path().join(module.path.replace('.', "/") + ".tri");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut source = String::from_utf8(module.source.clone()).unwrap();
        if module.path == "sample" && source.contains("fn main()->Field") {
            source = source.replacen("fn main()->Field", "fn result()->Field", 1);
            source += " fn main(input:Noun)->Noun{nox_noun_atom(result())}";
        }
        std::fs::write(path, source).unwrap();
    }
    trident::compile_native_artifact_project(
        &dir.path().join("sample.tri"),
        &Default::default(),
        trident::NativeArtifactProfile::RawNoun,
        trident::NATIVE_ARTIFACT_LIMITS,
    )
    .map(|artifact| artifact.bytes)
    .map_err(|errors| format!("{errors:?}"))
}

fn program(modules: &[schema::Module]) -> Vec<u8> {
    match compile(modules) {
        Compilation::Program { bytes, .. } => bytes,
        other => panic!("modules={modules:?}: {other:?}"),
    }
}

fn agrees(entry: &str, dependencies: &[(&str, &str)], expected: u64) -> Vec<u8> {
    let modules = package(entry, dependencies);
    let bytes = program(&modules);
    for code in [&bytes, &seed(&modules).unwrap()] {
        assert_eq!(
            data::run(code, &Atom(0)).unwrap().bytes,
            Atom(expected).encoded(),
            "{entry}"
        );
    }
    bytes
}

fn rejected(entry: &str, dependencies: &[(&str, &str)], owner: &str, context: &str, span: &str) {
    let modules = package(entry, dependencies);
    let errors = match compile(&modules) {
        Compilation::Errors(errors) => errors,
        other => panic!("accepted {entry}: {other:?}"),
    };
    assert_eq!(errors.len(), 1);
    let error = &errors[0];
    assert_eq!(error.code, 5, "{entry}: {error:?}");
    let index = modules
        .iter()
        .position(|module| module.path == owner)
        .unwrap();
    assert_eq!(error.module as usize, index, "{entry}: {error:?}");
    let source = std::str::from_utf8(&modules[index].source).unwrap();
    assert_eq!(
        source.matches(context).count(),
        1,
        "ambiguous diagnostic context"
    );
    let start = source.find(context).unwrap() + context.find(span).unwrap();
    assert_eq!(
        (error.start as usize, error.end as usize),
        (start, start + span.len()),
        "{source}: {error:?}"
    );
    assert!(seed(&modules).is_err(), "seed accepted {entry}");
}

#[test]
fn qualified_types_and_constructors_preserve_full_and_short_descriptor_identity() {
    support::worker(|| {
        let deps = [("bank.values", "// different foreign offsets\nmodule bank.values pub struct Item{pub x:Field,pub y:Field} pub struct Empty{} pub fn echo(x:Item)->Item{x}")];
        let source = "program sample use bank.values fn pass(x:values.Item)->bank.values.Item{x} fn main()->Field{let x:bank.values.Item=values.Item{y:9,x:7} let y:values.Item=bank.values.echo(pass(x)) if (values.Empty{})==(bank.values.Empty{}){y.x*10+y.y}else{0}}";
        let first = agrees(source, &deps, 79);
        let second = agrees(
            &source.replace("values.Item{y:9,x:7}", "bank.values.Item{x:7,y:9}"),
            &deps,
            79,
        );
        assert_eq!(
            first, second,
            "aliases and initializer label order are absent from ART1"
        );
        agrees(
            "program sample use bank.values fn main()->Field{let x=7 let p:values.Item=values.Item{x,y:9,} p.x*10+p.y}",
            &deps, 79,
        );
        agrees(
            "program sample use bank.values fn main()->Field{let p:Later=Later{x:values.Item{x:7,y:9}} p.x.y} struct Later{x:values.Item}",
            &deps, 9,
        );
    });
}

#[test]
fn type_aliases_follow_per_symbol_use_order_and_final_declaration_visibility() {
    support::worker(|| {
        let deps = [
            ("a.same", "module a.same pub struct Item{pub x:Field} pub struct Keep{pub x:Field} pub fn make()->Item{Item{x:3}} pub fn keep(x:Keep)->Field{x.x}"),
            ("z.same", "module z.same pub struct Item{pub x:Field} pub struct Keep{pub x:Field} struct Keep{pub x:Field} pub fn make()->Item{Item{x:7}}"),
        ];
        for (uses, expected) in [
            ("use a.same use z.same", 74),
            ("use z.same use a.same", 34),
            ("use a.same use z.same use a.same", 34),
        ] {
            agrees(&format!("program sample {uses} fn main()->Field{{let x:same.Item=same.make() let k:same.Keep=same.Keep{{x:4}} x.x*10+a.same.keep(k)}}"), &deps, expected);
        }
        rejected(
            "program sample use a.same use z.same fn main()->Field{let x=z.same.Keep{x:4} x.x}",
            &deps,
            "sample",
            "z.same.Keep{x:4}",
            "z.same.Keep",
        );
        for declarations in [
            "struct Item{pub x:Field} pub struct Item{pub x:Field}",
            "pub struct Item{pub x:Field} struct Item{pub x:Field}",
        ] {
            let dep = format!("module dep {declarations} pub fn make()->Item{{Item{{x:7}}}}");
            agrees(
                "program sample use dep fn main()->Field{dep.make().x}",
                &[("dep", &dep)],
                7,
            );
        }
        agrees(
            "program sample use a.types use dep fn main()->Field{let s:dep.S=dep.make() s.x.n}",
            &[("a.types", "module a.types pub struct Item{pub n:Field}"),
              ("dep", "module dep use a.types struct S{pub x:a.types.Item} pub struct S{pub x:types.Item} pub fn make()->S{S{x:types.Item{n:7}}}")], 7,
        );
    });
}

#[test]
fn every_dependency_layout_is_stable_and_forward_types_remain_unavailable() {
    support::worker(|| {
        // The changed-layout seed rejection oracle requires the RootI stable
        // nominal binding prerequisite; these assertions must never be skipped.
        for (definitions, context, span) in [
            (
                "struct S{x:Field} struct S{x:Bool}",
                "struct S{x:Bool}",
                "S",
            ),
            (
                "struct S{pub x:Field} struct S{x:Field}",
                "struct S{x:Field}",
                "S",
            ),
            (
                "struct S{x:Field,y:Field} struct S{y:Field,x:Field}",
                "struct S{y:Field,x:Field}",
                "S",
            ),
            (
                "struct S{x:[Field;2]} struct S{x:[Field;3]}",
                "struct S{x:[Field;3]}",
                "S",
            ),
            (
                "struct A{x:Field} struct B{a:A} struct A{x:Bool}",
                "struct A{x:Bool}",
                "A",
            ),
            (
                "struct A{x:Field} struct B{a:A} struct A{b:B}",
                "struct A{b:B}",
                "A",
            ),
            (
                "fn make()->Later{Later{x:7}} struct Later{x:Field}",
                "fn make()->Later",
                "Later",
            ),
            (
                "struct First{x:Later} struct Later{x:Field}",
                "struct First{x:Later}",
                "Later",
            ),
        ] {
            let dep = format!("// shifted\nmodule z.dep {definitions} pub fn good()->Field{{7}}");
            rejected(
                "program sample use z.dep fn main()->Field{7}",
                &[("a.unused", "module a.unused"), ("z.dep", &dep)],
                "z.dep",
                context,
                span,
            );
        }
    });
}

#[test]
fn opaque_nominal_values_cross_diamonds_with_owner_privacy_and_persistent_writes() {
    support::worker(|| {
        let deps = [
            ("leaf", "module leaf struct Hidden{pub x:Field,secret:Field} pub struct Box{pub inner:Hidden,pub other:Field} pub fn make()->Box{Box{inner:Hidden{x:7,secret:11},other:3}} pub fn read(x:Box)->Field{x.inner.secret}"),
            ("left", "module left use leaf pub fn get()->leaf.Box{leaf.make()}"),
            ("right", "module right use leaf pub fn take(x:leaf.Box)->Field{leaf.read(x)}"),
        ];
        agrees(
            "program sample use left use right fn main()->Field{let mut s=left.get() let old=s s.inner.x=9 s.other=5 right.take(s)+old.inner.x*100+s.inner.x*10+s.other+old.other}",
            &deps, 809,
        );
        agrees(
            "program sample use left use right fn main()->Field{let mut s=left.get() let old=s s.inner.x=9 s.inner=old.inner right.take(s)+s.inner.x}",
            &deps, 18,
        );
        for (body, context, span) in [
            (
                "let s=left.get() s.inner.secret",
                "s.inner.secret",
                "secret",
            ),
            (
                "let mut s=left.get() s.inner.secret=9 7",
                "s.inner.secret=9",
                "secret",
            ),
            ("let s:leaf.Box=left.get() 7", "s:leaf.Box", "leaf.Box"),
        ] {
            rejected(
                &format!("program sample use left use right fn main()->Field{{{body}}}"),
                &deps,
                "sample",
                context,
                span,
            );
        }
        rejected(
            "program sample use leaf fn main()->Field{let x=leaf.Hidden{x:7,secret:11} 7}",
            &deps,
            "sample",
            "leaf.Hidden{x:7,secret:11}",
            "leaf.Hidden",
        );
    });
}

#[test]
fn nominal_names_require_direct_visibility_and_same_shape_does_not_erase_owner() {
    support::worker(|| {
        let deps = [
            (
                "a.types",
                "module a.types pub struct Item{pub x:Field} pub fn take(x:Item)->Field{x.x}",
            ),
            ("z.types", "module z.types pub struct Item{pub x:Field}"),
        ];
        rejected(
            "program sample use a.types use z.types fn main()->Field{a.types.take(types.Item{x:7})}",
            &deps, "sample", "types.Item{x:7}", "types.Item{x:7}",
        );
        rejected(
            "program sample use a.types use z.types fn main()->Field{let x:a.types.Item=z.types.Item{x:7} 7}",
            &deps, "sample", "z.types.Item{x:7}", "z.types.Item{x:7}",
        );
        rejected(
            "program sample use a.types fn main()->Field{let x:Item=a.types.Item{x:7} 7}",
            &deps,
            "sample",
            "x:Item",
            "Item",
        );
        let dep = "module dep pub struct Locked{pub x:Field,secret:Field} pub fn make()->Locked{Locked{x:7,secret:9}}";
        rejected(
            "program sample use dep fn main()->Field{let x=dep.Locked{x:7,secret:9} x.x}",
            &[("dep", dep)],
            "sample",
            "dep.Locked{x:7,secret:9}",
            "dep.Locked",
        );
        agrees(
            "program sample use dep fn main()->Field{dep.make().x}",
            &[("dep", dep)],
            7,
        );
        rejected(
            "program sample use dep fn main()->Field{let x=dep.make() x.x=9 7}",
            &[("dep", dep)],
            "sample",
            "x.x=9",
            "x.x",
        );
    });
}

#[test]
fn qualified_constructor_namespace_ignores_lexical_roots_but_field_reads_keep_them() {
    support::worker(|| {
        let deps = [("dep", "module dep pub struct Item{pub x:Field} pub const Item:Field=3 pub fn Item()->Field{5}")];
        for (entry, expected) in [
            ("program sample use dep fn main()->Field{let dep=9 let x:dep.Item=dep.Item{x:7} x.x+dep}", 16),
            ("program sample use dep struct Local{Item:Field} fn main()->Field{let dep=Local{Item:9} let x:dep.Item=dep.Item{x:7} dep.Item()*100+x.x*10+dep.Item}", 579),
            ("program sample use dep fn main()->Field{let x=dep.Item{x:7} dep.Item()*100+x.x*10+dep.Item}", 573),
            ("program sample use dep fn main()->Field{let dep=9 let x=dep // ж\n . Item{x:7} x.x+dep}", 16),
            ("program sample use dep fn main()->Field{let dep=9 if (dep.Item{x:7}).x==7{dep}else{0}}", 9),
        ] { agrees(entry, &deps, expected); }
    });
}

fn digest_words(input: &Noun) -> Noun {
    let encoded = input.encoded();
    let word = |i: usize| {
        Atom(u64::from_le_bytes(
            encoded[8 + i * 8..16 + i * 8].try_into().unwrap(),
        ))
    };
    Noun::pair(Noun::pair(word(0), word(1)), Noun::pair(word(2), word(3)))
}

#[test]
fn dependency_aggregate_abis_preserve_every_digest_word_noun_and_array_element() {
    support::worker(|| {
        let entry = "program sample use dep fn main(input:Noun)->Noun{let(n,d)=dep.pass(dep.make(input)) let d=dep.echo(d) nox_noun_pair(n,nox_noun_pair(nox_noun_pair(nox_noun_atom(d[0]),nox_noun_atom(d[1])),nox_noun_pair(nox_noun_atom(d[2]),nox_noun_atom(d[3]))))}";
        let deps = [("dep", "module dep pub fn make(x:Noun)->(Noun,Digest){(x,nox_noun_identity(x))} pub fn pass(x:(Noun,Digest))->(Noun,Digest){x} pub fn echo(x:Digest)->Digest{x}")];
        let modules = package(entry, &deps);
        let guest = program(&modules);
        let oracle = seed(&modules).unwrap();
        for input in [Atom(0), Atom(18446744069414584320), data::nested()] {
            let expected = Noun::pair(input.clone(), digest_words(&input)).encoded();
            for code in [&guest, &oracle] {
                assert_eq!(data::run(code, &input).unwrap().bytes, expected);
            }
        }
        let entry = "program sample use dep fn words(x:[Field;3])->Noun{nox_noun_pair(nox_noun_atom(x[0]),nox_noun_pair(nox_noun_atom(x[1]),nox_noun_atom(x[2])))} fn main(input:Noun)->Noun{let p=dep.make(input,[3,5,7]) let q=dep.change(p) dep.ignore(q) nox_noun_pair(p.data,nox_noun_pair(words(p.words),nox_noun_pair(q.data,words(dep.words(q.words))))) }";
        let deps = [("dep", "module dep pub struct Packet{pub data:Noun,pub words:[Field;3]} pub fn make(x:Noun,w:[Field;3])->Packet{Packet{data:x,words:w}} pub fn change(x:Packet)->Packet{let mut p=x p.data=nox_noun_pair(p.data,nox_noun_atom(9)) p.words=[11,13,17] p} pub fn ignore(x:Packet){} pub fn words(x:[Field;3])->[Field;3]{x}")];
        let modules = package(entry, &deps);
        let input = data::nested();
        let old = Noun::pair(Atom(3), Noun::pair(Atom(5), Atom(7)));
        let new = Noun::pair(Atom(11), Noun::pair(Atom(13), Atom(17)));
        let expected = Noun::pair(
            input.clone(),
            Noun::pair(old, Noun::pair(Noun::pair(input.clone(), Atom(9)), new)),
        )
        .encoded();
        for code in [program(&modules), seed(&modules).unwrap()] {
            assert_eq!(data::run(&code, &input).unwrap().bytes, expected);
        }
    });
}

#[test]
fn imported_constructors_use_declaration_order_once_and_edits_keep_old_snapshots() {
    support::worker(|| {
        let deps = [(
            "dep",
            "module dep pub struct Pair{pub first:Field,pub second:Field}",
        )];
        let bytes = agrees(
            "program sample use dep fn main()->Field{let p=dep.Pair{second:sub(8,3),first:sub(9,2)} p.first*10+p.second}",
            &deps, 75,
        );
        let (value, trace) = support::trace_artifact(&bytes);
        assert_eq!(value, 75);
        assert_eq!(trace.0.iter().filter(|row| row.col(0) == 6).count(), 2);
        let inv = "as_field(as_u32(4294967296))";
        let axis = "nox_noun_as_field(nox_noun_head(nox_noun_atom(0)))";
        for (first, second, expected) in [
            (inv, axis, "Error(InvZero)"),
            (axis, inv, "Error(AxisError)"),
        ] {
            let entry = format!("program sample use dep fn main()->Field{{let unused=dep.Pair{{second:{second},first:{first}}} 7}}");
            let modules = package(&entry, &deps);
            for code in [program(&modules), seed(&modules).unwrap()] {
                assert_eq!(data::run(&code, &Atom(0)), Err(expected.into()));
            }
        }
        let bytes = agrees(
            "program sample use dep fn main()->Field{let mut p=dep.Pair{first:9,second:2} let old=p p.first=sub(p.first,p.second) old.first*100+p.first*10+p.second}",
            &deps, 972,
        );
        let (_, trace) = support::trace_artifact(&bytes);
        assert_eq!(trace.0.iter().filter(|row| row.col(0) == 6).count(), 1);
    });
}
