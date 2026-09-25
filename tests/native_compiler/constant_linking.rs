use super::{nouns::data, support};
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

fn seed(modules: &[schema::Module]) -> std::result::Result<Vec<u8>, String> {
    let dir = tempfile::tempdir().unwrap();
    for module in modules {
        let path = dir.path().join(module.path.replace('.', "/") + ".tri");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut source = String::from_utf8(module.source.clone()).unwrap();
        if module.path == "sample" {
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
    .map(|a| a.bytes)
    .map_err(|e| format!("{e:?}"))
}

fn agrees(entry: &str, dependencies: &[(&str, &str)], expected: u64) -> Vec<u8> {
    let modules = package(entry, dependencies);
    let bytes = match compile(&modules) {
        Compilation::Program {
            bytes,
            reductions,
            nodes,
            frames,
        } => {
            eprintln!("imports: {reductions} reductions, {nodes} nodes, {frames} frames");
            bytes
        }
        other => panic!("{entry}: {other:?}"),
    };
    assert_eq!(support::run_artifact(&bytes), expected, "{entry}");
    let oracle = seed(&modules).unwrap();
    assert_eq!(
        data::run(&oracle, &data::Noun::Atom(0)).unwrap().bytes,
        data::Noun::Atom(expected).encoded(),
        "seed {entry}"
    );
    bytes
}

fn rejected(entry: &str, dependencies: &[(&str, &str)], owner: &str, span: &str, code: u32) {
    let modules = package(entry, dependencies);
    let errors = match compile(&modules) {
        Compilation::Errors(errors) => errors,
        other => panic!("accepted {entry}: {other:?}"),
    };
    assert_eq!(errors.len(), 1);
    let error = &errors[0];
    assert_eq!(error.code, code, "{entry}: {error:?}");
    let module = &modules[error.module as usize];
    assert_eq!(module.path, owner, "{error:?}");
    assert_eq!(
        &module.source[error.start as usize..error.end as usize],
        span.as_bytes()
    );
    if code != 6 {
        assert!(seed(&modules).is_err(), "seed accepted {entry}");
    }
}

#[test]
fn guest_imports_compile_transitive_frozen_constants_without_host_resolution() {
    support::worker(|| {
        let deps = [
            ("a.value", "// ж\nmodule a.value pub const X:Field=3 pub const I:U32=7 pub const P:Field=18446744069414584321"),
            ("z.bridge", "module z.bridge use a.value pub const X:Field=value.X pub const I:U32=a.value.I pub const P:Field=value.P"),
        ];
        for (body, expected) in [
            ("bridge.X+z.bridge.X", 6),
            ("as_field(bridge.I)", 7),
            ("[7][bridge.P]", 7),
            ("if bridge.P{return 7}", 7),
        ] {
            agrees(
                &format!("program sample use z.bridge fn main()->Field{{{body}}}"),
                &deps,
                expected,
            );
        }
        let bytes = agrees("program sample use z.bridge const X:Field=bridge.X const Y:Field=X fn main()->Field{Y}", &deps, 3);
        assert_eq!(bytes, data::compile("program sample fn main()->Field{3}"));
        rejected(
            "program sample use z.bridge fn main()->Field{value.X}",
            &deps,
            "sample",
            "value.X",
            5,
        );
        rejected(
            "program sample use z.bridge fn main()->Field{a.value.X}",
            &deps,
            "sample",
            "a.value.X",
            5,
        );
    });
}

#[test]
fn guest_imports_retain_per_symbol_source_order_and_lexical_shadowing() {
    support::worker(|| {
        let deps = [
            (
                "a.same",
                "module a.same pub const X:Field=3 pub const Y:Field=4",
            ),
            (
                "z.same",
                "module z.same pub const X:Field=7 const Y:Field=9",
            ),
        ];
        for (uses, expected) in [
            ("use a.same use z.same", 74),
            ("use z.same use a.same", 34),
            ("use a.same use z.same use a.same", 34),
        ] {
            agrees(
                &format!("program sample {uses} fn main()->Field{{same.X*10+same.Y}}"),
                &deps,
                expected,
            );
        }
        agrees(
            "program sample use a.same use z.same const same:Field=99 fn main()->Field{same.X}",
            &deps,
            7,
        );
        agrees(
            "program sample use a.same struct S{X:Field} fn main()->Field{let same=S{X:9} same.X}",
            &deps,
            9,
        );
        agrees("program sample use a.same struct S{X:Field} fn f(same:S)->Field{same.X} fn main()->Field{f(S{X:9})}", &deps, 9);
        agrees(
            "program sample use a.same fn main()->Field{a.same.X*10+a.same.Y}",
            &deps,
            34,
        );
    });
}

#[test]
fn guest_imports_validate_private_and_replaced_declarations_before_exporting() {
    support::worker(|| {
        for source in [
            "module dep const BAD:Field=missing pub const X:Field=7",
            "module dep pub const X:Field=missing pub const X:Field=7",
        ] {
            rejected(
                "program sample use dep fn main()->Field{dep.X}",
                &[("dep", source)],
                "dep",
                "missing",
                5,
            );
        }
        for source in [
            "module dep pub const X:Field=3 const X:Field=7",
            "module dep const X:Field=7",
        ] {
            rejected(
                "program sample use dep fn main()->Field{dep.X}",
                &[("dep", source)],
                "sample",
                "dep.X",
                5,
            );
        }
        let deps = [(
            "dep",
            "module dep const X:Field=3 pub const X:Field=7 pub const Y:Field=X",
        )];
        agrees("program sample use dep fn main()->Field{dep.Y}", &deps, 7);
        rejected(
            "program sample use dep const X:U32=dep.X fn main()->Field{7}",
            &deps,
            "sample",
            "dep.X",
            5,
        );
        rejected(
            "program sample use dep fn main()->Field{dep.X()}",
            &deps,
            "sample",
            "dep.X",
            6,
        );
        rejected(
            "program sample use dep fn main()->Field{dep.X{}}",
            &deps,
            "sample",
            "dep.X",
            6,
        );
    });
}

#[test]
fn guest_import_errors_keep_original_package_indices_and_caller_byte_spans() {
    support::worker(|| {
        rejected(
            "program sample use a fn main()->Field{7}",
            &[("a", "module a use z"), ("z", "module z use a")],
            "z",
            "use a",
            4,
        );
        rejected(
            "program sample use a fn main()->Field{7}",
            &[("a", "module a use z")],
            "a",
            "use z",
            3,
        );
        rejected(
            "program sample use a fn main()->Field{7}",
            &[("a", "module b")],
            "a",
            "b",
            3,
        );
        rejected(
            "program sample use a fn main()->Field{7}",
            &[("a", "module a fn f()->Field{7}")],
            "a",
            "fn",
            6,
        );
        let name = "member_".to_string() + &"a".repeat(280);
        let dep = format!("// a different byte offset\nmodule a pub const {name}:Field=9");
        agrees(
            &format!("program sample use a fn main()->Field{{a //ж\n . {name}}}"),
            &[("a", &dep)],
            9,
        );
        let mut modules = package(
            "program sample use a fn main()->Field{a.X}",
            &[("a", "module a pub const X:Field=7"), ("b", "unused")],
        );
        modules.iter_mut().find(|m| m.path == "b").unwrap().source = vec![0xff];
        match compile(&modules) {
            Compilation::Program { bytes, .. } => assert_eq!(support::run_artifact(&bytes), 7),
            other => panic!("unused source opened: {other:?}"),
        }
    });
}
