#[path = "native_control/support.rs"]
mod support;
#[path = "support/mod.rs"]
mod target_support;

use std::{collections::BTreeSet, path::Path};
use trident::{tir::TIROp, CompileOptions};

fn project(root: &Path, definitions: &str, body: &str, raw: bool) -> std::path::PathBuf {
    std::fs::create_dir_all(root.join("bank")).unwrap();
    std::fs::write(
        root.join("bank/values.tri"),
        format!("module bank.values\n{definitions}"),
    )
    .unwrap();
    let functions = if raw {
        format!("fn result()->Field{{{body}}} fn main(input:Noun)->Noun{{nox_noun_atom(result())}}")
    } else {
        format!("fn main()->Field{{{body}}}")
    };
    let path = root.join("entry.tri");
    std::fs::write(
        &path,
        format!("program entry\nuse bank.values\n{functions}"),
    )
    .unwrap();
    path
}

fn accepts(definitions: &str, body: &str, expected: u64) {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path(), definitions, body, true);
    let program = trident::compile_raw_artifact_project(
        &path,
        &CompileOptions::default(),
        trident::RAW_ARTIFACT_LIMITS,
    )
    .unwrap();
    let value = support::run(&program.bytes, 0, 1_000_000, 16384, 196608)
        .unwrap()
        .0;
    assert_eq!(value, expected, "{definitions}: {body}");
    let path = project(dir.path(), definitions, body, false);
    let source = std::fs::read_to_string(&path).unwrap();
    assert!(trident::check_file_in_project(&source, &path).is_ok());
    let options = CompileOptions::default()
        .with_package(target_support::triton_package())
        .unwrap();
    for module in trident::build_tir_modules(&path, &options).unwrap() {
        let mut labels = BTreeSet::new();
        for op in module.ops {
            if let TIROp::FnStart(name) = op {
                assert!(
                    labels.insert(name),
                    "duplicate callable label in {}",
                    module.name
                );
            }
        }
    }
}

#[test]
fn final_private_callables_reject_through_native_stack_and_editor_apis() {
    support::worker(|| {
        for (definitions, body) in [
            ("pub fn f()->Field{7} fn f()->Bool{true}", "values.f()"),
            (
                "pub fn f<N>(x:[Field;N])->Field{x[0]} fn f<N>(x:[Field;N])->Field{x[0]}",
                "values.f<1>([7])",
            ),
            (
                "pub fn f(x:Field)->Field{x} fn f<N>(x:[Field;N])->Field{x[0]}",
                "values.f<1>([7])",
            ),
            (
                "pub fn f<N>(x:[Field;N])->Field{x[0]} fn f(x:Field)->Field{x}",
                "values.f(7)",
            ),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = project(dir.path(), definitions, body, true);
            assert!(trident::compile_raw_artifact_project(
                &path,
                &CompileOptions::default(),
                trident::RAW_ARTIFACT_LIMITS
            )
            .is_err());
            let path = project(dir.path(), definitions, body, false);
            let source = std::fs::read_to_string(&path).unwrap();
            assert!(trident::compile_to_bundle(&path, &CompileOptions::default()).is_err());
            assert!(trident::check_file_in_project(&source, &path).is_err());
            let options = CompileOptions::default()
                .with_package(target_support::triton_package())
                .unwrap();
            assert!(trident::build_tir_modules(&path, &options).is_err());
        }
    });
}

#[test]
fn exported_final_bodies_and_callable_kinds_agree_across_backends() {
    support::worker(|| {
        for (definitions, body) in [
            ("pub fn f()->Field{99} pub fn f(x:Bool)->Bool{x}", "if values.f(true){7}else{9}"),
            ("fn f()->Bool{false} pub fn f(x:Field)->Field{x+2}", "values.f(5)"),
            ("pub fn f()->Field{99} fn f()->Bool{true} pub fn wrapper()->Field{if f(){7}else{9}}", "values.wrapper()"),
            ("pub fn f(x:Field)->Field{x} pub fn f<N>(x:[Field;N])->Field{x[0]}", "values.f<1>([7])"),
            ("pub fn f<N>(x:[Field;N])->Field{x[0]} pub fn f(x:Field)->Field{x+2}", "values.f(5)"),
        ] {
            accepts(definitions, body, 7);
        }
    });
}

#[test]
fn generic_instances_use_the_final_body_and_its_parameter_names() {
    support::worker(|| {
        for first in [
            "pub fn f<N>(x:[Field;N])->Field{1}",
            "fn f<N>(x:[Field;N])->Field{1}",
        ] {
            accepts(
                &format!("{first} pub fn f<M>(values:[Field;M])->Field{{values[0]}}"),
                "values.f<1>([7])",
                7,
            );
        }
    });
}

