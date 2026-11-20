//! Regression anchors for the x86-64 backend (SIL-1 and review round 3).

use super::*;
use crate::anchor_support::*;

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
