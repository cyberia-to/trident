//! Regression anchors for the RV64 backend (SIL-1 and review round 3).

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
fn words_of(code: &[u8]) -> Vec<u32> {
    code.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}
fn is_mul(w: u32) -> bool { (w & 0xFE00_707F) == 0x0200_0033 }
fn is_mulhu(w: u32) -> bool { (w & 0xFE00_707F) == 0x0200_3033 }
fn rd_of(w: u32) -> u32 { (w >> 7) & 0x1F }
fn rs1_of(w: u32) -> u32 { (w >> 15) & 0x1F }
fn rs2_of(w: u32) -> u32 { (w >> 20) & 0x1F }

#[test]
fn sum_with_capacity_plus_one_live_returns_scratch_overflow() {
    let mut o = Order::<4096>::new();
    let f = live_sum(&mut o, SCRATCH_REGS.len() as u32 + 1);
    let r = compile_to_rv64(&o, f, 1);
    assert!(is_scratch_overflow(&r), "SIL-1.A: capacity+1 live values must give ScratchOverflow, got {:?}", r.as_ref().map(|c| c.len()));
}

#[test]
fn sum_up_to_capacity_live_still_compiles() {
    let cap = SCRATCH_REGS.len() as u32;
    for live in [cap - 1, cap] {
        let mut o = Order::<4096>::new();
        let f = live_sum(&mut o, live);
        assert!(compile_to_rv64(&o, f, 1).is_ok(), "{live} live values fit in {cap} scratch");
    }
}

/// SIL-1, group C: x5 (t0) is the fixed temporary of sub/not/shl/mul and cannot be a value scratch.
#[test]
fn x5_is_not_a_value_scratch() {
    assert!(!SCRATCH_REGS.contains(&5), "SIL-1.C: x5 (t0) is a dedicated temporary, it cannot carry values");
    assert_eq!(SCRATCH_REGS.len(), 5);
}

/// SIL-1, group C: in a compiled mul, x5 does not appear as rd/rs1/rs2 of MUL nor as a source of MULHU
/// (it is MULHU's temporary destination).
#[test]
fn mul_does_not_use_x5_as_value_operand() {
    let mut o = Order::<4096>::new();
    let a0 = axis_of(&mut o, 2);
    let a1 = axis_of(&mut o, 6);
    let f = binary_of(&mut o, 7, a0, a1);
    let w = words_of(&compile_to_rv64(&o, f, 2).unwrap());
    let fields = |x: u32| ((x >> 7) & 0x1F, (x >> 15) & 0x1F, (x >> 20) & 0x1F);
    let mul = w.iter().find(|x| (**x & 0xFE00_707F) == 0x0200_0033).expect("MUL");
    let mulhu = w.iter().find(|x| (**x & 0xFE00_707F) == 0x0200_3033).expect("MULHU");
    let (rd, rs1, rs2) = fields(*mul);
    assert!(rd != 5 && rs1 != 5 && rs2 != 5, "SIL-1.C: mul x{rd}, x{rs1}, x{rs2} uses x5 (t0) as a value");
    let (_, rs1h, rs2h) = fields(*mulhu);
    assert!(rs1h != 5 && rs2h != 5, "SIL-1.C: mulhu reads x{rs1h}/x{rs2h}; x5 is its temporary destination");
}

/// Review R2: `emit_loop` took registers straight from `next_scratch` without
/// touching `reg_stack`, so the exhaustion guard never saw loop registers.
#[test]
fn loop_whose_body_exceeds_scratch_capacity_reports_overflow() {
    let mut o = Order::<4096>::new();
    let f = loop_with_sum_body(&mut o, 3);
    let r = compile_to_rv64(&o, f, 0);
    assert!(is_scratch_overflow(&r), "R2: 3 carried locals plus a two-live sum body exceed the scratch pool, got {:?}", r.as_ref().map(|c| c.len()));
}

