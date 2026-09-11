// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! End-to-end nox surface tests.
//!
//! Each test drives the FULL compiler pipeline — parse → typecheck → nox
//! lowering → printed noun — for the `nox` target, then parses the printed
//! formula back into a `nox::Reduction` and reduces it on the real VM,
//! asserting the numeric result. Nothing here checks formula shape alone;
//! every feature is exercised against `nox::reduce`.

use nebu::Goldilocks;
use nox::{reduce, NoTrace, NullCalls, Outcome, Reduction};
use trident::target::TerrainConfig;
use trident::CompileOptions;

/// A noun mirror for parsing the compiler's printed output.
enum N {
    Atom(u64),
    Cell(Box<N>, Box<N>),
}

fn nox_options() -> CompileOptions {
    let mut o = CompileOptions::default();
    o.target_config = TerrainConfig::resolve("nox").expect("nox target.toml must resolve");
    o
}

/// Parse the compiler's noun printer output `[a b]` / `42` into `N`.
fn parse(src: &str) -> N {
    let bytes = src.as_bytes();
    let mut pos = 0;
    let n = parse_at(bytes, &mut pos);
    assert_eq!(pos, bytes.len(), "trailing input: {:?}", &src[pos..]);
    n
}

fn parse_at(b: &[u8], pos: &mut usize) -> N {
    if b[*pos] == b'[' {
        *pos += 1; // '['
        let left = parse_at(b, pos);
        assert_eq!(b[*pos], b' ', "expected space between cell halves");
        *pos += 1; // ' '
        let right = parse_at(b, pos);
        assert_eq!(b[*pos], b']', "expected closing bracket");
        *pos += 1; // ']'
        N::Cell(Box::new(left), Box::new(right))
    } else {
        let start = *pos;
        while *pos < b.len() && b[*pos].is_ascii_digit() {
            *pos += 1;
        }
        let v: u64 = std::str::from_utf8(&b[start..*pos])
            .unwrap()
            .parse()
            .unwrap();
        N::Atom(v)
    }
}

fn load<const M: usize>(ar: &mut Reduction<M>, n: &N) -> nox::Order {
    match n {
        N::Atom(v) => ar.atom(Goldilocks::new(*v)).expect("arena full"),
        N::Cell(h, t) => {
            let h = load(ar, h);
            let t = load(ar, t);
            ar.pair(h, t).expect("arena full")
        }
    }
}

/// Build the entry subject `[p_last .. [p0 0]]`.
fn subject_noun(params: &[u64]) -> N {
    let mut n = N::Atom(0);
    for &p in params {
        n = N::Cell(Box::new(N::Atom(p)), Box::new(n));
    }
    n
}

/// Compile `src` for nox, reduce against `params`, return the atom result.
fn run(src: &str, params: &[u64]) -> u64 {
    let formula_str = trident::compile_with_options(src, "surface.tri", &nox_options())
        .expect("compile for nox failed");
    let formula = parse(&formula_str);
    let subj = subject_noun(params);
    let mut ar = Reduction::<4096>::new();
    let f = load(&mut ar, &formula);
    let s = load(&mut ar, &subj);
    match reduce(&mut ar, s, f, 5_000_000, &NullCalls, &mut NoTrace) {
        Outcome::Ok(r, _) => ar.atom_value(r).expect("result not an atom").as_u64(),
        o => panic!("reduction failed: {:?}", o),
    }
}

/// Compile `src` for nox and reduce, expecting a reduction ERROR (crash).
fn run_expect_error(src: &str, params: &[u64]) {
    let formula_str = trident::compile_with_options(src, "surface.tri", &nox_options())
        .expect("compile for nox failed");
    let formula = parse(&formula_str);
    let subj = subject_noun(params);
    let mut ar = Reduction::<4096>::new();
    let f = load(&mut ar, &formula);
    let s = load(&mut ar, &subj);
    match reduce(&mut ar, s, f, 5_000_000, &NullCalls, &mut NoTrace) {
        Outcome::Ok(r, _) => {
            panic!(
                "expected crash, got {:?}",
                ar.atom_value(r).map(|g| g.as_u64())
            )
        }
        _ => {}
    }
}

// ── arithmetic + variables ───────────────────────────────────────

#[test]
fn arithmetic_and_params() {
    let src = "program test\npub fn f(a: Field, b: Field) -> Field { a * b + a }";
    assert_eq!(run(src, &[3, 4]), 15);
}

#[test]
fn let_bindings() {
    let src = "program test
pub fn f() -> Field {
    let x: Field = 6
    let y: Field = 7
    x * y
}";
    assert_eq!(run(src, &[]), 42);
}

// ── mutable assignment (subject edit) ────────────────────────────

