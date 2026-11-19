//! Regression anchors for the arm64 backend (criba, SIL-1 and review round 3).

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
fn words_of(code: &[u8]) -> Vec<u32> {
    code.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
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

/// Compiles `[9 [a0 a1]]` (nox eq) with 2 parameters and returns the 32-bit words.
fn eq_words() -> Vec<u32> {
    let mut o = Order::<1024>::new();
    let ax2 = o.atom(Goldilocks::new(2)).unwrap();
    let t0 = o.atom(Goldilocks::new(0)).unwrap();
    let a0 = o.pair(t0, ax2).unwrap();
    let ax6 = o.atom(Goldilocks::new(6)).unwrap();
    let t0b = o.atom(Goldilocks::new(0)).unwrap();
    let a1 = o.pair(t0b, ax6).unwrap();
    let body = o.pair(a0, a1).unwrap();
    let tag = o.atom(Goldilocks::new(9)).unwrap();
    let formula = o.pair(tag, body).unwrap();
    let code = compile_to_arm64(&o, formula, 2).unwrap();
    code.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// nox eq: 0 if equal, 1 if different => CSET dst, ne = CSINC Xd, XZR, XZR, EQ.
/// CSINC (64 bit) = 0x9A800400 | Rm<<16 | cond<<12 | Rn<<5 | Rd; XZR=31; EQ=0b0000
/// => base 0x9A9F07E0. Base 0x9A9F17E0 carries cond=NE (=CSET eq, 1 if equal).
#[test]
fn eq_emits_cset_ne_with_eq_condition() {
    let words = eq_words();
    let cset_ne = words.iter().filter(|w| (**w & !0x1F) == 0x9A9F07E0).count();
    let cset_eq = words.iter().filter(|w| (**w & !0x1F) == 0x9A9F17E0).count();
    assert_eq!(cset_eq, 0, "silicon-src-2.CL-LOG.6: 0x9A9F17E0 is CSET eq (1 if EQUAL), nox contract requires 1 if NE");
    assert_eq!(cset_ne, 1, "silicon-src-2.CL-LOG.6: eq must emit exactly one CSINC Xd,XZR,XZR,EQ (0x9A9F07E0)");
}

/// SIL-1 (issue #130), group A: with more live values than scratch registers the
/// allocator handed out an already-live register without error.
#[test]
fn sum_with_capacity_plus_one_live_returns_scratch_overflow() {
    assert_eq!(SCRATCH_COUNT, 7);
    let mut o = Order::<4096>::new();
    let f = live_sum(&mut o, SCRATCH_COUNT as u32 + 1);
    let r = compile_to_arm64(&o, f, 1);
    assert!(is_scratch_overflow(&r), "SIL-1.A: 8 live values with 7 scratch must give ScratchOverflow, got {:?}", r.as_ref().map(|c| c.len()));
}

#[test]
fn sum_up_to_capacity_live_still_compiles() {
    for live in [SCRATCH_COUNT as u32 - 1, SCRATCH_COUNT as u32] {
        let mut o = Order::<4096>::new();
        let f = live_sum(&mut o, live);
        assert!(compile_to_arm64(&o, f, 1).is_ok(), "{live} live values fit in {SCRATCH_COUNT} scratch");
    }
}

/// SIL-1, group B: after pop(rb),pop(ra),push(dst) dst aliases ra; MUL dst,ra,rb
/// wrote ra before UMULH read it.
/// Not covered: execution on ARM64 hardware.
#[test]
fn mul_emits_umulh_before_mul_writes_dst() {
    let mut o = Order::<4096>::new();
    let a0 = axis_of(&mut o, 2);
    let a1 = axis_of(&mut o, 6);
    let f = binary_of(&mut o, 7, a0, a1);
    let w = words_of(&compile_to_arm64(&o, f, 2).unwrap());
    let i_mul = w.iter().position(|x| (*x & 0xFFE0_FC00) == 0x9B00_7C00).expect("MUL");
    let i_umulh = w.iter().position(|x| (*x & 0xFFE0_FC00) == 0x9BC0_7C00).expect("UMULH");
    let mul = w[i_mul];
    assert_eq!(mul & 0x1F, (mul >> 5) & 0x1F, "premise: dst aliases ra");
    assert!(i_umulh < i_mul, "SIL-1.B: UMULH (idx {i_umulh}) reads ra after MUL (idx {i_mul}) wrote it");
}

/// SIL-1, group B: sub underflow branch, `sub dst,x16,rb` overwrote ra (== dst) before `add dst,dst,ra`.
#[test]
fn sub_underflow_does_not_write_ra_before_adding_it() {
    let mut o = Order::<4096>::new();
    let a0 = axis_of(&mut o, 2);
    let a1 = axis_of(&mut o, 6);
    let f = binary_of(&mut o, 6, a0, a1);
    let w = words_of(&compile_to_arm64(&o, f, 2).unwrap());
    let ra = (w[0] & 0x1F) as u8; // first mov: destination of the first operand
    let i = w.iter()
        .position(|x| (*x & 0xFFE0_FC00) == 0xCB00_0000 && ((*x >> 5) & 0x1F) == 16)
        .expect("sub rd, x16, rb of the underflow branch");
    assert_ne!((w[i] & 0x1F) as u8, ra, "SIL-1.B: sub rd,x16,rb writes ra (x{ra}) before add rd,rd,ra");
}

/// Review R2: `emit_loop` took registers straight from `next_scratch` without
/// touching `reg_stack`, so the exhaustion guard never saw loop registers.
#[test]
fn loop_whose_body_exceeds_scratch_capacity_reports_overflow() {
    let mut o = Order::<4096>::new();
    let f = loop_with_sum_body(&mut o, 5);
    let r = compile_to_arm64(&o, f, 0);
    assert!(is_scratch_overflow(&r), "R2: 5 carried locals plus a two-live sum body exceed the scratch pool, got {:?}", r.as_ref().map(|c| c.len()));
}

/// Review R2, non-spurious guard: a shallow loop must keep compiling.
#[test]
fn shallow_loop_still_compiles() {
    for carried in [1u64, 4] {
        let mut o = Order::<4096>::new();
        let f = loop_with_sum_body(&mut o, carried);
        let r = compile_to_arm64(&o, f, 0);
        assert!(r.is_ok(), "R2: loop with {carried} carried locals fits in the scratch pool, got {:?}", r.as_ref().map(|c| c.len()));
    }
}
