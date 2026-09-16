//! Target and project context through actual nox execution.
use super::*;

fn run_formula(formula: &str, params: &[u64]) -> (u64, u64) {
    let mut arena = Reduction::<4096>::new();
    let f = load(&mut arena, &parse(formula));
    let s = load(&mut arena, &subject_noun(params));
    let budget = 5_000_000;
    match reduce(&mut arena, s, f, budget, &NullCalls, &mut NoTrace) {
        Outcome::Ok(r, remaining) => (
            arena.atom_value(r).expect("result is an atom").as_u64(),
            budget - remaining,
        ),
        other => panic!("reduction failed: {other:?}"),
    }
}

/// Project compilation, cost analysis and the warrior bundle must agree on
/// the selected source, while VM execution supplies the independent oracle.
fn assert_project(
    entry: &std::path::Path,
    options: &CompileOptions,
    params: &[u64],
    expected: u64,
) {
    let assembly = trident::compile_project_with_options(entry, options).unwrap();
    let (value, bill) = run_formula(&assembly, params);
    assert_eq!(value, expected);
    let cost = trident::nox_cost_project(entry, options).unwrap();
    assert!(
        (cost.bill.min..=cost.bill.max).contains(&bill),
        "{cost:?}, runtime {bill}"
    );
    let bundle = trident::compile_to_bundle(entry, options).unwrap();
    assert_eq!(run_formula(&bundle.assembly, params), (expected, bill));
    assert_eq!(bundle.cost.table_values, vec![cost.bill.max]);
}

#[test]
fn nox_cfg_selects_functions_constants_structs_and_entry() {
    let source = r#"program configured
#[cfg(debug)]
const VALUE: Field = 3
#[cfg(release)]
const VALUE: Field = 7
#[cfg(debug)]
struct Pair { x: Field, y: Field }
#[cfg(release)]
struct Pair { y: Field, x: Field }
#[cfg(debug)]
fn pick() -> Field { VALUE }
#[cfg(release)]
fn pick() -> Field { VALUE + 10 }
#[cfg(debug)]
fn main() -> Field { let p: Pair = Pair { x: pick(), y: 99 }
    p.x }
#[cfg(release)]
fn main() -> Field { let p: Pair = Pair { x: pick(), y: 99 }
    p.x + 100 }
"#;
    let dir = tempfile::tempdir().unwrap();
    let entry = dir.path().join("main.tri");
    std::fs::write(&entry, source).unwrap();
    for (profile, expected) in [("debug", 3), ("release", 117)] {
        let options = CompileOptions::for_profile(profile);
        let assembly = trident::compile_with_options(source, "main.tri", &options).unwrap();
        assert_eq!(run_formula(&assembly, &[]).0, expected);
        assert_project(&entry, &options, &[], expected);
    }
}

#[test]
fn nox_locals_and_parameters_shadow_module_constants() {
    let source = r#"program shadow
const X: Field = 5
fn value(X: Field) -> Field { X }
fn main() -> Field {
    let X: Field = 7
    value(X)
}
"#;
    assert_eq!(run(source, &[]), 7);
    let dir = tempfile::tempdir().unwrap();
    let entry = dir.path().join("main.tri");
    std::fs::write(&entry, source).unwrap();
    assert_project(&entry, &nox_options(), &[], 7);
}

#[test]
fn nox_shadowed_constant_does_not_replace_dynamic_loop_bound() {
    let source = r#"program bound
const N: U32 = 1
fn main(N: U32) -> Field {
    let mut sum: Field = 0
    for i in 0..N bounded 4 { sum = sum + 1 }
    sum
}
"#;
    assert_eq!(run(source, &[3]), 3);
    assert_eq!(run(source, &[0]), 0);
}

#[test]
fn nox_shadowed_constant_is_not_a_static_array_index() {
    let source = r#"program index
const INDEX: U32 = 0
fn main(INDEX: U32) -> Field {
    let a: [Field; 2] = [7, 9]
    a[INDEX]
}
"#;
    let errors = trident::compile_with_options(source, "index.tri", &nox_options()).unwrap_err();
    assert!(errors.iter().any(|e| e
        .message
        .contains("array index must be a compile-time constant")));
}

