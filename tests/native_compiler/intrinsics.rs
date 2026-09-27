use super::{nouns::data, support};
use data::Noun::{self, Atom};
use support::{schema, Compilation};

#[path = "intrinsic_calls.rs"]
mod calls;
#[path = "intrinsic_vm.rs"]
mod vm;

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

fn program(modules: &[schema::Module]) -> Vec<u8> {
    match compile(modules) {
        Compilation::Program { bytes, .. } => bytes,
        other => panic!("modules={modules:?}: {other:?}"),
    }
}

// Only the independent seed oracle writes a temporary source project. The
// guest receives the original, separate MOD1 sources and resolves them itself.
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
        &trident::CompileOptions {
            dep_dirs: vec![dir.path().to_path_buf()],
            ..Default::default()
        },
        trident::NativeArtifactProfile::RawNoun,
        trident::NATIVE_ARTIFACT_LIMITS,
    )
    .map(|artifact| artifact.bytes)
    .map_err(|errors| format!("{errors:?}"))
}

fn agrees(entry: &str, dependencies: &[(&str, &str)], input: &Noun, expected: &Noun) -> Vec<u8> {
    let modules = package(entry, dependencies);
    let bytes = program(&modules);
    for code in [&bytes, &seed(&modules).unwrap()] {
        assert_eq!(
            data::run(code, input).unwrap().bytes,
            expected.encoded(),
            "{entry}"
        );
    }
    bytes
}

fn scalar(entry: &str, dependencies: &[(&str, &str)], expected: u64) -> Vec<u8> {
    agrees(entry, dependencies, &Atom(0), &Atom(expected))
}

fn error(entry: &str, dependencies: &[(&str, &str)], owner: &str, code: u32) -> schema::Diagnostic {
    let modules = package(entry, dependencies);
    let errors = match compile(&modules) {
        Compilation::Errors(errors) => errors,
        other => panic!("accepted modules={modules:?}: {other:?}"),
    };
    assert_eq!(errors.len(), 1, "{entry}: {errors:?}");
    let error = errors.into_iter().next().unwrap();
    let index = modules
        .iter()
        .position(|module| module.path == owner)
        .unwrap();
    assert_eq!(
        (error.code, error.module),
        (code, index as u32),
        "{entry}: {error:?}"
    );
    assert!(error.start <= error.end && error.end as usize <= modules[index].source.len());
    error
}

fn reject_at(
    entry: &str,
    dependencies: &[(&str, &str)],
    owner: &str,
    context: &str,
    span: &str,
    code: u32,
) {
    let error = error(entry, dependencies, owner, code);
    let source = if owner == "sample" {
        entry
    } else {
        dependencies
            .iter()
            .find(|(path, _)| *path == owner)
            .unwrap()
            .1
    };
    assert_eq!(
        source.matches(context).count(),
        1,
        "ambiguous test context: {context}"
    );
    let start = source.find(context).unwrap() + context.find(span).unwrap();
    assert_eq!(
        (error.start, error.end),
        (start as u32, (start + span.len()) as u32),
        "{source}: {error:?}"
    );
}

