// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Differential harness: real stdlib modules executed on the real nox VM,
//! compared against independently computed Rust ground truth.
//!
//! The census measures the first active public function's reachable call tree
//! in each library. It does not certify an entire module. Each newly compiling
//! entry has an executed representative below, named by the actual functions
//! checked. Compiler-module constants do not establish self-hosting; crypto
//! initialization/prime fixtures do not establish full cryptographic algorithms.
//! Fibonacci and standard Poseidon2-HL use independent integer references.

use nebu::Goldilocks;
use nox::{reduce, CallProvider, LookProvider, NoTrace, Outcome, Reduction};
use std::sync::atomic::{AtomicUsize, Ordering};
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

fn subject_noun(params: &[u64]) -> N {
    let mut n = N::Atom(0);
    for &p in params {
        n = N::Cell(Box::new(N::Atom(p)), Box::new(n));
    }
    n
}

struct SecretInputs {
    values: Vec<u64>,
    next: AtomicUsize,
}

impl LookProvider for SecretInputs {
    fn look(&self, _: Goldilocks, _: Goldilocks, _: Goldilocks) -> Option<Goldilocks> {
        None
    }
}

impl<const M: usize> CallProvider<M> for SecretInputs {
    fn provide(&self, ar: &mut Reduction<M>, _: Goldilocks, _: nox::Order) -> Option<nox::Order> {
        let i = self.next.fetch_add(1, Ordering::SeqCst);
        ar.atom(Goldilocks::new(*self.values.get(i)?))
    }
}

fn leaves<const M: usize>(ar: &Reduction<M>, value: nox::Order, out: &mut Vec<u64>) {
    if let Some(atom) = ar.atom_value(value) {
        out.push(atom.as_u64());
    } else {
        leaves(ar, ar.head(value).expect("pair head"), out);
        leaves(ar, ar.tail(value).expect("pair tail"), out);
    }
}

fn execute(assembly: &str, params: &[u64], secrets: &[u64]) -> Result<Vec<u64>, String> {
    let formula = parse(assembly);
    let subj = subject_noun(params);
    let inputs = SecretInputs {
        values: secrets.to_vec(),
        next: AtomicUsize::new(0),
    };
    // Match Joy's arena and worker stack; large unrolled formulas need both.
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(move || {
            let mut ar = Reduction::<{ 1 << 18 }>::new();
            let f = load(&mut ar, &formula);
            let s = load(&mut ar, &subj);
            match reduce(&mut ar, s, f, 50_000_000, &inputs, &mut NoTrace) {
                Outcome::Ok(r, _) => {
                    let mut out = Vec::new();
                    leaves(&ar, r, &mut out);
                    Ok(out)
                }
                other => Err(format!("reduce failed: {other:?}")),
            }
        })
        .expect("spawn reduce thread")
        .join()
        .expect("reduce thread panicked")
}

fn run(src: &str, params: &[u64]) -> u64 {
    let assembly = trident::compile_with_options(src, "differential.tri", &nox_options())
        .expect("compile for nox failed");
    let output = execute(&assembly, params, &[]).expect("execution failed");
    assert_eq!(output.len(), 1, "expected scalar output");
    output[0]
}

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/differential")
        .join(format!("{name}.tri"));
    trident::compile_project_with_options(&path, &nox_options())
        .unwrap_or_else(|errors| panic!("fixture {name} failed to compile: {errors:?}"))
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

#[path = "../benches/references/common/poseidon_standard.rs"]
mod standard_poseidon;
fn poseidon_hash2_ref(a: u64, b: u64) -> u64 {
    standard_poseidon::hash2(a, b)
}

// ── the differentials ───────────────────────────────────────────────────────

#[test]
fn fibonacci_module_matches_rust_ground_truth() {
    let src = wrap_module(
        "lib/std/math/fibonacci.tri",
        "fn main(n: Field) -> Field { fib256(n) }",
    );
    for n in [0u64, 1, 2, 3, 10, 55, 100, 255] {
        assert_eq!(run(&src, &[n]), fib_ref(n), "fib256({n}) diverged");
    }
}