#[test]
fn nox_imports_preserve_qualified_names_and_callee_lexical_scope() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("pkg")).unwrap();
    std::fs::write(
        dir.path().join("pkg/left.tri"),
        r#"module pkg.left
const OFFSET: Field = 2
fn adjust(a: Field) -> Field { a + OFFSET }
pub fn value(a: Field) -> Field { adjust(a) }
"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("pkg/right.tri"),
        r#"module pkg.right
use pkg.left
const OFFSET: Field = 10
fn adjust(a: Field) -> Field { a * OFFSET }
pub fn value(a: Field) -> Field { adjust(a) + left.value(a) }
"#,
    )
    .unwrap();
    let entry = dir.path().join("main.tri");
    std::fs::write(
        &entry,
        r#"program imported
use pkg.left
use pkg.right
const OFFSET: Field = 1000
fn adjust(a: Field) -> Field { a + OFFSET }
fn main() -> Field { left.value(3) + pkg.right.value(4) + adjust(1) }
"#,
    )
    .unwrap();
    assert_project(&entry, &nox_options(), &[], 1052);
}

#[test]
fn nox_imported_cfg_and_module_constants_reach_all_output_paths() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("values.tri"),
        r#"module values
#[cfg(debug)]
pub const OFFSET: Field = 3
#[cfg(release)]
pub const OFFSET: Field = 20
#[cfg(debug)]
pub fn selected() -> Field { OFFSET }
#[cfg(release)]
pub fn selected() -> Field { OFFSET + 1 }
"#,
    )
    .unwrap();
    let entry = dir.path().join("main.tri");
    std::fs::write(
        &entry,
        "program imported\nuse values\nfn main() -> Field { values.selected() + values.OFFSET }\n",
    )
    .unwrap();
    assert_project(&entry, &CompileOptions::for_profile("debug"), &[], 6);
    assert_project(&entry, &CompileOptions::for_profile("release"), &[], 41);
}

#[test]
fn nox_imported_intrinsic_uses_declared_builtin() {
    let dir = tempfile::tempdir().unwrap();
    let entry = dir.path().join("main.tri");
    std::fs::write(&entry, "program intrinsics\nuse vm.core.field\nfn main() -> Field { field.mul(field.add(3, 4), 2) }\n").unwrap();
    assert_project(&entry, &nox_options(), &[], 14);
}

#[test]
fn nox_imported_struct_layouts_have_module_identity() {
    let dir = tempfile::tempdir().unwrap();
    for (name, fields, expected) in [
        ("left", "x: Field, y: Field", 3),
        ("right", "y: Field, x: Field", 7),
    ] {
        std::fs::write(dir.path().join(format!("{name}.tri")), format!(
            "module {name}\nstruct Pair {{ {fields} }}\npub fn value() -> Field {{ let p: Pair = Pair {{ x: {expected}, y: 99 }}\n p.x }}\n"
        )).unwrap();
    }
    let entry = dir.path().join("main.tri");
    std::fs::write(&entry, "program structs\nuse left\nuse right\nfn main() -> Field { left.value() + right.value() }\n").unwrap();
    assert_project(&entry, &nox_options(), &[], 10);
}

#[test]
fn nox_transitive_imported_state_read_preserves_root_in_all_artifacts() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("storage.tri"),
        r#"module storage
fn read_key(k: Field) -> Field { os.state.read(k) }
#[cfg(debug)]
pub fn read(k: Field) -> Field { k + 1 }
#[cfg(release)]
pub fn read(k: Field) -> Field { read_key(k) }
"#,
    )
    .unwrap();
    let entry = dir.path().join("main.tri");
    std::fs::write(
        &entry,
        "program state\nuse storage\nfn main(k: Field) -> Field { storage.read(k) }\n",
    )
    .unwrap();
    // Inactive state access must not change the entry's subject ABI.
    let debug = CompileOptions::for_profile("debug");
    let bundle = trident::compile_to_bundle(&entry, &debug).unwrap();
    assert!(!bundle.reads_state);
    assert_project(&entry, &debug, &[7], 8);

    let release = CompileOptions::for_profile("release");
    let assembly = trident::compile_project_with_options(&entry, &release).unwrap();
    let bundle = trident::compile_to_bundle(&entry, &release).unwrap();
    assert!(bundle.reads_state);
    assert_eq!(assembly, bundle.assembly);
    let (value, bill, reads) = run_state_formula(&assembly, &[7]);
    assert_eq!((value, reads), (77, 1));
    let cost = trident::nox_cost_project(&entry, &release).unwrap();
    assert!((cost.bill.min..=cost.bill.max).contains(&bill));
}