#[test]
fn guest_known_only_intrinsics_are_typed_but_rejected_only_when_reachable() {
    support::worker(|| {
        for (kind, parameters, result, arguments, body) in [
            ("field_add", "a:Field,b:Field", "Field", "7,9", "f(7,9)"),
            ("field_mul", "a:Field,b:Field", "Field", "7,9", "f(7,9)"),
            ("neg", "a:Field", "Field", "7", "f(7)"),
            ("inv", "a:Field", "Field", "7", "f(7)"),
            (
                "split",
                "a:Field",
                "(U32,U32)",
                "7",
                "let(a,b)=f(7) as_field(a)+as_field(b)",
            ),
        ] {
            let source = format!("// distinct owner offsets\nmodule ext.ops #[intrinsic({kind})] pub fn f({parameters})->{result} pub fn unused()->Field{{{body}}}");
            let dependencies = [("ext.ops", source.as_str())];
            let entry = "program sample use ext.ops fn main()->Field{7}";
            assert_eq!(
                data::run(&program(&package(entry, &dependencies)), &Atom(0))
                    .unwrap()
                    .bytes,
                Atom(7).encoded()
            );
            let call = format!("f({arguments})");
            reject_at(
                "program sample use ext.ops fn main()->Field{ops.unused()}",
                &dependencies,
                "ext.ops",
                body,
                &call,
                6,
            );
            let direct = if kind == "split" {
                "let(a,b)=ops.f(7) as_field(a)+as_field(b)".into()
            } else {
                format!("ops.f({arguments})")
            };
            let entry = format!("program sample use ext.ops fn main()->Field{{{direct}}}");
            reject_at(
                &entry,
                &dependencies,
                "sample",
                &direct,
                &format!("ops.f({arguments})"),
                6,
            );
        }
        let dependencies = [
            (
                "ext.ops",
                "module ext.ops #[intrinsic(inv)] pub fn inverse(a:Field)->Field",
            ),
            (
                "a",
                "// origin\nmodule a use ext.ops pub fn leaf()->Field{ops.inverse(7)}",
            ),
            ("b", "module b use a pub fn branch()->Field{a.leaf()}"),
            ("z", "module z use a pub fn branch()->Field{a.leaf()}"),
        ];
        reject_at(
            "program sample use b use z fn main()->Field{b.branch()+z.branch()}",
            &dependencies,
            "a",
            "ops.inverse(7)",
            "ops.inverse(7)",
            6,
        );
        // Even an unreachable known-only call must have the registered ABI.
        reject_at(
            "program sample use ext.ops fn main()->Field{7}",
            &[(
                "ext.ops",
                "module ext.ops #[intrinsic(inv)] fn f(a:Field)->Field fn unused()->Field{f(true)}",
            )],
            "ext.ops",
            "f(true)",
            "true",
            5,
        );
    });
}

#[test]
fn guest_intrinsic_identity_follows_final_callable_and_direct_alias_binding() {
    support::worker(|| {
        for (declarations, call, expected) in [
            ("#[intrinsic(sub)] pub fn pick(a:Field,b:Field)->Field pub fn pick(a:Field,b:Field)->Field{a+b}", "ops.pick(7,5)", 12),
            ("pub fn pick(a:Field,b:Field)->Field{a+b} #[intrinsic(sub)] pub fn pick(a:Field,b:Field)->Field", "ops.pick(7,5)", 2),
            ("#[intrinsic(sub)] pub fn pick(a:Field,b:Field)->Field pub fn pick(a:Field)->Field{a+9}", "ops.pick(7)", 16),
            ("pub fn pick(a:Field)->Field{a+9} #[intrinsic(sub)] pub fn pick(a:Field,b:Field)->Field", "ext.ops.pick(7,5)", 2),
        ] {
            let source = format!("module ext.ops {declarations}");
            scalar(&format!("program sample use ext.ops fn main()->Field{{{call}}}"), &[("ext.ops", &source)], expected);
        }
        let dependencies = [
            ("vm.same", "module vm.same #[intrinsic(sub)] pub fn pick(a:Field,b:Field)->Field pub fn keep()->Field{3}"),
            ("ext.same", "module ext.same pub fn pick(a:Field,b:Field)->Field{a+b} pub fn keep()->Field{8} fn keep()->Field{9}"),
        ];
        for (uses, expected) in [
            ("use vm.same use ext.same", 123),
            ("use ext.same use vm.same", 23),
            ("use vm.same use ext.same use vm.same", 23),
        ] {
            scalar(
                &format!("program sample {uses} fn main()->Field{{same.pick(7,5)*10+same.keep()}}"),
                &dependencies,
                expected,
            );
        }
        scalar("program sample use vm.same use ext.same fn main()->Field{vm.same.pick(7,5)*100+ext.same.pick(7,5)}", &dependencies, 212);
        reject_at("program sample use ext.ops fn main()->Field{ops.pick(7)}",
            &[("ext.ops", "module ext.ops pub fn pick(a:Field)->Field{a} #[intrinsic(sub)] pub fn pick(a:Field,b:Field)->Field")],
            "sample", "ops.pick(7)", "ops.pick(7)", 5);
    });
}