#[test]
fn mutation_survives_branch() {
    let src = "program test
pub fn f(c: Field) -> Field {
    let mut x: Field = 1
    if c == 0 { x = 99 }
    x
}";
    assert_eq!(run(src, &[0]), 99);
    assert_eq!(run(src, &[5]), 1);
}

#[test]
fn early_return() {
    let src = "program test
pub fn f(x: Field) -> Field {
    if x == 0 { return 7 }
    x + 100
}";
    assert_eq!(run(src, &[0]), 7);
    assert_eq!(run(src, &[5]), 105);
}

// ── bounded for loops ────────────────────────────────────────────

#[test]
fn static_loop() {
    let src = "program test
pub fn f() -> Field {
    let mut s: Field = 0
    for i in 0..6 { s = s + 1 }
    s
}";
    assert_eq!(run(src, &[]), 6);
}

#[test]
fn dynamic_bounded_loop() {
    let src = "program test
pub fn f(n: Field) -> Field {
    let mut s: Field = 0
    for i in 0..n bounded 10 { s = s + 1 }
    s
}";
    assert_eq!(run(src, &[4]), 4);
    assert_eq!(run(src, &[10]), 10);
    assert_eq!(run(src, &[25]), 10);
}

// ── function inlining ────────────────────────────────────────────

#[test]
fn function_calls_inline() {
    let src = "program test
fn cube(x: Field) -> Field { x * x * x }
pub fn f() -> Field { cube(3) + cube(2) }";
    assert_eq!(run(src, &[]), 35);
}

// ── aggregates ───────────────────────────────────────────────────

#[test]
fn structs_and_fields() {
    let src = "program test
struct Point { x: Field, y: Field }
pub fn f() -> Field {
    let mut p = Point { x: 10, y: 20 }
    p.x = 5
    p.x + p.y
}";
    assert_eq!(run(src, &[]), 25);
}

#[test]
fn arrays_and_indexing() {
    let src = "program test
pub fn f() -> Field {
    let mut a: [Field; 4] = [1, 2, 3, 4]
    a[2] = 30
    a[0] + a[2] + a[3]
}";
    assert_eq!(run(src, &[]), 35);
}

#[test]
fn tuple_destructuring() {
    let src = "program test
pub fn f() -> Field {
    let (a, b, c): (Field, Field, Field) = (1, 2, 3)
    a + b * c
}";
    assert_eq!(run(src, &[]), 7);
}

// ── os.state.read → look (pattern 17) ────────────────────────────

struct StateProvider {
    l0: u64,
}

impl nox::LookProvider for StateProvider {
    fn look(&self, c: Goldilocks, ns: Goldilocks, key: Goldilocks) -> Option<Goldilocks> {
        if c == Goldilocks::new(self.l0) && ns == Goldilocks::new(0) {
            Some(Goldilocks::new(key.as_u64() * 10 + 7))
        } else {
            None
        }
    }
}

impl<const M: usize> nox::CallProvider<M> for StateProvider {
    fn provide(
        &self,
        _r: &mut Reduction<M>,
        _tag: Goldilocks,
        _object: nox::Order,
    ) -> Option<nox::Order> {
        None
    }
}

#[test]
fn os_state_read_end_to_end() {
    // Full pipeline: parse → typecheck (nox target) → lowering → printed
    // noun → reduce with a LookProvider. Subject = [root_tree [k 0]] per the
    // reads_state contract (reference/os.md, Graph row).
    let src = "program test\npub fn f(k: Field) -> Field { os.state.read(k) + 1 }";
    let formula_str = trident::compile_with_options(src, "surface.tri", &nox_options())
        .expect("compile for nox failed");
    assert!(
        formula_str.contains("[17 [[1 0]"),
        "look missing: {}",
        formula_str
    );
    let formula = parse(&formula_str);

    let root = [11u64, 22, 33, 44];
    let root_tree = N::Cell(
        Box::new(N::Atom(root[0])),
        Box::new(N::Cell(
            Box::new(N::Atom(root[1])),
            Box::new(N::Cell(
                Box::new(N::Atom(root[2])),
                Box::new(N::Atom(root[3])),
            )),
        )),
    );
    let subj = N::Cell(Box::new(root_tree), Box::new(subject_noun(&[4])));

    let mut ar = Reduction::<4096>::new();
    let f = load(&mut ar, &formula);
    let s = load(&mut ar, &subj);
    let provider = StateProvider { l0: root[0] };
    match reduce(&mut ar, s, f, 5_000_000, &provider, &mut NoTrace) {
        // key 4 → 47, +1 = 48
        Outcome::Ok(r, _) => assert_eq!(ar.atom_value(r).unwrap().as_u64(), 48),
        o => panic!("reduction failed: {:?}", o),
    }
}

