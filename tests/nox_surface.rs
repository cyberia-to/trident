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
        let v: u64 = std::str::from_utf8(&b[start..*pos]).unwrap().parse().unwrap();
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
            panic!("expected crash, got {:?}", ar.atom_value(r).map(|g| g.as_u64()))
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
    let p = Point { x: 10, y: 20 }
    p.x + p.y
}";
    assert_eq!(run(src, &[]), 30);
}

#[test]
fn arrays_and_indexing() {
    let src = "program test
pub fn f() -> Field {
    let a: [Field; 4] = [10, 20, 30, 40]
    a[0] + a[2] + a[3]
}";
    assert_eq!(run(src, &[]), 80);
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
