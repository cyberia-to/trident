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
    run_profile(src, params, "debug")
}

fn run_profile(src: &str, params: &[u64], profile: &str) -> u64 {
    let mut options = nox_options();
    options.profile = profile.into();
    options.cfg_flags = std::collections::BTreeSet::from([profile.into()]);
    let formula_str = trident::compile_with_options(src, "surface.tri", &options)
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
#[path = "nox_surface/project_contracts.rs"]
mod project_contracts;

#[test]
fn event_statements_fail_closed_until_nox_has_an_event_wire_contract() {
    for kind in ["reveal", "seal"] {
        let source = format!("program events\nevent E{{a:Field}}\nfn main(){{{kind} E{{a:1}}}}");
        let error =
            trident::compile_with_options(&source, "events.tri", &nox_options()).unwrap_err();
        assert!(
            error
                .iter()
                .any(|d| d.message.contains("reveal/seal not yet supported")),
            "{error:?}"
        );
    }
}

#[test]
fn bounded_loop_return_exits_function_and_preserves_fallthrough_scope() {
    let src = "program loop_return
fn main(n: Field) -> Field {
    let mut total: Field = 10
    for i in 0..n bounded 4 {
        let step = as_field(i) + 1
        if as_field(i) == 2 { return total }
        total = total + step
    }
    let after = total + 100
    after
}";
    for profile in ["debug", "release"] {
        for (input, expected) in [(0, 110), (2, 113), (4, 13), (20, 13)] {
            assert_eq!(run_profile(src, &[input], profile), expected);
        }
    }
}

#[test]
fn loop_returns_nested_aggregate_helper_without_executing_later_effects() {
    let src = "program aggregate_return
struct Pair { a: Field, b: Field }
fn search(n: Field) -> Pair {
    let mut total: Field = 3
    for i in 0..3 {
        let shadow = as_field(i)
        for j in 0..2 {
            if as_field(i) == n {
                let shadow = total + as_field(j)
                return Pair { a: shadow, b: 7 }
            }
            total = total + 1
        }
    }
    Pair { a: total, b: 9 }
}
fn main(n: Field) -> Field {
    let result = search(n)
    result.a * 10 + result.b
}";
    let effects = "program suppressed
fn helper() {
    for i in 0..3 {
        if as_field(i) == 1 { return }
    }
    let secret: Field = divine()
    assert(secret == 9)
}
fn main() -> Field { helper()\n 42 }";
    for profile in ["debug", "release"] {
        for (n, expected) in [(0, 37), (1, 57), (2, 77), (9, 99)] {
            assert_eq!(run_profile(src, &[n], profile), expected);
        }
        assert_eq!(run_profile(effects, &[], profile), 42);
    }
}

#[test]
fn loop_return_drops_iteration_shadowing_and_skips_unreachable_state_lookup() {
    let src = "program state_return
fn main(n: Field) -> Field {
    for i in 0..2 {
        if as_field(i) == 0 { return n }
        let ignored = os.state.read(11)
    }
    os.state.read(12)
}";
    for profile in ["debug", "release"] {
        let options = CompileOptions::for_profile(profile);
        let text = trident::compile_with_options(src, "state.tri", &options).unwrap();
        let mut arena = Reduction::<4096>::new();
        let formula = load(&mut arena, &parse(&text));
        let subject = load(&mut arena, &parse("[[1 [2 [3 4]]] [23 0]]"));
        // NullCalls refuses all lookups: successful reduction proves neither
        // the later iteration nor the post-loop state access was evaluated.
        match reduce(
            &mut arena,
            subject,
            formula,
            1_000_000,
            &NullCalls,
            &mut NoTrace,
        ) {
            Outcome::Ok(value, _) => assert_eq!(arena.atom_value(value).unwrap().as_u64(), 23),
            other => panic!("unreachable lookup executed: {other:?}"),
        }
    }
    let fallthrough = "program shadow
fn main() -> Field {
    let x: Field = 11
    for i in 0..2 {
        let x: Field = 99
        if x == 0 { return 1 }
    }
    let y = x + 2
    y
}";
    for profile in ["debug", "release"] {
        assert_eq!(run_profile(fallthrough, &[], profile), 13);
    }
}

#[test]
fn loop_without_return_preserves_scope_before_following_bindings() {
    let src = "program later_binding
fn main() -> Field {
    let mut total: Field = 2
    for i in 0..3 { total = total + as_field(i) }
    let after = total + 10
    after
}";
    for profile in ["debug", "release"] {
        assert_eq!(run_profile(src, &[], profile), 15);
    }
}

#[test]
fn zero_bound_and_exclusive_end_do_not_enter_returning_body() {
    for profile in ["debug", "release"] {
        for range in ["3..3", "4..2", "0..n bounded 0"] {
            let source = format!("program empty\nfn main(n: Field) -> Field {{ for i in {range} {{ return 99 }}\n 7 }}");
            assert_eq!(run_profile(&source, &[9], profile), 7);
        }
        let source = "program exclusive\nfn main() -> Field { for i in 0..2 { if as_field(i) == 2 { return 99 } }\n 7 }";
        assert_eq!(run_profile(source, &[], profile), 7);
    }
}

#[test]
fn returning_loop_seals_shadowed_aggregate_type_with_its_name() {
    let source = "program typed_shadow
struct Pair { a: Field, b: Field }
struct Reverse { b: Field, a: Field }
fn main(n: Field) -> Field {
    let x = Pair { a: 11, b: 22 }
    for i in 0..2 {
        if n == as_field(i) { return x.a } else {
            let x = Reverse { b: 99, a: 88 }
        }
        if x.a == 22 { return 999 }
    }
    x.a
}";
    for profile in ["debug", "release"] {
        for n in [0, 1, 9] {
            assert_eq!(run_profile(source, &[n], profile), 11);
        }
    }
}

#[path = "nox_surface/entry.rs"]
mod entry;

#[path = "nox_surface/imported_entry.rs"]
mod imported_entry;

#[path = "nox_surface/terminal.rs"]
mod terminal;

#[path = "nox_surface/loop_indices.rs"]
mod loop_indices;