#[test]
fn foreign_tree_and_unknown_targets_never_fall_back_to_nox() {
    let source = "program terrain\nfn main(a: Field) -> Field { a + 2 }\n";
    let dir = tempfile::tempdir().unwrap();
    let entry = dir.path().join("main.tri");
    std::fs::write(&entry, source).unwrap();
    assert!(TerrainConfig::resolve("nock").is_err());
    let nock = TerrainConfig::load(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/catalog/vm/nock/target.toml"
    )))
    .unwrap();
    let mut unknown = TerrainConfig::nox();
    unknown.name = "unknown-tree".into();
    for terrain in [nock, unknown] {
        let mut options = nox_options();
        options.target_config = terrain;
        for errors in [
            trident::compile_with_options(source, "main.tri", &options).unwrap_err(),
            trident::compile_project_with_options(&entry, &options).unwrap_err(),
            trident::nox_cost_project(&entry, &options).unwrap_err(),
            trident::compile_to_bundle(&entry, &options).unwrap_err(),
        ] {
            assert!(errors
                .iter()
                .any(|e| e.message.contains("requires its own lowering")));
        }
    }
    for target in ["nock", "unknown-tree"] {
        let output = dir.path().join(format!("{target}.out"));
        let result = std::process::Command::new(env!("CARGO_BIN_EXE_trident"))
            .args([
                "build",
                entry.to_str().unwrap(),
                "--target",
                target,
                "-o",
                output.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            !result.status.success(),
            "foreign target unexpectedly built"
        );
        assert!(
            !output.exists(),
            "failed lowering must not create an artifact"
        );
    }
}

#[test]
fn bundle_entry_metadata_matches_executed_public_function() {
    let dir = tempfile::tempdir().unwrap();
    let entry = dir.path().join("entry.tri");
    std::fs::write(&entry, "module entry\nfn private_helper() -> Field { 999 }\npub fn run(x: Field) -> Field { x + 5 }\n").unwrap();
    let bundle = trident::compile_to_bundle(&entry, &CompileOptions::default()).unwrap();
    assert_eq!(bundle.entry_point, "run");
    let signature = &bundle
        .functions
        .iter()
        .find(|function| function.name == bundle.entry_point)
        .unwrap()
        .signature;
    assert!(signature.contains("x: Field"), "{signature}");
    let mut arena = Reduction::<4096>::new();
    let formula = load(&mut arena, &parse(&bundle.assembly));
    let subject = load(&mut arena, &subject_noun(&[4]));
    match reduce(&mut arena, subject, formula, 1000, &NullCalls, &mut NoTrace) {
        Outcome::Ok(result, _) => assert_eq!(arena.atom_value(result).unwrap().as_u64(), 9),
        result => panic!("{result:?}"),
    }
}

fn run_state_formula(formula: &str, params: &[u64]) -> (u64, u64, usize) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Provider(AtomicUsize);
    impl nox::LookProvider for Provider {
        fn look(&self, root: Goldilocks, ns: Goldilocks, key: Goldilocks) -> Option<Goldilocks> {
            assert_eq!(root.as_u64(), 11);
            assert_eq!(ns.as_u64(), 0);
            self.0.fetch_add(1, Ordering::SeqCst);
            Some(Goldilocks::new(key.as_u64() * 10 + 7))
        }
    }
    impl<const M: usize> nox::CallProvider<M> for Provider {
        fn provide(
            &self,
            _: &mut Reduction<M>,
            _: Goldilocks,
            _: nox::Order,
        ) -> Option<nox::Order> {
            None
        }
    }
    let mut arena = Reduction::<4096>::new();
    let formula = load(&mut arena, &parse(formula));
    let root = N::Cell(
        Box::new(N::Atom(11)),
        Box::new(N::Cell(
            Box::new(N::Atom(22)),
            Box::new(N::Cell(Box::new(N::Atom(33)), Box::new(N::Atom(44)))),
        )),
    );
    let subject = load(
        &mut arena,
        &N::Cell(Box::new(root), Box::new(subject_noun(params))),
    );
    let provider = Provider(AtomicUsize::new(0));
    let budget = 5_000_000;
    match reduce(
        &mut arena,
        subject,
        formula,
        budget,
        &provider,
        &mut NoTrace,
    ) {
        Outcome::Ok(result, remaining) => (
            arena.atom_value(result).unwrap().as_u64(),
            budget - remaining,
            provider.0.load(Ordering::SeqCst),
        ),
        outcome => panic!("state helper reduction failed: {outcome:?}"),
    }
}

