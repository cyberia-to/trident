#[path = "native_control/support.rs"]
mod support;
#[path = "support/mod.rs"]
mod target_support;

use std::collections::BTreeSet;
use std::path::Path;
use trident::ir::tree::lower::nox::NoxCompiler;
use trident::{parse_source_silent, CompileOptions};

const ERROR: &str = "incompatible redeclaration of struct";

fn debug_flags() -> BTreeSet<String> {
    BTreeSet::from(["debug".into()])
}

fn direct_rejects(sources: &[&str], message: &str) {
    let files: Vec<_> = sources
        .iter()
        .map(|source| parse_source_silent(source, "source.tri").unwrap())
        .collect();
    let refs: Vec<_> = files.iter().collect();
    let entry = files.last().unwrap();
    let flags = debug_flags();
    for result in [
        NoxCompiler::new().compile_modules(&refs, entry, &flags),
        NoxCompiler::new().compile_raw_modules(&refs, entry, &flags),
    ] {
        let error = result.unwrap_err();
        assert!(error.contains(message), "{error}");
    }
    let options = CompileOptions::default()
        .with_package(target_support::triton_package())
        .unwrap();
    let errors = trident::tir::builder::TIRBuilder::new(options.target_config)
        .with_module_types(&refs)
        .with_cfg_flags(flags)
        .build_file(entry)
        .unwrap_err();
    assert!(
        errors.iter().any(|error| error.message.contains(message)),
        "{errors:?}"
    );
}

fn changed_layout(definitions: &str, name: &str) {
    let source = format!("program app {definitions} fn main()->Field{{7}}");
    let errors = trident::check_silent(&source, "entry.tri").unwrap_err();
    let diagnostic = errors
        .iter()
        .find(|error| error.message.contains(ERROR))
        .unwrap_or_else(|| panic!("{errors:?}"));
    let start = source.rfind(&format!("struct {name}")).unwrap() + "struct ".len();
    assert_eq!(diagnostic.span.start as usize, start);
    assert_eq!(diagnostic.span.end as usize, start + name.len());
    assert!(diagnostic.message.contains(&format!("'app.{name}'")));
    direct_rejects(&[&source], ERROR);
}

fn write(root: &Path, name: &str, source: &str) {
    let path = root.join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, source).unwrap();
}

fn accepted_project(root: &Path, uses: &str, body: &str, expected: u64) {
    let path = root.join("entry.tri");
    let source = format!(
        "program app {uses} fn result()->Field{{{body}}} \
         fn main(input:Noun)->Noun{{nox_noun_atom(result())}}"
    );
    write(root, "entry.tri", &source);
    let artifact = trident::compile_raw_artifact_project(
        &path,
        &CompileOptions::default(),
        trident::RAW_ARTIFACT_LIMITS,
    )
    .unwrap();
    assert_eq!(
        support::run(&artifact.bytes, 0, 1_000_000, 16384, 196608)
            .unwrap()
            .0,
        expected
    );
    let source = format!("program app {uses} fn main()->Field{{{body}}}");
    write(root, "entry.tri", &source);
    trident::check_file_in_project(&source, &path).unwrap();
    let options = CompileOptions::default()
        .with_package(target_support::triton_package())
        .unwrap();
    let modules = trident::build_tir_modules(&path, &options).unwrap();
    assert!(!format!("{modules:?}").contains("ERROR:"));
}

#[test]
fn changed_layouts_are_rejected_before_all_lowerings() {
    for definitions in [
        "struct S{x:Field} struct S{x:Bool}",
        "struct S{x:Field} struct S{x:U32}",
        "struct S{x:Field} struct S{x:Field,y:Field}",
        "struct S{x:Field,y:Field} struct S{x:Field}",
        "struct S{x:Field} struct S{y:Field}",
        "struct S{x:Field,y:Field} struct S{y:Field,x:Field}",
        "struct S{pub x:Field} struct S{x:Field}",
        "struct S{x:Field} struct S{pub x:Field}",
        "struct S{x:[Field;2]} struct S{x:[Field;3]}",
        "struct S{x:(Field,Bool)} struct S{x:(Bool,Field)}",
        "struct A{x:Field} struct B{x:Field} struct S{x:A} struct S{x:B}",
    ] {
        changed_layout(definitions, "S");
    }
}