/// Review R2, non-spurious guard: a shallow loop must keep compiling.
#[test]
fn shallow_loop_still_compiles() {
    for carried in [1u64, 2] {
        let mut o = Order::<4096>::new();
        let f = loop_with_sum_body(&mut o, carried);
        let r = compile_to_rv64(&o, f, 0);
        assert!(r.is_ok(), "R2: loop with {carried} carried locals fits in the scratch pool, got {:?}", r.as_ref().map(|c| c.len()));
    }
}

/// `mul(a1, a2)` over two parameters, compiled for RV64.
fn compiled_mul_words() -> Vec<u32> {
    let mut o = Order::<4096>::new();
    let a0 = axis_of(&mut o, 2);
    let a1 = axis_of(&mut o, 6);
    let f = binary_of(&mut o, 7, a0, a1);
    words_of(&compile_to_rv64(&o, f, 2).unwrap())
}

/// Review R1(a): pop,pop,push makes `dst` alias `ra`, so a MUL emitted before MULHU
/// overwrote the operand MULHU still had to read.
#[test]
fn mulhu_is_emitted_before_mul_writes_dst() {
    let w = compiled_mul_words();
    let i_mul = w.iter().position(|x| is_mul(*x)).expect("MUL");
    let i_mulhu = w.iter().position(|x| is_mulhu(*x)).expect("MULHU");
    assert_eq!(rd_of(w[i_mul]), rs1_of(w[i_mul]), "premise: dst aliases ra");
    assert!(i_mulhu < i_mul, "R1: MULHU (word {i_mulhu}) reads ra after MUL (word {i_mul}) wrote it");
}

/// Review R1(b): x6 (t1) is a mul temporary, so it must not be handed out as a value register.
#[test]
fn both_dedicated_temporaries_are_outside_the_value_pool() {
    assert!(!SCRATCH_REGS.contains(&5), "R1: x5 (t0) is a dedicated temporary");
    assert!(!SCRATCH_REGS.contains(&6), "R1: x6 (t1) is a dedicated temporary of mul");
    assert_eq!(SCRATCH_REGS.len(), 5);
}

/// Review R1(b), behavioural: in `add(a, mul(b, c))` the pending operand `a` sits in a pool
/// register while the mul sequence runs; no instruction before the final add may write it.
#[test]
fn mul_temporaries_do_not_overwrite_a_pending_operand() {
    let mut o = Order::<4096>::new();
    let pending = axis_of(&mut o, 2);
    let (b, c) = (axis_of(&mut o, 6), axis_of(&mut o, 14));
    let m = binary_of(&mut o, 7, b, c);
    let f = binary_of(&mut o, 5, pending, m);
    let w = words_of(&compile_to_rv64(&o, f, 3).unwrap());
    let held = rd_of(w[0]); // first instruction: mv held, param
    let is_add = |x: u32| (x & 0xFE00_707F) == 0x0000_0033;
    let final_add = w.iter().position(|x| is_add(*x) && rd_of(*x) == held && rs1_of(*x) == held)
        .expect("final add of the pending operand");
    for (i, x) in w[1..final_add].iter().enumerate() {
        let op = x & 0x7F;
        let writes_rd = op == 0x33 || op == 0x13 || op == 0x37 || op == 0x6F;
        assert!(!(writes_rd && rd_of(*x) == held),
            "R1: word {} writes x{held} while it holds a pending operand", i + 1);
    }
}

/// Review R1: MUL and MULHU must not use the dedicated temporaries as value operands,
/// and MULHU is the only one that targets x5.
#[test]
fn mul_operands_avoid_dedicated_temporaries() {
    let w = compiled_mul_words();
    let mul = *w.iter().find(|x| is_mul(**x)).expect("MUL");
    let mulhu = *w.iter().find(|x| is_mulhu(**x)).expect("MULHU");
    for r in [rd_of(mul), rs1_of(mul), rs2_of(mul), rs1_of(mulhu), rs2_of(mulhu)] {
        assert!(r != 5 && r != 6, "R1: mul/mulhu operand field uses x{r}");
    }
    assert_eq!(rd_of(mulhu), 5);
}
