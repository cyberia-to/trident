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

// The guest receives MOD1 bytes. Only this independent seed oracle writes a
// temporary source project; the oracle never supplies guest resolution data.
fn seed(modules: &[schema::Module]) -> Result<Vec<u8>, String> {
    let dir = tempfile::tempdir().unwrap();
    for module in modules {
        let path = dir.path().join(module.path.replace('.', "/") + ".tri");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut source = String::from_utf8(module.source.clone()).unwrap();
        if module.path == "sample" {
            assert!(source.contains("fn main()->Field"));
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
    assert_eq!(support::run_artifact(&bytes), expected, "{entry}");
    let oracle = seed(&modules).unwrap();
    assert_eq!(
        data::run(&oracle, &data::Noun::Atom(0)).unwrap().bytes,
        data::Noun::Atom(expected).encoded(),
        "seed {entry}"
    );
    bytes
}

fn rejected(entry: &str, dependencies: &[(&str, &str)], owner: &str, span: &str) {
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
    assert_eq!(
        &modules[index].source[error.start as usize..error.end as usize],
        span.as_bytes(),
        "{entry}: {error:?}"
    );
    assert!(seed(&modules).is_err(), "seed accepted {entry}");
}

#[test]
fn imported_functions_keep_private_helpers_forward_calls_and_scalar_abis() {
    support::worker(|| {
        agrees(
            "program sample use dep fn main()->Field{let mut x=3 x=dep.run(x) dep.run(x)+x}",
            &[("dep", "// distinct offsets\nmodule dep const STEP:Field=1 pub fn run(x:Field)->Field{later(x)} fn later(x:Field)->Field{x+STEP}")],
            9,
        );
        let deps = [("dep", "module dep pub fn bit(x:Bool)->Bool{if x{return false} true} pub fn word(x:U32)->U32{x} pub fn unit(){return} pub fn main(x:Bool)->Bool{x}")];
        agrees(
            "program sample use dep fn main()->Field{dep.unit() if dep.bit(false){as_field(dep.word(as_u32(4294967295)))}else{9}}",
            &deps,
            4294967295,
        );
        agrees(
            "program sample use dep fn main()->Field{if dep.main(true){7}else{9}}",
            &deps,
            7,
        );
        agrees(
            "program sample use dep fn main()->Field{if dep.f(7,true){9}else{3}}",
            &[("dep", "module dep pub fn f(x:Field,x:Bool)->Bool{x}")],
            9,
        );
        agrees(
            "program sample use dep fn main()->Field{let x=3 dep.bump(x)+x}",
            &[("dep", "module dep pub fn bump(x:Field)->Field{for i in 0..2{if i==as_u32(1){return x+1}} 9}")],
            7,
        );
    });
}

#[test]
fn imported_callables_follow_direct_per_symbol_use_order_and_final_visibility() {
    support::worker(|| {
        let deps = [
            (
                "a.same",
                "module a.same pub fn x()->Field{3} pub fn y()->Field{4}",
            ),
            (
                "z.same",
                "module z.same pub fn x()->Field{7} pub fn y()->Field{8} fn y()->Field{9}",
            ),
        ];
        for (uses, expected) in [
            ("use a.same use z.same", 74),
            ("use z.same use a.same", 34),
            ("use a.same use z.same use a.same", 34),
        ] {
            agrees(
                &format!("program sample {uses} fn main()->Field{{same.x()*10+same.y()}}"),
                &deps,
                expected,
            );
        }
        agrees(
            "program sample use a.same use z.same fn main()->Field{a.same.x()*10+z.same.x()}",
            &deps,
            37,
        );
        rejected(
            "program sample use a.same use z.same fn main()->Field{z.same.y()}",
            &deps,
            "sample",
            "z.same.y",
        );
        agrees(
            "program sample use dep fn main()->Field{if dep.f(true){7}else{9}}",
            &[(
                "dep",
                "module dep pub fn f()->Field{3} pub fn f(x:Bool)->Bool{x}",
            )],
            7,
        );
        rejected(
            "program sample use dep fn main()->Field{dep.f()}",
            &[(
                "dep",
                "module dep pub fn f()->Field{3} pub fn f(x:Field)->Field{x}",
            )],
            "sample",
            "dep.f()",
        );
        rejected(
            "program sample use dep fn main()->Field{dep.f(true)}",
            &[("dep", "module dep pub fn f(x:Field)->Field{x}")],
            "sample",
            "true",
        );
        rejected(
            "program sample use dep fn main()->Field{as_field(dep.f(7))}",
            &[("dep", "module dep pub fn f(x:U32)->U32{x}")],
            "sample",
            "7",
        );
    });
}

#[test]
fn imported_functions_do_not_expose_private_transitive_or_ambient_callables() {
    support::worker(|| {
        let deps = [
            (
                "a",
                "module a pub fn visible()->Field{7} fn hidden()->Field{9}",
            ),
            ("dep", "module dep use a pub fn run()->Field{a.visible()}"),
        ];
        agrees(
            "program sample use dep fn main()->Field{dep.run()}",
            &deps,
            7,
        );
        for (uses, call) in [
            ("use a", "hidden()"),
            ("use a", "visible()"),
            ("use a", "a.hidden()"),
            ("use dep", "a.visible()"),
        ] {
            let span = call.strip_suffix("()").unwrap();
            rejected(
                &format!("program sample {uses} fn main()->Field{{{call}}}"),
                &deps,
                "sample",
                span,
            );
        }
        rejected(
            "program sample use a use z fn main()->Field{z.run()}",
            &[
                ("a", "module a pub fn helper()->Field{7}"),
                ("z", "module z pub fn run()->Field{helper()}"),
            ],
            "z",
            "helper",
        );
        agrees(
            "program sample use b use c fn main()->Field{b.f()*10+c.f()}",
            &[
                ("a", "module a pub fn f()->Field{3}"),
                ("b", "module b use a pub fn f()->Field{a.f()+1}"),
                ("c", "module c use a pub fn f()->Field{a.f()+2}"),
            ],
            45,
        );
    });
}

#[test]
fn dependency_body_errors_retain_original_owner_spans_including_replaced_bodies() {
    support::worker(|| {
        for source in [
            "module z.dep fn unused()->Field{missing} pub fn good()->Field{7}",
            "module z.dep pub fn good()->Field{missing} pub fn good()->Field{7}",
            "module z.dep pub fn unused()->Field{missing} pub fn good()->Field{7}",
        ] {
            rejected(
                "program sample use z.dep fn main()->Field{dep.good()}",
                &[("a.padding", "module a.padding"), ("z.dep", source)],
                "z.dep",
                "missing",
            );
        }
        rejected(
            "program sample use z.dep fn main()->Field{7}",
            &[
                ("a.padding", "module a.padding"),
                (
                    "z.dep",
                    "module z.dep fn f()->Field{f()} pub fn f()->Bool{true}",
                ),
            ],
            "z.dep",
            "f()",
        );
        rejected(
            "program sample use z.dep fn main()->Field{7}",
            &[("z.dep", "module z.dep #[pure] pub fn f()->Field{ram_read()} fn ram_read()->Field{7} pub fn f()->Field{9}")],
            "z.dep", "ram_read",
        );
        rejected(
            "program sample use z.dep fn main()->Field{7}",
            &[
                ("a.padding", "module a.padding"),
                ("z.dep", "module z.dep fn f()->Field{f()}"),
            ],
            "z.dep",
            "f",
        );
        agrees(
            "program sample use dep fn main()->Field{dep.f()}",
            &[("dep", "module dep fn f()->Field{f()} pub fn f()->Field{7}")],
            7,
        );
    });
}

#[test]
fn imported_call_namespace_is_distinct_from_constants_and_lexical_module_roots() {
    support::worker(|| {
        let deps = [(
            "dep",
            "// foreign declaration spans\nmodule dep pub const f:Field=3 pub fn f()->Field{7}",
        )];
        agrees(
            "program sample use dep fn main()->Field{dep.f()*10+dep.f}",
            &deps,
            73,
        );
        agrees(
            "program sample use dep fn main()->Field{let dep=9 dep.f()+dep}",
            &deps,
            16,
        );
        agrees(
            "program sample use dep struct S{f:Field} fn main()->Field{let dep=S{f:9} dep.f()*10+dep.f}",
            &deps,
            79,
        );
        agrees(
            "program sample use dep fn main()->Field{let dep=9 dep // ж\n . f()+dep}",
            &deps,
            16,
        );
    });
}

#[test]
fn imported_ordinary_asserts_keep_call_semantics_and_purity_uses_the_member_name() {
    support::worker(|| {
        let deps = [("dep", "module dep pub fn assert(x:Bool){}")];
        agrees(
            "program sample use dep fn main()->Field{dep.assert(false) 7}",
            &deps,
            7,
        );
        rejected(
            "program sample use dep fn main()->Field{dep.assert(false)}",
            &deps,
            "sample",
            "dep.assert(false)",
        );
        agrees(
            "program sample use dep fn main()->Field{dep.assert(false)}",
            &[("dep", "module dep pub fn assert(x:Bool)->Field{7}")],
            7,
        );
        let deps = [(
            "dep",
            "module dep pub fn ram_read()->Field{7} pub fn wrapper()->Field{ram_read()}",
        )];
        rejected(
            "program sample use dep #[pure] fn main()->Field{dep.ram_read()}",
            &deps,
            "sample",
            "dep.ram_read",
        );
        agrees(
            "program sample use dep #[pure] fn main()->Field{dep.wrapper()}",
            &deps,
            7,
        );
    });
}

#[test]
fn imported_calls_evaluate_arguments_once_and_preserve_distinct_trap_order() {
    support::worker(|| {
        let deps = [(
            "dep",
            "module dep pub fn pick(a:Field,b:Field)->Field{a*10+b}",
        )];
        let bytes = agrees(
            "program sample use dep fn main()->Field{dep.pick(sub(9,2),sub(8,3))}",
            &deps,
            75,
        );
        let (value, trace) = support::trace_artifact(&bytes);
        assert_eq!(value, 75);
        assert_eq!(trace.0.iter().filter(|row| row.col(0) == 6).count(), 2);
        let a = "as_field(as_u32(4294967296))";
        let b = "nox_noun_as_field(nox_noun_head(nox_noun_atom(0)))";
        for (left, right, expected) in [(a, b, "Error(InvZero)"), (b, a, "Error(AxisError)")] {
            let entry =
                format!("program sample use dep fn main()->Field{{dep.pick({left},{right})}}");
            let modules = package(&entry, &deps);
            let bytes = program(&modules); // Compilation must succeed first.
            assert_eq!(support::try_run_artifact(&bytes), Err(expected.into()));
            assert_eq!(
                data::run(&seed(&modules).unwrap(), &data::Noun::Atom(0)),
                Err(expected.into())
            );
        }
    });
}

#[test]
fn imported_function_artifacts_ignore_declaration_discovery_and_unused_definitions() {
    support::worker(|| {
        let first = agrees(
            "program sample use dep fn main()->Field{dep.g(3)}",
            &[(
                "dep",
                "module dep fn f(x:Field)->Field{x+1} pub fn g(x:Field)->Field{f(x)*2}",
            )],
            8,
        );
        let reordered = agrees(
            "program sample use dep fn unused()->Field{99} fn main()->Field{dep.g(3)}",
            &[("dep", "module dep fn unused()->Field{42} pub fn g(x:Field)->Field{f(x)*2} fn f(x:Field)->Field{x+1}")],
            8,
        );
        assert_eq!(first, reordered);
        let main_only = agrees(
            "program sample use dep fn main()->Field{7}",
            &[("dep", "module dep pub fn unused()->Field{9}")],
            7,
        );
        assert_eq!(
            main_only,
            data::compile("program sample fn main()->Field{7}")
        );
    });
}

#[test]
fn imported_table_order_compares_owner_then_member_without_entry_privilege() {
    support::worker(|| {
        let deps = [
            ("a", "module a pub fn z()->Field{7}"),
            ("a.b", "module a.b pub fn f()->Field{9}"),
        ];
        let bytes = agrees(
            "program sample use a use a.b fn main()->Field{a.z()*10+a.b.f()}",
            &deps,
            79,
        );
        // Alpha-renaming to one owner preserves the abstract function order:
        // (a,z), (a.b,f), (sample,main). Sorting dotted strings swaps the first two.
        assert_eq!(
            bytes,
            data::compile(
                "program sample fn aa()->Field{7} fn bb()->Field{9} fn main()->Field{aa()*10+bb()}"
            )
        );
        let reversed = agrees(
            "program sample use a.b use a fn main()->Field{a.z()*10+a.b.f()}",
            &deps,
            79,
        );
        assert_eq!(bytes, reversed);
        let bytes = agrees(
            "program sample use z.dep fn main()->Field{dep.f()}",
            &[("z.dep", "module z.dep pub fn f()->Field{7}")],
            7,
        );
        assert_eq!(
            bytes,
            data::compile("program sample fn main()->Field{zz()} fn zz()->Field{7}")
        );
    });
}