#[test]
fn os_state_read_sets_bundle_flag() {
    // compile_to_bundle must declare the state dependency; a stateless
    // program must not.
    let dir = std::env::temp_dir().join("trident_os_state_bundle_test");
    std::fs::create_dir_all(&dir).unwrap();
    let stateful = dir.join("stateful.tri");
    std::fs::write(
        &stateful,
        "program stateful\npub fn f(k: Field) -> Field { os.state.read(k) }",
    )
    .unwrap();
    let stateless = dir.join("stateless.tri");
    std::fs::write(
        &stateless,
        "program stateless\npub fn f(k: Field) -> Field { k + 1 }",
    )
    .unwrap();

    let b1 = trident::compile_to_bundle(&stateful, &nox_options()).expect("bundle failed");
    assert!(
        b1.reads_state,
        "state-reading program must declare reads_state"
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&b1.to_json()).unwrap()["reads_state"],
        true
    );

    let b2 = trident::compile_to_bundle(&stateless, &nox_options()).expect("bundle failed");
    assert!(!b2.reads_state);
    assert!(
        !b2.to_json().contains("reads_state"),
        "stateless bundle JSON must omit the field (backward compat)"
    );
}

// ── builtins ─────────────────────────────────────────────────────

#[test]
fn sub_builtin() {
    let src = "program test\npub fn f() -> Field { sub(100, 58) }";
    assert_eq!(run(src, &[]), 42);
}

#[test]
fn assert_true_then_value() {
    let src = "program test
pub fn f() -> Field {
    assert(2 == 2)
    9
}";
    assert_eq!(run(src, &[]), 9);
}

#[test]
fn assert_false_crashes() {
    let src = "program test
pub fn f() -> Field {
    assert(1 == 2)
    9
}";
    run_expect_error(src, &[]);
}

/// reference/language.md: `Digest` is `[Field; D]`; on nox D = 4 and the
/// limbs live at axes 4..7 of the hash pair. Limb access must reduce.
#[test]
fn digest_limbs_index_on_nox() {
    let limbs: Vec<u64> = (0..4)
        .map(|k| {
            run(
                &format!(
                    "program limb\nfn main(a: Field) -> Field {{\n    let d: Digest = hash(a, 0, 0, 0, 0, 0, 0, 0)\n    d[{k}]\n}}\n"
                ),
                &[42],
            )
        })
        .collect();
    assert!(limbs.iter().all(|&l| l != 0), "limbs {limbs:?}");
    assert_eq!(
        limbs
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4,
        "limbs distinct {limbs:?}"
    );
    // the same hash, summed limbs in-program == summed limbs out of program
    let sum = run(
        "program sum\nfn main(a: Field) -> Field {\n    let d: Digest = hash(a, 0, 0, 0, 0, 0, 0, 0)\n    d[0] + d[1] + d[2] + d[3]\n}\n",
        &[42],
    );
    let p: u128 = 0xFFFF_FFFF_0000_0001;
    assert_eq!(
        sum as u128,
        limbs.iter().map(|&l| l as u128).sum::<u128>() % p
    );
}

/// A depth-32 Merkle path chained through digest limbs — the operation the
/// README compares across VMs — compiles and reduces on nox.
#[test]
fn merkle_path_depth_32_reduces_on_nox() {
    let src = "program merkle32\n\nfn main(leaf: Field, s0: Field) -> Field {\n    let mut acc: Digest = hash(leaf, 0, 0, 0, 0, 0, 0, 0)\n    for i in 0..32 bounded 32 {\n        acc = hash(acc[0], acc[1], acc[2], acc[3], s0, 0, 0, 0)\n    }\n    acc[0]\n}\n";
    let root = run(src, &[7, 9]);
    assert_ne!(root, 0);
}

/// Execute the emitted formula and return its value and actual reduction bill.
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
fn nox_transitive_imported_state_read_fails_closed_in_all_artifacts() {
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
    for errors in [
        trident::compile_project_with_options(&entry, &release).unwrap_err(),
        trident::nox_cost_project(&entry, &release).unwrap_err(),
        trident::compile_to_bundle(&entry, &release).unwrap_err(),
    ] {
        assert!(errors
            .iter()
            .any(|e| e.message.contains("os.state.read inside a called function")));
    }
}

#[test]
fn foreign_tree_and_unknown_targets_never_fall_back_to_nox() {
    let source = "program terrain\nfn main(a: Field) -> Field { a + 2 }\n";
    let dir = tempfile::tempdir().unwrap();
    let entry = dir.path().join("main.tri");
    std::fs::write(&entry, source).unwrap();
    let nock = TerrainConfig::resolve("nock").unwrap();
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
