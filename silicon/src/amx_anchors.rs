//! Regression anchors for the AMX backend (SIL-1 and review round 3).
//! The pre-existing `mod tests` in amx.rs is separate and untouched.

use super::*;
use crate::anchor_support::*;

#[test]
fn sum_with_capacity_plus_one_live_returns_scratch_overflow() {
    assert_eq!(SCRATCH_REGS.len(), 7);
    let mut o = Order::<4096>::new();
    let f = live_sum(&mut o, SCRATCH_REGS.len() as u32 + 1);
    let r = compile_to_amx(&o, f, 1);
    assert!(is_scratch_overflow(&r), "SIL-1.A: 8 live values with 7 scratch must give ScratchOverflow, got {:?}", r.as_ref().map(|c| c.len()));
}

#[test]
fn sum_up_to_capacity_live_still_compiles() {
    let cap = SCRATCH_REGS.len() as u32;
    for live in [cap - 1, cap] {
        let mut o = Order::<4096>::new();
        let f = live_sum(&mut o, live);
        assert!(compile_to_amx(&o, f, 1).is_ok(), "{live} live values fit in {cap} scratch");
    }
}

/// SIL-1, group B: sub underflow branch, `sub dst, x16, rb` overwrote ra (== dst) before `add dst, dst, ra`.
#[test]
fn sub_underflow_does_not_write_ra_before_adding_it() {
    let mut o = Order::<4096>::new();
    let a0 = axis_of(&mut o, 2);
    let a1 = axis_of(&mut o, 6);
    let f = binary_of(&mut o, 6, a0, a1);
    let asm = compile_to_amx(&o, f, 2).unwrap();
    let lines: Vec<&str> = asm.lines().map(str::trim).collect();
    let cmp = lines.iter().find(|l| l.starts_with("cmp ")).expect("cmp ra, rb");
    let ra = cmp["cmp ".len()..].split(',').next().unwrap().trim();
    let sub16 = lines.iter().find(|l| l.starts_with("sub ") && l.contains(", x16, ")).expect("sub rd, x16, rb");
    let dst = sub16["sub ".len()..].split(',').next().unwrap().trim();
    assert_ne!(dst, ra, "SIL-1.B: `{sub16}` writes ra ({ra}) before adding it");
}

/// Review R2: `emit_loop` took registers straight from `next_scratch` without
/// touching `reg_stack`, so the exhaustion guard never saw loop registers.
#[test]
fn loop_whose_body_exceeds_scratch_capacity_reports_overflow() {
    let mut o = Order::<4096>::new();
    let f = loop_with_sum_body(&mut o, 5);
    let r = compile_to_amx(&o, f, 0);
    assert!(is_scratch_overflow(&r), "R2: 5 carried locals plus a two-live sum body exceed the scratch pool, got {:?}", r.as_ref().map(|c| c.len()));
}

/// Review R2, non-spurious guard: a shallow loop must keep compiling.
#[test]
fn shallow_loop_still_compiles() {
    for carried in [1u64, 4] {
        let mut o = Order::<4096>::new();
        let f = loop_with_sum_body(&mut o, carried);
        let r = compile_to_amx(&o, f, 0);
        assert!(r.is_ok(), "R2: loop with {carried} carried locals fits in the scratch pool, got {:?}", r.as_ref().map(|c| c.len()));
    }
}

/// Review round 2, R3: `pop_reg` decrements `next_scratch`, so popping each back-edge
/// value right after emitting it handed the next value the same register; with two
/// carried locals both received the last value.
#[test]
fn back_edge_with_two_carried_assigns_from_distinct_registers() {
    let mut o = Order::<4096>::new();
    let one = axis_of(&mut o, 1);
    // loop: [2 [[3 [[1 0] [3 [i0 [3 [i1 [0 1]]]]]]] [1 back_edge]]]
    let (i0, i1) = (quote_of(&mut o, 1), quote_of(&mut o, 2));
    let inits_tail = cons_of(&mut o, i1, one);
    let inits = cons_of(&mut o, i0, inits_tail);
    let slot = quote_of(&mut o, 0);
    let setup = cons_of(&mut o, slot, inits);
    // back-edge: [2 [[3 [[1 0] [3 [v0 [3 [v1 [0 1]]]]]]] [0 2]]]
    let (v0, v1) = (quote_of(&mut o, 10), quote_of(&mut o, 20));
    let vals_tail = cons_of(&mut o, v1, one);
    let vals = cons_of(&mut o, v0, vals_tail);
    let slot2 = quote_of(&mut o, 0);
    let new_subject = cons_of(&mut o, slot2, vals);
    let axis = axis_of(&mut o, 2);
    let back_edge = binary_of(&mut o, 2, new_subject, axis);
    let t1 = atom_of(&mut o, 1);
    let body = o.pair(t1, back_edge).unwrap();
    let f = binary_of(&mut o, 2, setup, body);
    let asm = compile_to_amx(&o, f, 0).expect("loop");
    let lines: Vec<&str> = asm.lines().map(str::trim).collect();
    let j = lines.iter().position(|l| l.starts_with("b .Lamx")).expect("back-edge branch");
    let source = |l: &str| l.split_once(',').expect("mov dst, src").1.trim().to_string();
    let (s0, s1) = (source(lines[j - 2]), source(lines[j - 1]));
    assert_ne!(s0, s1, "both carried locals are assigned from {s0}: the second value overwrote the first");
}