#[test]
fn guest_intrinsics_do_not_escape_private_final_or_direct_import_visibility() {
    support::worker(|| {
        let dependencies = [
            ("ext.ops", "module ext.ops #[intrinsic(sub)] pub fn visible(a:Field,b:Field)->Field #[intrinsic(sub)] fn hidden(a:Field,b:Field)->Field #[intrinsic(sub)] pub fn gone(a:Field,b:Field)->Field #[intrinsic(sub)] fn gone(a:Field,b:Field)->Field"),
            ("wrapper", "module wrapper use ext.ops pub fn run()->Field{ops.visible(9,2)}"),
        ];
        scalar(
            "program sample use wrapper fn main()->Field{wrapper.run()}",
            &dependencies,
            7,
        );
        for (uses, call) in [
            ("use ext.ops", "ops.hidden(9,2)"),
            ("use ext.ops", "ops.gone(9,2)"),
            ("use ext.ops", "visible(9,2)"),
            ("use wrapper", "ops.visible(9,2)"),
            ("use wrapper", "ext.ops.visible(9,2)"),
        ] {
            let entry = format!("program sample {uses} fn main()->Field{{{call}}}");
            reject_at(
                &entry,
                &dependencies,
                "sample",
                call,
                call.split('(').next().unwrap(),
                5,
            );
        }
    });
}

#[test]
fn guest_intrinsic_abis_validate_private_and_replaced_declarations_at_attributes() {
    support::worker(|| {
        for (kind, signature) in [
            ("nox_noun_pair", "fn bad(a:Noun,b:Field)->Noun"),
            ("nox_noun_head", "fn bad(a:Field)->Noun"),
            ("nox_noun_identity", "fn bad(a:Noun)->Field"),
            ("assert", "fn bad(a:Bool)->Field"),
            ("assert_eq", "fn bad(a:Field,b:Bool)"),
            ("split", "fn bad(a:U32)->(U32,U32)"),
            ("split", "fn bad(a:Field,b:Field)->(U32,U32)"),
            ("split", "fn bad(a:Field)->(U32,Field)"),
            ("split", "fn bad(a:Field)->(Field,U32)"),
            (
                "sub",
                "pub fn bad(a:Field)->Field pub fn bad(a:Field)->Field{a}",
            ),
        ] {
            let attribute = format!("#[intrinsic({kind})]");
            let declaration = format!("{attribute} {signature}");
            let source = format!("// original module offsets\nmodule ext.ops {declaration}");
            reject_at(
                "program sample use ext.ops fn main()->Field{7}",
                &[("ext.ops", &source)],
                "ext.ops",
                &declaration,
                &attribute,
                5,
            );
        }
        for owner in ["dep", "extx.ops", "x.extx.ops", "Ext.ops"] {
            let source =
                format!("module {owner} #[intrinsic(sub)] pub fn bad(a:Field,b:Field)->Field");
            let entry = format!("program sample use {owner} fn main()->Field{{7}}");
            reject_at(
                &entry,
                &[(owner, &source)],
                owner,
                "#[intrinsic(sub)]",
                "#[intrinsic(sub)]",
                5,
            );
        }
        // JOB1's identifier-only program owner fails namespace validation first.
        reject_at(
            "program sample #[intrinsic(sub)] fn main(a:Field,b:Field)->Field",
            &[],
            "sample",
            "#[intrinsic(sub)]",
            "#[intrinsic(sub)]",
            5,
        );
        // The guest receives explicit logical owners. The seed filesystem
        // resolver remaps legacy .ext. owners into os.*, so this namespace
        // policy case checks the complete independent literal result directly.
        let modules = package("program sample use custom.ext.ops fn main()->Field{ops.pick(9,2)}",
            &[("custom.ext.ops", "module custom.ext.ops #[intrinsic(sub)] pub fn pick(left:Field,right:Field)->Field")]);
        assert_eq!(
            data::run(&program(&modules), &Atom(0)).unwrap().bytes,
            Atom(7).encoded()
        );
    });
}