#[test]
fn poseidon_module_matches_rust_ground_truth() {
    let src = wrap_module(
        "lib/std/crypto/poseidon.tri",
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

// Selected first-entry call trees, not whole-module certification.
#[test]
fn compiler_token_node_opcode_and_type_constants_match_their_public_ids() {
    for (name, expected) in [
        ("lexer_tokens", 5601),
        ("lower_opcodes", 301),
        ("parser_node_kinds", 301),
        ("checker_type_kinds", 901),
    ] {
        assert_eq!(
            execute(&fixture(name), &[], &[]).unwrap(),
            vec![expected],
            "{name}"
        );
    }
}

#[test]
fn bigint_zero_keccak_zero_lane_and_sponge_zero_states_are_all_zero() {
    for (name, fields) in [
        ("bigint_zero", 8),
        ("keccak_zero_lane", 2),
        ("lut_sponge_zero", 8),
        ("poseidon2_zero", 8),
    ] {
        // Structs use a right cons-list ending in the unit atom 0.
        assert_eq!(
            execute(&fixture(name), &[], &[]).unwrap(),
            vec![0; fields + 1],
            "{name}"
        );
    }
}

#[test]
fn ed25519_and_secp256k1_prime_getters_match_integer_definitions() {
    // p25519 = 2^255 - 19; secp256k1 p = 2^256 - 2^32 - 977.
    // Construct little-endian radix-2^32 limbs independently of .tri literals.
    let mut ed = vec![u32::MAX as u64; 8];
    ed[0] -= 18;
    ed[7] >>= 1;
    let mut secp = vec![u32::MAX as u64; 8];
    secp[0] -= 976;
    secp[1] -= 1;
    for (name, mut expected) in [("ed25519_prime", ed), ("secp256k1_prime", secp)] {
        expected.push(0); // aggregate terminator
        assert_eq!(
            execute(&fixture(name), &[], &[]).unwrap(),
            expected,
            "{name}"
        );
    }
}

#[test]
fn sha256_init_returns_the_standard_eight_word_iv() {
    // SHA-256 initial hash words; this fixture does not exercise compression.
    let expected = vec![
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19, 0,
    ];
    assert_eq!(
        execute(&fixture("sha256_init"), &[], &[]).unwrap(),
        expected
    );
}

#[test]
fn quantum_complex_zero_add_and_mul_match_field_complex_arithmetic() {
    let assembly = fixture("quantum_complex_mul");
    for (a, b, c, d) in [(0, 0, 1, 2), (3, 4, 5, 6), (P as u64 - 1, 2, 7, 8)] {
        let re = ((fmul(a, c) as u128 + P - fmul(b, d) as u128) % P) as u64;
        let im = fadd(fmul(a, d), fmul(b, c));
        assert_eq!(
            execute(&assembly, &[a, b, c, d], &[]).unwrap(),
            vec![re, im, 0]
        );
    }
}

#[test]
fn auth_verify_preimage_accepts_matching_secret_and_rejects_wrong_secret() {
    // Checks preimage equality via the target's hash intrinsic, not Tip5 vectors.
    let assembly = fixture("auth_preimage");
    for secret in [0, 42, P as u64 - 1] {
        assert_eq!(execute(&assembly, &[secret], &[secret]).unwrap(), vec![1]);
    }
    assert!(execute(&assembly, &[42], &[43]).is_err());
    assert!(execute(&assembly, &[42], &[]).is_err());
}

// ── the census pin ──────────────────────────────────────────────────────────

#[test]
fn native_compiler_scalar_helpers_match_ascii_and_bounded_integer_arithmetic() {
    let digit = fixture("native_compiler_digit");
    for value in (0..=255).chain([u64::from(u32::MAX)]) {
        assert_eq!(
            execute(&digit, &[value], &[]).unwrap(),
            vec![u64::from((48..=57).contains(&value))]
        );
    }
    let increment = fixture("native_compiler_increment");
    for value in [0, 1, 255, 4094, 4095] {
        assert_eq!(execute(&increment, &[value], &[]).unwrap(), vec![value + 1]);
    }
}

/// Cost analysis of a library selects its first active public function.
/// Every successful entry needs the scoped executed fixture above. This pin
/// does not claim all functions in those modules lower or execute correctly.
#[test]
fn census_every_in_surface_module_has_a_differential() {
    let root = env!("CARGO_MANIFEST_DIR");
    let options = nox_options();
    let mut in_surface: Vec<String> = Vec::new();
    let mut stack = vec![format!("{root}/lib/std")];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path.display().to_string());
            } else if path.extension().is_some_and(|e| e == "tri")
                && trident::nox_cost_project(&path, &options).is_ok()
            {
                in_surface.push(
                    path.strip_prefix(root)
                        .unwrap()
                        .components()
                        .map(|part| part.as_os_str().to_str().unwrap())
                        .collect::<Vec<_>>()
                        .join("/"),
                );
            }
        }
    }
    in_surface.sort();
    assert_eq!(
        in_surface,
        vec![
            "lib/std/compiler/lexer.tri".to_string(),
            "lib/std/compiler/lower.tri".to_string(),
            "lib/std/compiler/nox/ascii.tri".to_string(),
            "lib/std/compiler/nox/syntax.tri".to_string(),
            "lib/std/compiler/parser.tri".to_string(),
            "lib/std/compiler/typecheck.tri".to_string(),
            "lib/std/crypto/bigint.tri".to_string(),
            "lib/std/crypto/ed25519.tri".to_string(),
            "lib/std/crypto/keccak256.tri".to_string(),
            "lib/std/crypto/lut_sponge.tri".to_string(),
            "lib/std/crypto/poseidon.tri".to_string(),
            "lib/std/crypto/poseidon2.tri".to_string(),
            "lib/std/crypto/preimage.tri".to_string(),
            "lib/std/crypto/secp256k1.tri".to_string(),
            "lib/std/crypto/sha256.tri".to_string(),
            "lib/std/math/fibonacci.tri".to_string(),
            "lib/std/quantum/gates.tri".to_string(),
        ],
        "the nox surface changed — update the differential harness to cover \
         every newly compiling module, then extend this pin"
    );
}
