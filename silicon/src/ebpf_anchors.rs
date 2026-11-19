//! Regression anchors for the eBPF backend (criba, SIL-1 and review round 3).

use super::*;
use nebu::Goldilocks;

fn atom_of<const N: usize>(o: &mut Order<N>, v: u64) -> NounId { o.atom(Goldilocks::new(v)).unwrap() }
fn axis_of<const N: usize>(o: &mut Order<N>, ax: u64) -> NounId {
    let t = atom_of(o, 0);
    let a = atom_of(o, ax);
    o.pair(t, a).unwrap()
}
fn quote_of<const N: usize>(o: &mut Order<N>, v: u64) -> NounId {
    let t = atom_of(o, 1);
    let a = atom_of(o, v);
    o.pair(t, a).unwrap()
}
fn binary_of<const N: usize>(o: &mut Order<N>, tag: u64, l: NounId, r: NounId) -> NounId {
    let body = o.pair(l, r).unwrap();
    let t = atom_of(o, tag);
    o.pair(t, body).unwrap()
}
fn cons_of<const N: usize>(o: &mut Order<N>, head: NounId, tail: NounId) -> NounId {
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

fn instructions(code: &[u8]) -> Vec<u64> {
    code.chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect()
}
/// ALU64 AND K instructions with imm = -1 (opcode 0x57): identity, masks nothing.
fn and64_k_minus_one(code: &[u8]) -> usize {
    instructions(code).iter().filter(|i| (**i & 0xFF) == 0x57 && (**i >> 32) == 0xFFFF_FFFF).count()
}
/// ALU32 AND K instructions with imm = -1 (opcode 0x54): mask to 32 bits.
fn and32_k_minus_one(code: &[u8]) -> usize {
    instructions(code).iter().filter(|i| (**i & 0xFF) == 0x54 && (**i >> 32) == 0xFFFF_FFFF).count()
}

/// Covers: ALU32 helper (opcode 0x54) and the full emit_mul flow (pattern 7).
/// Not covered: execution in an eBPF kernel/VM.
#[test]
fn mul_masks_to_32_bits_with_alu32_not_and64_minus_one() {
    assert_eq!(ebpf_and32_imm(0, -1) & 0xFF, 0x54, "ALU32|K|AND = 0x54");
    let mut o = Order::<1024>::new();
    let a0 = axis_of(&mut o, 2);
    let a1 = axis_of(&mut o, 6);
    let formula = binary_of(&mut o, 7, a0, a1);
    let code = compile_to_ebpf(&o, formula, 2).unwrap();
    assert_eq!(and64_k_minus_one(&code), 0,
        "silicon-src-3.CL-LOG.2: ALU64 AND K with imm -1 is identity, it does not mask to 32 bits");
    assert_eq!(and32_k_minus_one(&code), 4,
        "emit_mul must mask with ALU32 AND K -1 in exactly 4 places: al, bl, mid1, mid2");
}

/// Same defect class in emit_not (pattern 13): AND 0xFFFFFFFF as ALU64 K (-1) is identity.
#[test]
fn not_masks_to_32_bits_with_alu32() {
    let mut o = Order::<1024>::new();
    let a0 = axis_of(&mut o, 2);
    let t = atom_of(&mut o, 13);
    let formula = o.pair(t, a0).unwrap();
    let code = compile_to_ebpf(&o, formula, 1).unwrap();
    assert_eq!(and64_k_minus_one(&code), 0, "emit_not: AND64 K -1 is identity");
    assert_eq!(and32_k_minus_one(&code), 1, "emit_not must mask with ALU32 AND K -1");
}

/// Same defect class in emit_shl (pattern 14).
#[test]
fn shl_masks_to_32_bits_with_alu32() {
    let mut o = Order::<1024>::new();
    let a0 = axis_of(&mut o, 2);
    let a1 = axis_of(&mut o, 6);
    let formula = binary_of(&mut o, 14, a0, a1);
    let code = compile_to_ebpf(&o, formula, 2).unwrap();
    assert_eq!(and64_k_minus_one(&code), 0, "emit_shl: AND64 K -1 is identity");
    assert_eq!(and32_k_minus_one(&code), 1, "emit_shl must mask with ALU32 AND K -1");
}

#[test]
fn sum_with_capacity_plus_one_live_returns_scratch_overflow() {
    assert_eq!(SCRATCH_REGS.len(), 4);
    let mut o = Order::<4096>::new();
    let f = live_sum(&mut o, SCRATCH_REGS.len() as u32 + 1);
    let r = compile_to_ebpf(&o, f, 1);
    assert!(is_scratch_overflow(&r), "SIL-1.A: 5 live values with 4 scratch must give ScratchOverflow, got {:?}", r.as_ref().map(|c| c.len()));
}

#[test]
fn sum_up_to_capacity_live_still_compiles() {
    let cap = SCRATCH_REGS.len() as u32;
    for live in [cap - 1, cap] {
        let mut o = Order::<4096>::new();
        let f = live_sum(&mut o, live);
        assert!(compile_to_ebpf(&o, f, 1).is_ok(), "{live} live values fit in {cap} scratch");
    }
}

/// Review R2: `emit_loop` took registers straight from `next_scratch` without
/// touching `reg_stack`, so the exhaustion guard never saw loop registers.
#[test]
fn loop_whose_body_exceeds_scratch_capacity_reports_overflow() {
    let mut o = Order::<4096>::new();
    let f = loop_with_sum_body(&mut o, 3);
    let r = compile_to_ebpf(&o, f, 0);
    assert!(is_scratch_overflow(&r), "R2: 3 carried locals plus a two-live sum body exceed the scratch pool, got {:?}", r.as_ref().map(|c| c.len()));
}

/// Review R2, non-spurious guard: a shallow loop must keep compiling.
#[test]
fn shallow_loop_still_compiles() {
    for carried in [1u64, 1] {
        let mut o = Order::<4096>::new();
        let f = loop_with_sum_body(&mut o, carried);
        let r = compile_to_ebpf(&o, f, 0);
        assert!(r.is_ok(), "R2: loop with {carried} carried locals fits in the scratch pool, got {:?}", r.as_ref().map(|c| c.len()));
    }
}
