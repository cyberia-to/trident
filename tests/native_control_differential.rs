//! The same source functions agree with legacy nox and independent integers.
#[path = "native_control/support.rs"]
mod support;
use nebu::Goldilocks;
use nox::{sequential, NoTrace, Order, Outcome, Reduction};
use support::*;
use trident::ir::tree::lower::{nox::NoxCompiler, Noun};
use trident::RAW_ARTIFACT_LIMITS as LIMITS;

fn load(arena: &mut Reduction<ARENA>, noun: &Noun) -> Order {
    match noun {
        Noun::Atom(n) => arena.atom(Goldilocks::new(*n)).unwrap(),
        Noun::Cell(a, b) => {
            let a = load(arena, a);
            let b = load(arena, b);
            arena.pair(a, b).unwrap()
        }
    }
}
fn legacy(source: &str, input: u64) -> u64 {
    trident::check_silent(source, "flat.tri").unwrap();
    let file = trident::parse_source_silent(source, "flat.tri").unwrap();
    let formula = NoxCompiler::new().compile_file(&file).unwrap();
    let mut arena = Reduction::<ARENA>::new();
    let formula = load(&mut arena, &formula);
    let input = arena.atom(Goldilocks::new(input)).unwrap();
    let zero = arena.atom(Goldilocks::ZERO).unwrap();
    let input = arena.pair(input, zero).unwrap();
    let result = sequential::reduce(
        &mut arena,
        input,
        formula,
        10_000_000,
        sequential::Limits { max_frames: 65536 },
        &mut NoTrace,
    )
    .unwrap();
    let Outcome::Ok(output, _) = result.outcome else {
        panic!("{:?}", result.outcome);
    };
    arena.atom_value(output).unwrap().as_u64()
}

#[test]
fn reusable_bodies_match_legacy_execution_and_integer_ground_truth() {
    worker(|| {
        type Case = (&'static str, fn(u64) -> u64);
        let cases: [Case;5] = [
            ("let mut total: Field = 0\nfor i in 0..n bounded 8 { total = total + as_field(i) }\ntotal", |n| n.min(8) * n.min(8).saturating_sub(1) / 2),
            ("let mut a = [1,2,3,4]\nfor i in 0..4 { a[i] = a[i] + n * as_field(i) }\na[0] + a[1]*10 + a[2]*100 + a[3]*1000", |n| 4321+3210*n),
            ("let mut end: Field = n\nlet mut count: Field = 0\nfor i in 0..end bounded 8 { count = count+1\nif as_field(i) == 1 { end = 4 } }\ncount", |n| if n < 2 { n } else { 4 }),
            ("for i in 0..4 { for j in 0..3 { if as_field(i) == n { return as_field(i)*10+as_field(j) } } }\n999", |n| if n < 4 { n*10 } else {999}),
            ("let mut total: Field = n\nfor i in 0..3 { for j in i..3 { total = total*10+as_field(j)+1 } }\ntotal", |n| n*1_000_000+123233),
        ];
        for (body, expected) in cases {
            let functions = format!("program p\nfn compute(n: Field) -> Field {{ {body} }}\n");
            let raw = compile(&format!("{functions}fn main(input: Noun) -> Noun {{ nox_noun_atom(compute(nox_noun_as_field(input))) }}"));
            let flat = format!("{functions}fn main(input: Field) -> Field {{ compute(input) }}");
            for input in 0..9 {
                let native = run(&raw.bytes, input, 10_000_000, 65536, LIMITS.max_nodes)
                    .unwrap()
                    .0;
                assert_eq!(native, expected(input), "native: {body}, input {input}");
                assert_eq!(
                    legacy(&flat, input),
                    expected(input),
                    "legacy: {body}, input {input}"
                );
            }
        }
    });
}