#[test]
fn nested_snapshots_cannot_turn_into_a_different_layout_or_cycle() {
    for definitions in [
        "struct A{x:Field} struct B{a:A} struct A{b:B}",
        "struct A{x:Field} struct B{a:A} struct A{x:Bool}",
        "struct A{pub x:Field} struct B{a:A} struct A{x:Field}",
    ] {
        changed_layout(definitions, "A");
    }
}

#[test]
fn inactive_changes_are_ignored_but_active_changes_fail() {
    changed_layout("struct S{x:Field} #[cfg(debug)] struct S{x:Bool}", "S");
    let source = "program app struct S{x:Field} #[cfg(absent)] struct S{x:Bool} \
                  fn main(input:Noun)->Noun{let s=S{x:7} nox_noun_atom(s.x)}";
    assert_eq!(support::evaluate(source, 0), 7);
    let file = parse_source_silent(source, "entry.tri").unwrap();
    NoxCompiler::new()
        .compile_raw_modules(&[&file], &file, &debug_flags())
        .unwrap();
}

#[test]
fn repeated_layouts_accept_checked_equal_array_extents() {
    let source = "program app const N:U32=2 \
        struct S{pub x:([Field;N+1],Bool)} \
        fn make()->S{S{x:([3,5,7],true)}} \
        pub struct S{pub x:([Field;3],Bool)} \
        fn main(input:Noun)->Noun{let s=make() let (words,flag)=s.x nox_noun_atom(words[2])}";
    assert_eq!(support::evaluate(source, 0), 7);
    let source = source.replace(
        "fn main(input:Noun)->Noun{let s=make() let (words,flag)=s.x nox_noun_atom(words[2])}",
        "fn main()->Field{let s=make() let (words,flag)=s.x words[2]}",
    );
    trident::check_silent(&source, "entry.tri").unwrap();
    let file = parse_source_silent(&source, "entry.tri").unwrap();
    NoxCompiler::new().compile_file(&file).unwrap();
    let options = CompileOptions::default()
        .with_package(target_support::triton_package())
        .unwrap();
    let ops = trident::tir::builder::TIRBuilder::new(options.target_config)
        .build_file(&file)
        .unwrap();
    assert!(!format!("{ops:?}").contains("ERROR:"));
}

#[test]
fn normalization_never_saturates_overflowed_array_extents() {
    let source = "program app struct S{x:[Field;18446744073709551615+1]} \
                  struct S{x:[Field;18446744073709551615]} fn main()->Field{7}";
    let errors = trident::check_silent(source, "entry.tri").unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("array size addition overflow")));
    direct_rejects(&[source], "array size addition overflow");
}

#[test]
fn full_and_short_imported_names_have_the_same_nominal_identity() {
    support::worker(|| {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "bank/types.tri",
            "module bank.types pub struct Item{pub value:Field}",
        );
        write(
            dir.path(),
            "value.tri",
            "module value use bank.types const N:U32=2 \
            struct S{pub item:bank.types.Item,pub words:[Field;N+1]} \
            pub fn make()->S{S{item:types.Item{value:7},words:[3,5,11]}} \
            pub struct S{pub item:types.Item,pub words:[Field;3]}",
        );
        accepted_project(
            dir.path(),
            "use value",
            "let s=value.make() s.item.value+s.words[2]",
            18,
        );
    });
}

#[test]
fn identical_fields_from_different_owners_are_different_types() {
    support::worker(|| {
        let dir = tempfile::tempdir().unwrap();
        let left = "module left.types pub struct Item{pub value:Field}";
        let right = "module right.types pub struct Item{pub value:Field}";
        let source = "program app use left.types use right.types \
                      struct S{x:left.types.Item} struct S{x:types.Item} fn main()->Field{7}";
        write(dir.path(), "left/types.tri", left);
        write(dir.path(), "right/types.tri", right);
        write(dir.path(), "entry.tri", source);
        let errors =
            trident::check_file_in_project(source, &dir.path().join("entry.tri")).unwrap_err();
        assert!(
            errors.iter().any(|error| error.message.contains(ERROR)),
            "{errors:?}"
        );
        direct_rejects(&[left, right, source], ERROR);
    });
}

