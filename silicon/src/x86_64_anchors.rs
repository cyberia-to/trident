//! Regression anchors for the x86-64 backend (SIL-1 and review round 3).

use super::*;
use nebu::Goldilocks;

fn atom_of(o: &mut Order<4096>, v: u64) -> NounId { o.atom(Goldilocks::new(v)).unwrap() }
fn axis_of(o: &mut Order<4096>, ax: u64) -> NounId {
    let t = atom_of(o, 0);
    let a = atom_of(o, ax);
    o.pair(t, a).unwrap()
}
fn quote_of(o: &mut Order<4096>, v: u64) -> NounId {
    let t = atom_of(o, 1);
    let a = atom_of(o, v);
    o.pair(t, a).unwrap()
}
fn binary_of(o: &mut Order<4096>, tag: u64, l: NounId, r: NounId) -> NounId {
    let body = o.pair(l, r).unwrap();
    let t = atom_of(o, tag);
    o.pair(t, body).unwrap()
}
fn cons_of(o: &mut Order<4096>, head: NounId, tail: NounId) -> NounId {
    let body = o.pair(head, tail).unwrap();
    let t = atom_of(o, 3);
    o.pair(t, body).unwrap()
}
/// Right-nested sum with `live` leaves alive at once: each `a` stays in its
/// register while `b` is emitted (live=1 is a leaf; live=n is n-1 sums).
fn live_sum(o: &mut Order<4096>, live: u32) -> NounId {
    let mut f = axis_of(o, 2);
    for _ in 1..live {
        let leaf = axis_of(o, 2);
        f = binary_of(o, 5, leaf, f);
    }
    f
}
fn is_scratch_overflow<T>(r: &Result<T, CompileError>) -> bool {
    matches!(r, Err(CompileError::ScratchOverflow))
}
/// Loop formula `[2 [[3 [[1 0] inits]] [1 body]]]` with `carried` init values and
/// a body that adds two live values: `add(axis 2, axis 2)`.
fn loop_with_sum_body(o: &mut Order<4096>, carried: u64) -> NounId {
    let mut chain = axis_of(o, 1);
    for i in 0..carried {
        let init = quote_of(o, i + 1);
        chain = cons_of(o, init, chain);
    }
    let slot = quote_of(o, 0);
    let setup = cons_of(o, slot, chain);
    let (l, r) = (axis_of(o, 2), axis_of(o, 2));
    let body = binary_of(o, 5, l, r);
    let t1 = atom_of(o, 1);
    let quoted_body = o.pair(t1, body).unwrap();
    binary_of(o, 2, setup, quoted_body)
}

#[test]
fn sum_with_capacity_plus_one_live_returns_scratch_overflow() {
    assert_eq!(SCRATCH.len(), 6);
    let mut o = Order::<4096>::new();
    let f = live_sum(&mut o, SCRATCH.len() as u32 + 1);
    let r = compile_to_x86_64(&o, f, 1);
    assert!(is_scratch_overflow(&r), "SIL-1.A: 7 live values with 6 scratch must give ScratchOverflow, got {:?}", r.as_ref().map(|c| c.len()));
}

#[test]
fn sum_up_to_capacity_live_still_compiles() {
    let cap = SCRATCH.len() as u32;
    for live in [cap - 1, cap] {
        let mut o = Order::<4096>::new();
        let f = live_sum(&mut o, live);
        assert!(compile_to_x86_64(&o, f, 1).is_ok(), "{live} live values fit in {cap} scratch");
    }
}

/// SIL-1, group B: sub underflow branch, `mov dst, P` overwrote ra (== dst) before `add dst, ra`.
/// The constant P must be materialized in a temporary that is not a scratch register (rax).
#[test]
fn sub_underflow_does_not_materialize_p_in_a_scratch() {
    let mut o = Order::<4096>::new();
    let a0 = axis_of(&mut o, 2);
    let a1 = axis_of(&mut o, 6);
    let f = binary_of(&mut o, 6, a0, a1);
    let code = compile_to_x86_64(&o, f, 2).unwrap();
    let p_le = P.to_le_bytes();
    let i = code.windows(8).position(|w| w == p_le).expect("MOV reg, imm64 P in the underflow branch");
    let (rex, op) = (code[i - 2], code[i - 1]);
    assert_eq!(rex & 0xF8, 0x48, "REX.W");
    let reg = ((rex & 1) << 3) | (op - 0xB8);
    assert!(!SCRATCH.contains(&reg), "SIL-1.B: P is loaded into r{reg} (scratch, aliases ra) before the ADD with ra");
}

/// Review R2: `emit_loop` took registers straight from `next_scratch` without
/// touching `reg_stack`, so the exhaustion guard never saw loop registers.
#[test]
fn loop_whose_body_exceeds_scratch_capacity_reports_overflow() {
    let mut o = Order::<4096>::new();
    let f = loop_with_sum_body(&mut o, 4);
    let r = compile_to_x86_64(&o, f, 0);
    assert!(is_scratch_overflow(&r), "R2: 4 carried locals plus a two-live sum body exceed the scratch pool, got {:?}", r.as_ref().map(|c| c.len()));
}

/// Review R2, non-spurious guard: a shallow loop must keep compiling.
#[test]
fn shallow_loop_still_compiles() {
    for carried in [1u64, 3] {
        let mut o = Order::<4096>::new();
        let f = loop_with_sum_body(&mut o, carried);
        let r = compile_to_x86_64(&o, f, 0);
        assert!(r.is_ok(), "R2: loop with {carried} carried locals fits in the scratch pool, got {:?}", r.as_ref().map(|c| c.len()));
    }
}