#[test]
fn replacing_functions_preserves_validation_of_earlier_ordinary_bodies() {
    for first in [
        "pub fn f()->Field{true}",
        "#[pure] pub fn f()->Field{ram_read(0)}",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = project(
            dir.path(),
            &format!("{first} pub fn f()->Field{{7}}"),
            "values.f()",
            false,
        );
        let options = CompileOptions::default()
            .with_package(target_support::triton_package())
            .unwrap();
        assert!(trident::build_tir_modules(&path, &options).is_err());
    }
}

#[test]
fn final_entry_owns_stack_input_layout_and_is_emitted_once() {
    let source = "program entry\nfn main(x:Field)->Field{x}\nfn main(xs:[Field;2])->Field{xs[1]}";
    let options = CompileOptions::default()
        .with_package(target_support::triton_package())
        .unwrap();
    let ir = trident::build_tir(source, "entry.tri", &options).unwrap();
    assert_eq!(
        ir.iter()
            .filter(|op| matches!(op,TIROp::FnStart(name) if name=="main"))
            .count(),
        1
    );
    let widths: Vec<_> = ir
        .iter()
        .filter_map(|op| match op {
            TIROp::EntryParameters(leaves) => Some(leaves.len()),
            _ => None,
        })
        .collect();
    assert_eq!(widths, vec![2]);
}

#[test]
fn bundle_metadata_and_entry_follow_final_active_definitions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("entry.tri");
    std::fs::write(
        &path,
        "module entry pub fn first()->Field{1} pub fn second()->Field{2} fn first()->Bool{true}",
    )
    .unwrap();
    let bundle = trident::compile_to_bundle(&path, &CompileOptions::default()).unwrap();
    assert_eq!(bundle.entry_point, "second");
    assert_eq!(bundle.functions.len(), 2);
    let first: Vec<_> = bundle
        .functions
        .iter()
        .filter(|f| f.name == "first")
        .collect();
    assert_eq!(first.len(), 1);
    assert!(first[0].signature.contains("Bool"));
    std::fs::write(&path, "module entry pub fn f()->Field{7}").unwrap();
    let active = trident::compile_to_bundle(&path, &CompileOptions::for_profile("debug")).unwrap();
    std::fs::write(
        &path,
        "module entry pub fn f()->Field{7} #[cfg(release)] pub fn f()->Bool{false}",
    )
    .unwrap();
    let conditional =
        trident::compile_to_bundle(&path, &CompileOptions::for_profile("debug")).unwrap();
    assert_eq!(conditional.functions.len(), 1);
    assert_eq!(active.functions[0].hash, conditional.functions[0].hash);
    assert_eq!(
        active.functions[0].signature,
        conditional.functions[0].signature
    );
    assert_eq!(active.assembly, conditional.assembly);
    assert_ne!(active.source_hash, conditional.source_hash);
}

#[test]
fn test_execution_selects_final_annotations_without_masking_active_failures() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("checks.tri");
    let stack = CompileOptions::for_profile("debug")
        .with_package(target_support::triton_package())
        .unwrap();
    for (definitions, count, skipped) in [
        ("#[test] fn sample(){assert(false)} fn sample(){}", 0, 0),
        ("fn sample(){assert(false)} #[test] fn sample(){}", 1, 0),
        (
            "#[test] fn sample(){assert(false)} #[test] fn sample(){}",
            1,
            0,
        ),
        (
            "#[test] fn sample(){} #[cfg(release)] fn sample(){assert(false)}",
            1,
            0,
        ),
        (
            "#[test] fn sample(){} #[cfg(release)] #[test] fn sample(){assert(false)}",
            1,
            1,
        ),
    ] {
        std::fs::write(&path, format!("module checks {definitions}")).unwrap();
        let report = trident::run_tests(&path, &CompileOptions::for_profile("debug")).unwrap();
        assert_eq!(
            report.matches("test checks.sample ... ok").count(),
            count,
            "{report}"
        );
        assert!(report.contains(&format!("{skipped} skipped")), "{report}");
        let tests = trident::prepare_test_programs(&path, &stack).unwrap();
        assert_eq!(tests.tests.len(), count);
        assert_eq!(tests.skipped, skipped);
    }
    std::fs::write(
        &path,
        "module checks #[test] fn sample(){} #[test] fn sample(){assert(false)}",
    )
    .unwrap();
    let error = trident::run_tests(&path, &CompileOptions::default()).unwrap_err();
    assert!(error
        .iter()
        .any(|d| d.message.contains("0 passed; 1 failed")));
}
