//! Regression anchors for the Hexagon backend (SIL-1).

use super::*;
use nebu::Goldilocks;

fn atom_of(o: &mut Order<4096>, v: u64) -> NounId { o.atom(Goldilocks::new(v)).unwrap() }
fn axis_of(o: &mut Order<4096>, ax: u64) -> NounId {
    let t = atom_of(o, 0);
    let a = atom_of(o, ax);
    o.pair(t, a).unwrap()
}
fn binary_of(o: &mut Order<4096>, tag: u64, l: NounId, r: NounId) -> NounId {
    let body = o.pair(l, r).unwrap();
    let t = atom_of(o, tag);
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

#[test]
fn sum_with_capacity_plus_one_live_returns_scratch_overflow() {
    assert_eq!(SCRATCH_COUNT, 10);
    let mut o = Order::<4096>::new();
    let f = live_sum(&mut o, SCRATCH_COUNT + 1);
    let r = compile_to_hexagon(&o, f, 1);
    assert!(is_scratch_overflow(&r), "SIL-1.A: 11 live values with 10 scratch must give ScratchOverflow, got {:?}", r.as_ref().map(|c| c.len()));
}

#[test]
fn sum_up_to_capacity_live_still_compiles() {
    for live in [SCRATCH_COUNT - 1, SCRATCH_COUNT] {
        let mut o = Order::<4096>::new();
        let f = live_sum(&mut o, live);
        assert!(compile_to_hexagon(&o, f, 1).is_ok(), "{live} live values fit in {SCRATCH_COUNT} scratch");
    }
}

// -- group A2: the hexagon allocator must free on consume ------------------

fn quote_lit(o: &mut Order<4096>, v: u64) -> NounId {
    let t = atom_of(o, 1);
    let a = atom_of(o, v);
    o.pair(t, a).unwrap()
}

/// Minimal interpreter of the emitted text for jump-free formulas: executes
/// `{ Rd = ... }` over 32 u32 registers (the 32-bit semantics of phase 1)
/// and returns R0 on reaching `jumpr R31`.
fn run_asm(asm: &str, params: &[u32]) -> u32 {
    let mut r = [0u32; 32];
    for (i, p) in params.iter().enumerate() {
        r[i] = *p;
    }
    fn reg(s: &str) -> usize { s.trim().trim_start_matches('R').parse().expect("register") }
    fn val(r: &[u32; 32], s: &str) -> u32 {
        let s = s.trim();
        if let Some(h) = s.strip_prefix("##0x") {
            u32::from_str_radix(h, 16).unwrap()
        } else if let Some(d) = s.strip_prefix('#') {
            d.parse().unwrap()
        } else {
            r[reg(s)]
        }
    }
    for line in asm.lines() {
        let l = line.trim().trim_start_matches('{').trim_end_matches('}').trim();
        if l.is_empty() || l.starts_with("//") || l.starts_with('.') || l.ends_with(':') {
            continue;
        }
        if l == "jumpr R31" {
            return r[0];
        }
        let (dst, rhs) = l.split_once('=').expect("assignment");
        let rhs = rhs.trim();
        let new_value = if let Some(args) = rhs.strip_suffix(')') {
            let (op, a) = args.split_once('(').expect("operation");
            let v: Vec<u32> = a.split(',').map(|x| val(&r, x)).collect();
            match op {
                "add" => v[0].wrapping_add(v[1]),
                "sub" => v[0].wrapping_sub(v[1]),
                "mpyi" => v[0].wrapping_mul(v[1]),
                "xor" => v[0] ^ v[1],
                "and" => v[0] & v[1],
                "asl" => v[0].wrapping_shl(v[1]),
                "not" => !v[0],
                _ => panic!("operation not supported by the interpreter: {op}"),
            }
        } else {
            val(&r, rhs)
        };
        r[reg(dst)] = new_value;
    }
    panic!("code does not end in jumpr R31")
}

/// Right-nested sum of `live` distinct quotes 1..=live, all alive at once.
fn live_quote_sum(o: &mut Order<4096>, live: u32) -> NounId {
    let mut f = quote_lit(o, live as u64);
    for i in (1..live).rev() {
        let leaf = quote_lit(o, i as u64);
        f = binary_of(o, 5, leaf, f);
    }
    f
}

/// SIL-1, group A2: pop_reg did not decrement next_scratch; after 10 TOTAL
/// allocations R6 was recycled even though the live depth was 2. With the
/// overflow of A, a long chain at low depth gave a spurious Err.
#[test]
fn long_add_chain_at_low_depth_compiles_and_sums() {
    let mut o = Order::<4096>::new();
    let mut f = quote_lit(&mut o, 1);
    for i in 2..=16u64 {
        let leaf = quote_lit(&mut o, i);
        f = binary_of(&mut o, 5, f, leaf);
    }
    let asm = compile_to_hexagon(&o, f, 0).expect("live depth 2: not an overflow");
    assert_eq!(run_asm(&asm, &[]), 136);
}

/// With exactly the capacity of live values all registers are distinct and the result is the reference one.
#[test]
fn sum_at_capacity_live_gives_reference_value() {
    let mut o = Order::<4096>::new();
    let f = live_quote_sum(&mut o, SCRATCH_COUNT);
    let asm = compile_to_hexagon(&o, f, 0).expect("exact capacity");
    assert_eq!(run_asm(&asm, &[]), 55);
}

/// Guard: freeing on consume requires operands to stay alive while the other one is emitted.
/// sub(100, add(7, mul(3, 4))) = 81.
#[test]
fn nested_binaries_give_reference_value() {
    let mut o = Order::<4096>::new();
    let (c3, c4, c7, c100) = (quote_lit(&mut o, 3), quote_lit(&mut o, 4), quote_lit(&mut o, 7), quote_lit(&mut o, 100));
    let m = binary_of(&mut o, 7, c3, c4);
    let s = binary_of(&mut o, 5, c7, m);
    let f = binary_of(&mut o, 6, c100, s);
    let asm = compile_to_hexagon(&o, f, 0).unwrap();
    assert_eq!(run_asm(&asm, &[]), 81);
}

/// Guard: add over two parameters cannot read the same register twice.
#[test]
fn add_of_two_params_reads_distinct_registers() {
    let mut o = Order::<4096>::new();
    let a0 = axis_of(&mut o, 2);
    let a1 = axis_of(&mut o, 6);
    let f = binary_of(&mut o, 5, a0, a1);
    let asm = compile_to_hexagon(&o, f, 2).unwrap();
    assert_eq!(run_asm(&asm, &[1000, 20]), 1020);
    let line = asm.lines().find(|l| l.contains("= add(")).expect("add");
    let args = line.split_once("add(").unwrap().1.trim_end_matches([' ', '}', ')']);
    let (ra, rb) = args.split_once(',').unwrap();
    assert_ne!(ra.trim(), rb.trim(), "operands in the same register: {line}");
}

/// Guard: the value bound by a let stays alive while the body is compiled.
/// let x = 3+4 in 5+x = 12.
#[test]
fn let_keeps_bound_value_alive_during_body() {
    let mut o = Order::<4096>::new();
    let (c3, c4, c5) = (quote_lit(&mut o, 3), quote_lit(&mut o, 4), quote_lit(&mut o, 5));
    let value = binary_of(&mut o, 5, c3, c4);
    let one = axis_of(&mut o, 1);
    let t3 = atom_of(&mut o, 3);
    let cons = { let b = o.pair(value, one).unwrap(); o.pair(t3, b).unwrap() };
    let x = axis_of(&mut o, 2);
    let body = binary_of(&mut o, 5, c5, x);
    let t1 = atom_of(&mut o, 1);
    let quoted_body = o.pair(t1, body).unwrap();
    let f = binary_of(&mut o, 2, cons, quoted_body);
    let asm = compile_to_hexagon(&o, f, 0).expect("let");
    assert_eq!(run_asm(&asm, &[]), 12);
}

/// Guard: the new values of a back-edge with two carried locals come from distinct registers.
#[test]
fn back_edge_with_two_carried_assigns_from_distinct_registers() {
    let mut o = Order::<4096>::new();
    // loop: [2 [[3 [[1 0] [3 [i0 [3 [i1 [0 1]]]]]]] [1 back_edge]]]
    let (i0, i1) = (quote_lit(&mut o, 1), quote_lit(&mut o, 2));
    let one = axis_of(&mut o, 1);
    let t3 = atom_of(&mut o, 3);
    let inits_tail = { let b = o.pair(i1, one).unwrap(); o.pair(t3, b).unwrap() };
    let inits = { let b = o.pair(i0, inits_tail).unwrap(); o.pair(t3, b).unwrap() };
    let slot = quote_lit(&mut o, 0);
    let a = { let b = o.pair(slot, inits).unwrap(); o.pair(t3, b).unwrap() };
    // back-edge: [2 [[3 [[1 0] [3 [v0 [3 [v1 [0 1]]]]]]] [0 2]]]
    let (v0, v1) = (quote_lit(&mut o, 10), quote_lit(&mut o, 20));
    let vals_tail = { let b = o.pair(v1, one).unwrap(); o.pair(t3, b).unwrap() };
    let vals = { let b = o.pair(v0, vals_tail).unwrap(); o.pair(t3, b).unwrap() };
    let slot2 = quote_lit(&mut o, 0);
    let new_subject = { let b = o.pair(slot2, vals).unwrap(); o.pair(t3, b).unwrap() };
    let axis = axis_of(&mut o, 2);
    let back_edge = binary_of(&mut o, 2, new_subject, axis);
    let t1 = atom_of(&mut o, 1);
    let body = o.pair(t1, back_edge).unwrap();
    let f = binary_of(&mut o, 2, a, body);
    let asm = compile_to_hexagon(&o, f, 0).expect("loop");
    let lines: Vec<&str> = asm.lines().collect();
    let j = lines.iter().position(|l| l.contains("jump .L")).expect("back-edge");
    let source = |l: &str| l.split_once('=').unwrap().1.trim().trim_end_matches('}').trim().to_string();
    let (s0, s1) = (source(lines[j - 2]), source(lines[j - 1]));
    assert_ne!(s0, s1, "both carried locals are assigned from {s0}: the second value overwrote the first");
}
