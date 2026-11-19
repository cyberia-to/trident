//! Regression anchors for the AMX backend (SIL-1 and review round 3).
//! The pre-existing `mod tests` in amx.rs is separate and untouched.

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