#[test]
fn stable_layouts_allow_final_visibility_and_opaque_returns() {
    support::worker(|| {
        let dir = tempfile::tempdir().unwrap();
        for declarations in [
            "struct S{pub x:Field,secret:Field} pub struct S{pub x:Field,secret:Field}",
            "pub struct S{pub x:Field,secret:Field} struct S{pub x:Field,secret:Field}",
        ] {
            write(
                dir.path(),
                "value.tri",
                &format!("module value {declarations} pub fn make()->S{{S{{x:7,secret:11}}}}"),
            );
            accepted_project(dir.path(), "use value", "let s=value.make() s.x", 7);
            let source = "program app use value fn main()->Field{let s=value.make() s.secret}";
            write(dir.path(), "entry.tri", source);
            let errors =
                trident::check_file_in_project(source, &dir.path().join("entry.tri")).unwrap_err();
            assert!(
                errors.iter().any(|error| error.message.contains("private")),
                "{errors:?}"
            );
        }
        let source = "program app use value fn main()->Field{let s:value.S=value.make() s.x}";
        write(dir.path(), "entry.tri", source);
        assert!(trident::check_file_in_project(source, &dir.path().join("entry.tri")).is_err());
    });
}

#[test]
fn changed_private_or_replaced_dependency_layouts_still_fail() {
    support::worker(|| {
        let dir = tempfile::tempdir().unwrap();
        for (dependency, body) in [
            ("module value pub struct S{pub x:Field} pub fn make()->S{S{x:true}} pub struct S{pub x:Bool}", "value.make().x+1"),
            ("module value pub struct S{pub x:Field} pub fn make()->S{S{x:7}} pub struct S{x:Field}", "value.make().x"),
            ("module value struct S{x:Field} struct S{x:Bool} pub fn ordinary()->Field{7}", "7"),
        ] {
            write(dir.path(), "value.tri", dependency);
            let source = format!("program app use value fn main()->Field{{{body}}}");
            write(dir.path(), "entry.tri", &source);
            let errors = trident::check_file_in_project(&source, &dir.path().join("entry.tri")).unwrap_err();
            assert!(errors.iter().any(|error| error.message.contains(ERROR)), "{errors:?}");
            direct_rejects(&[dependency, &source], ERROR);
        }
    });
}

#[test]
fn repeated_names_do_not_enable_forward_type_references() {
    for source in [
        "program app fn make()->S{S{x:7}} struct S{x:Field} struct S{x:Field} fn main()->Field{7}",
        "program app struct S{x:T} struct T{x:Field} struct S{x:T} fn main()->Field{7}",
    ] {
        let errors = trident::check_silent(source, "entry.tri").unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("unknown type")),
            "{errors:?}"
        );
        assert!(
            !errors.iter().any(|error| error.message.contains(ERROR)),
            "{errors:?}"
        );
    }
}

#[test]
fn direct_layout_guard_does_not_add_full_type_checking() {
    let files: Vec<_> = [
        "module unused struct Box{value:Noun} pub fn ignored()->Field{true}",
        "program app struct S{x:Field} struct S{x:Field} fn main()->Field{7}",
    ]
    .iter()
    .map(|source| parse_source_silent(source, "source.tri").unwrap())
    .collect();
    let refs: Vec<_> = files.iter().collect();
    NoxCompiler::new()
        .compile_modules(&refs, &files[1], &debug_flags())
        .unwrap();
    let options = CompileOptions::default()
        .with_package(target_support::triton_package())
        .unwrap();
    trident::tir::builder::TIRBuilder::new(options.target_config)
        .with_module_types(&refs)
        .build_file(&files[1])
        .unwrap();
}