#[test]
fn nox_state_root_survives_shadowing_helpers_and_inactive_branches() {
    let source = r#"program shadow_state
fn hash(k: Field) -> Field { os.state.read(k) }
fn plus(k: Field) -> Field { k + 1 }
fn helper(k: Field) -> Field { let offset: Field = 2
    hash(plus(k)) + offset }
fn main(k: Field) -> Field {
    let offset: Field = 100
    if k == 0 { return offset } else { return helper(k) + offset }
}
"#;
    let dir = tempfile::tempdir().unwrap();
    let entry = dir.path().join("main.tri");
    std::fs::write(&entry, source).unwrap();
    let bundle = trident::compile_to_bundle(&entry, &nox_options()).unwrap();
    assert!(bundle.reads_state);
    let (value, _, reads) = run_state_formula(&bundle.assembly, &[4]);
    assert_eq!((value, reads), (159, 1));
    let (value, _, reads) = run_state_formula(&bundle.assembly, &[0]);
    assert_eq!((value, reads), (100, 0));
}

#[test]
fn nox_cfg_entry_and_unreachable_state_helpers_preserve_stateless_abi() {
    let source = r#"program cfg_state
fn helper(k: Field) -> Field { os.state.read(k) }
#[cfg(debug)]
fn main(k: Field) -> Field { k + 1 }
#[cfg(release)]
fn main(k: Field) -> Field { helper(k) }
"#;
    let dir = tempfile::tempdir().unwrap();
    let entry = dir.path().join("main.tri");
    std::fs::write(&entry, source).unwrap();
    let debug = trident::compile_to_bundle(&entry, &CompileOptions::for_profile("debug")).unwrap();
    assert!(!debug.reads_state);
    assert_eq!(run_formula(&debug.assembly, &[4]).0, 5);
    let release =
        trident::compile_to_bundle(&entry, &CompileOptions::for_profile("release")).unwrap();
    assert!(release.reads_state);
    assert_eq!(run_state_formula(&release.assembly, &[4]).0, 47);
}

#[test]
fn portable_standard_poseidon_matches_upstream_on_nox() {
    std::thread::Builder::new().stack_size(64 * 1024 * 1024).spawn(|| {
        let directory = tempfile::tempdir().unwrap();
        let entry = directory.path().join("standard.tri");
        std::fs::write(&entry,"program standard\nuse std.crypto.poseidon\nfn main(a: Field) -> Field { poseidon.hash2(a,0) }\n").unwrap();
        let bundle = trident::compile_to_bundle(&entry,&nox_options()).unwrap();
        assert!(!bundle.reads_state);
        let mut arena = Reduction::<65536>::new();
        let formula = load(&mut arena,&parse(&bundle.assembly));
        let subject = load(&mut arena,&subject_noun(&[7]));
        match reduce(&mut arena,subject,formula,50_000_000,&NullCalls,&mut NoTrace) {
            // Frozen from p3-goldilocks0.4.2 Poseidon2GoldilocksHL<8>.
            Outcome::Ok(result,_) => assert_eq!(arena.atom_value(result).unwrap().as_u64(),13487548838448116774),
            outcome => panic!("Poseidon2 nox execution failed: {outcome:?}"),
        }
    }).unwrap().join().unwrap();
}

#[test]
fn imported_loop_return_keeps_caller_frame_and_cost_bounds() {
    let dir = tempfile::tempdir().unwrap();
    let entry = dir.path().join("main.tri");
    std::fs::write(dir.path().join("search.tri"), "module search\npub fn find(n: Field) -> (Field, Field) { for i in 0..3 { if as_field(i) == n { return (n, 7) } }\n (9, 11) }").unwrap();
    std::fs::write(&entry, "program caller\nuse search\nfn main(n: Field) -> Field { let keep: Field = 100\n let (a, b) = search.find(n)\n keep + a + b }").unwrap();
    for profile in ["debug", "release"] {
        let options = CompileOptions::for_profile(profile);
        assert_project(&entry, &options, &[1], 108);
        assert_project(&entry, &options, &[4], 120);
    }
}
