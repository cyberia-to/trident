// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Differential harness: real stdlib modules executed on the real nox VM,
//! compared against independently computed Rust ground truth.
//!
//! Anchors (the release plan's correctness anchor, soft3-release M3):
//! - every stdlib module inside the nox surface is executed here end-to-end
//!   (parse → typecheck → nox lowering → `nox::reduce`) and its outputs must
//!   equal ground truth computed by a separate Rust implementation ported
//!   from `benches/references/`;
//! - the census pin fails when the nox surface grows, demanding differential
//!   coverage for every newly compiling module — coverage cannot silently lag;
//! - triton×nox output agreement is pending the trisha repair
//!   (cyberia-to/trisha#1): the installed trisha predates the warrior CLI and
//!   the workspace build is broken. When trisha revives, the same wrapped
//!   sources run there.
//!
//! The noun parse/load/reduce helper mirrors tests/nox_surface.rs.

use nebu::Goldilocks;
use nox::{reduce, NoTrace, NullCalls, Outcome, Reduction};
use trident::target::TerrainConfig;
use trident::CompileOptions;

// ── noun harness (mirrors tests/nox_surface.rs) ─────────────────────────────

enum N {
    Atom(u64),
    Cell(Box<N>, Box<N>),
}

fn nox_options() -> CompileOptions {
    let mut o = CompileOptions::default();
    o.target_config = TerrainConfig::resolve("nox").expect("nox target.toml must resolve");
    o
}

fn parse(src: &str) -> N {
    let bytes = src.as_bytes();
    let mut pos = 0;
    let n = parse_at(bytes, &mut pos);
    assert_eq!(pos, bytes.len(), "trailing input: {:?}", &src[pos..]);
    n
}

fn parse_at(b: &[u8], pos: &mut usize) -> N {
    if b[*pos] == b'[' {
        *pos += 1;
        let left = parse_at(b, pos);
        *pos += 1; // ' '
        let right = parse_at(b, pos);
        assert_eq!(b[*pos], b']', "expected closing bracket");
        *pos += 1;
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

fn subject_noun(params: &[u64]) -> N {
    let mut n = N::Atom(0);
    for &p in params {
        n = N::Cell(Box::new(N::Atom(p)), Box::new(n));
    }
    n
}

fn run(src: &str, params: &[u64]) -> u64 {
    let formula_str = trident::compile_with_options(src, "differential.tri", &nox_options())
        .expect("compile for nox failed");
    let formula = parse(&formula_str);
    let subj = subject_noun(params);
    // Unrolled stdlib formulas outgrow small arenas; mirror joy's warrior:
    // a 2^18-slot arena on a dedicated 256 MiB stack (rs/warrior.rs).
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(move || {
            let mut ar = Reduction::<{ 1 << 18 }>::new();
            let f = load(&mut ar, &formula);
            let s = load(&mut ar, &subj);
            match reduce(&mut ar, s, f, 50_000_000, &NullCalls, &mut NoTrace) {
                Outcome::Ok(r, _) => ar.atom_value(r).expect("result not an atom").as_u64(),
                other => panic!("reduce failed: {other:?}"),
            }
        })
        .expect("spawn reduce thread")
        .join()
        .expect("reduce thread panicked")
}

// ── wrapping a stdlib module into an executable program ─────────────────────

/// Read a `module std.…` file and wrap it as a `program` with the given main.
fn wrap_module(rel_path: &str, main_src: &str) -> String {
    let root = env!("CARGO_MANIFEST_DIR");
    let src = std::fs::read_to_string(format!("{root}/{rel_path}"))
        .unwrap_or_else(|e| panic!("read {rel_path}: {e}"));
    let body: String = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("module "))
        .collect::<Vec<_>>()
        .join("\n");
    format!("program differential\n{body}\n{main_src}\n")
}

// ── ground truth, ported from benches/references (independent of the .tri) ──

const P: u128 = 0xFFFF_FFFF_0000_0001;

fn fadd(a: u64, b: u64) -> u64 {
    ((a as u128 + b as u128) % P) as u64
}
fn fmul(a: u64, b: u64) -> u64 {
    ((a as u128 * b as u128) % P) as u64
}

fn fib_ref(n: u64) -> u64 {
    let (mut a, mut b) = (0u64, 1u64);
    for _ in 0..n {
        let next = fadd(a, b);
        a = b;
        b = next;
    }
    a
}

fn sbox(x: u64) -> u64 {
    let x2 = fmul(x, x);
    let x4 = fmul(x2, x2);
    fmul(x4, x)
}
fn mix2(a: u64, b: u64) -> (u64, u64) {
    (fadd(fadd(a, a), b), fadd(fadd(fadd(a, b), b), b))
}
fn round2(a: u64, b: u64, rc0: u64, rc1: u64) -> (u64, u64) {
    mix2(sbox(fadd(a, rc0)), sbox(fadd(b, rc1)))
}
fn poseidon_hash2_ref(a: u64, b: u64) -> u64 {
    let mut s = (a, b);
    for &(r0, r1) in &[(3, 7), (11, 13), (17, 19), (23, 29)] {
        s = round2(s.0, s.1, r0, r1);
    }
    s.0
}

// ── the differentials ───────────────────────────────────────────────────────

#[test]
fn fibonacci_module_matches_rust_ground_truth() {
    let src = wrap_module(
        "std/math/fibonacci.tri",
        "fn main(n: Field) -> Field { fib256(n) }",
    );
    for n in [0u64, 1, 2, 3, 10, 55, 100, 255] {
        assert_eq!(run(&src, &[n]), fib_ref(n), "fib256({n}) diverged");
    }
}

#[test]
fn poseidon_module_matches_rust_ground_truth() {
    let src = wrap_module(
        "std/crypto/poseidon.tri",
        "fn main(a: Field, b: Field) -> Field { hash2(a, b) }",
    );
    for (a, b) in [(42u64, 1337u64), (0, 0), (1, 0), (P as u64 - 2, 7)] {
        assert_eq!(
            run(&src, &[a, b]),
            poseidon_hash2_ref(a, b),
            "hash2({a},{b}) diverged"
        );
    }
}

// ── the census pin ──────────────────────────────────────────────────────────

/// Every stdlib module whose nox cost analysis succeeds is inside the nox
/// surface and MUST have a differential above. When the surface grows this
/// pin fails: add the module's differential, then extend the expected list.
#[test]
fn census_every_in_surface_module_has_a_differential() {
    let root = env!("CARGO_MANIFEST_DIR");
    let options = nox_options();
    let mut in_surface: Vec<String> = Vec::new();
    let mut stack = vec![format!("{root}/std")];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path.display().to_string());
            } else if path.extension().is_some_and(|e| e == "tri")
                && trident::nox_cost_project(&path, &options).is_ok()
            {
                in_surface.push(
                    path.display()
                        .to_string()
                        .trim_start_matches(&format!("{root}/"))
                        .to_string(),
                );
            }
        }
    }
    in_surface.sort();
    assert_eq!(
        in_surface,
        vec![
            "std/crypto/poseidon.tri".to_string(),
            "std/math/fibonacci.tri".to_string(),
        ],
        "the nox surface changed — update the differential harness to cover \
         every newly compiling module, then extend this pin"
    );
}