#[test]
fn guest_intrinsic_attributes_keep_last_identity_and_lexical_error_precedence() {
    support::worker(|| {
        scalar("program sample use ext.ops fn main()->Field{ops.pick(9,2)}",
            &[("ext.ops", "module ext.ops #[intrinsic(not_supported)] #[pure] #[intrinsic(sub)] pub fn pick(a:Field,b:Field)->Field")], 7);
        for payload in ["not_supported", "vm.core.field.sub", "sub other", ""] {
            let attribute = format!("#[intrinsic({payload})]");
            let declaration =
                format!("#[intrinsic(sub)] {attribute} pub fn pick(a:Field,b:Field)->Field");
            let source = format!("module ext.ops {declaration}");
            reject_at(
                "program sample use ext.ops fn main()->Field{7}",
                &[("ext.ops", &source)],
                "ext.ops",
                &attribute,
                &attribute,
                6,
            );
        }
        for (declaration, code) in [
            ("#[intrinsic(sub] pub fn f(a:Field,b:Field)->Field", 2),
            ("#[intrinsic((sub)] pub fn f(a:Field,b:Field)->Field", 2),
            ("#[intrinsic(\0)] pub fn f(a:Field,b:Field)->Field", 1),
            (
                "#[test] #[intrinsic(sub)] pub fn f(a:Field,b:Field)->Field",
                6,
            ),
            (
                "#[pure()] #[intrinsic(sub)] pub fn f(a:Field,b:Field)->Field",
                6,
            ),
            ("#[intrinsic(sub)] pub fn f<N>(a:Field,b:Field)->Field", 6),
            ("#[intrinsic(sub)] pub fn f(a:Field,b:Field)->Field{a+b}", 6),
            ("#[intrinsic(assert)] pub fn f(a:Bool){}", 6),
        ] {
            let source = format!("module ext.ops {declaration}");
            error(
                "program sample use ext.ops fn main()->Field{7}",
                &[("ext.ops", &source)],
                "ext.ops",
                code,
            );
        }
    });
}

#[test]
fn guest_resolved_assertions_preserve_unit_calls_traps_and_literal_halting_only() {
    support::worker(|| {
        let dependencies = [("ext.guard", "module ext.guard #[intrinsic(assert)] pub fn stop(c:Bool) #[intrinsic(assert_eq)] pub fn equal(a:Field,b:Field) pub fn wrap(c:Bool){stop(c)}")];
        scalar(
            "program sample use ext.guard fn main()->Field{guard.stop(true) guard.equal(7,7) 7}",
            &dependencies,
            7,
        );
        for body in [
            "guard.stop(false)",
            "return ext.guard.stop((false))",
            "guard.equal(7,9) 7",
        ] {
            let entry = format!("program sample use ext.guard fn main()->Field{{{body}}}");
            let modules = package(&entry, &dependencies);
            for code in [program(&modules), seed(&modules).unwrap()] {
                assert_eq!(
                    data::run(&code, &Atom(0)),
                    Err("Error(InvZero)".into()),
                    "{entry}"
                );
            }
        }
        for body in ["guard.stop(1==0)", "guard.wrap(false)"] {
            let entry = format!("program sample use ext.guard fn main()->Field{{{body}}}");
            reject_at(&entry, &dependencies, "sample", body, body, 5);
            assert!(seed(&package(&entry, &dependencies)).is_err());
        }
        scalar(
            "program sample use ext.guard fn main()->Field{guard.assert(false) 7}",
            &[("ext.guard", "module ext.guard pub fn assert(c:Bool){}")],
            7,
        );
        scalar(
            "program sample use ext.guard fn main()->Field{guard.stop(false) 7}",
            &[(
                "ext.guard",
                "module ext.guard #[intrinsic(assert)] pub fn stop(c:Bool) pub fn stop(c:Bool){}",
            )],
            7,
        );
        reject_at(
            "program sample use ext.guard fn main()->Field{guard.stop(false)}",
            &[(
                "ext.guard",
                "module ext.guard #[intrinsic(assert)] pub fn stop(c:Bool) pub fn stop(c:Bool){}",
            )],
            "sample",
            "guard.stop(false)",
            "guard.stop(false)",
            5,
        );
    });
}
