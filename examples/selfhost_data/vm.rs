//! VM feasibility probes: execute formulas on nox, never substitute host results.
use super::model::{atom, pair, value};
use nebu::Goldilocks;
use nox::{ErrorKind, NoTrace, NullCalls, Order, Outcome, Reduction, VecTrace};

fn quote(ar: &mut Reduction<2048>, n: Order) -> Order {
    let op = atom(ar, 1).unwrap();
    pair(ar, op, n).unwrap()
}

fn constant(ar: &mut Reduction<2048>, v: u64) -> Order {
    let n = atom(ar, v).unwrap();
    quote(ar, n)
}

fn axis(ar: &mut Reduction<2048>, i: u64) -> Order {
    let op = atom(ar, 0).unwrap();
    let i = atom(ar, i).unwrap();
    pair(ar, op, i).unwrap()
}

fn binary(ar: &mut Reduction<2048>, op: u64, a: Order, b: Order) -> Order {
    let op = atom(ar, op).unwrap();
    let body = pair(ar, a, b).unwrap();
    pair(ar, op, body).unwrap()
}

fn run(ar: &mut Reduction<2048>, input: Order, formula: Order) -> (Order, u64) {
    let outcome = nox::reduce(ar, input, formula, 10_000, &NullCalls, &mut NoTrace);
    match outcome {
        Outcome::Ok(n, remaining) => (n, 10_000 - remaining),
        other => panic!("native reduction failed: {other:?}"),
    }
}

#[test]
fn checked_atom_projection_rejects_pairs_on_the_vm() {
    let mut ar = Reduction::<2048>::new();
    let input = axis(&mut ar, 1);
    let zero = constant(&mut ar, 0);
    let formula = binary(&mut ar, 5, input, zero);
    let scalar = atom(&mut ar, 18_446_744_069_414_584_320).unwrap();
    assert_eq!(run(&mut ar, scalar, formula).0, scalar);
    let cell = pair(&mut ar, scalar, scalar).unwrap();
    assert!(matches!(
        nox::reduce(&mut ar, cell, formula, 100, &NullCalls, &mut NoTrace),
        Outcome::Error(ErrorKind::TypeError)
    ));
}

#[test]
fn checked_pair_projections_and_cons_use_native_structure() {
    let mut ar = Reduction::<2048>::new();
    let a = atom(&mut ar, 7).unwrap();
    let b = atom(&mut ar, 19).unwrap();
    let pair_value = pair(&mut ar, a, b).unwrap();
    let qa = quote(&mut ar, a);
    let qb = quote(&mut ar, b);
    let cons = binary(&mut ar, 3, qa, qb);
    assert_eq!(run(&mut ar, a, cons).0, pair_value);
    for (i, expected) in [(2, a), (3, b)] {
        let subject = axis(&mut ar, 1);
        let projection = axis(&mut ar, i);
        let continuation = quote(&mut ar, projection);
        let formula = binary(&mut ar, 2, subject, continuation);
        assert_eq!(run(&mut ar, pair_value, formula).0, expected);
        assert!(matches!(
            nox::reduce(&mut ar, a, formula, 100, &NullCalls, &mut NoTrace),
            Outcome::Error(ErrorKind::AxisError)
        ));
    }
}

#[test]
fn runtime_axis_is_computed_from_the_input_and_trace_modes_agree() {
    let mut ar = Reduction::<2048>::new();
    let data = axis(&mut ar, 2);
    let addr = axis(&mut ar, 3);
    let zero = constant(&mut ar, 0);
    let continuation = binary(&mut ar, 3, zero, addr);
    let formula = binary(&mut ar, 2, data, continuation);
    // Formula is fixed before distinct runtime addresses are supplied.
    let leaves: Vec<_> = [10, 20, 30, 40]
        .into_iter()
        .map(|v| atom(&mut ar, v).unwrap())
        .collect();
    let left = pair(&mut ar, leaves[0], leaves[1]).unwrap();
    let right = pair(&mut ar, leaves[2], leaves[3]).unwrap();
    let tree = pair(&mut ar, left, right).unwrap();
    for (i, expected) in leaves.into_iter().enumerate() {
        let index = atom(&mut ar, 4 + i as u64).unwrap();
        let input = pair(&mut ar, tree, index).unwrap();
        let (result, cost) = run(&mut ar, input, formula);
        assert_eq!(result, expected);
        assert_eq!(cost, 6);
        assert!(matches!(
            nox::reduce(&mut ar, input, formula, cost - 1, &NullCalls, &mut NoTrace),
            Outcome::Halt(_)
        ));
        assert!(
            matches!(nox::reduce(&mut ar, input, formula, cost, &NullCalls, &mut NoTrace),
            Outcome::Ok(n, 0) if n == expected)
        );
        let mut trace = VecTrace::default();
        assert!(
            matches!(nox::reduce(&mut ar, input, formula, 10_000, &NullCalls, &mut trace),
            Outcome::Ok(n, remaining) if n == result && 10_000 - remaining == cost)
        );
        assert_eq!(trace.0.len() as u64, cost);
    }
    let input = pair(&mut ar, tree, tree).unwrap();
    assert!(matches!(
        nox::reduce(&mut ar, input, formula, 100, &NullCalls, &mut NoTrace),
        Outcome::Error(ErrorKind::Malformed)
    ));
    assert!(matches!(
        nox::reduce(&mut ar, tree, formula, 0, &NullCalls, &mut NoTrace),
        Outcome::Halt(_)
    ));
}

#[test]
fn field_axis_can_select_the_last_position_of_a_height32_tree() {
    let mut ar = Reduction::<2048>::new();
    let mut root = atom(&mut ar, 77).unwrap();
    for _ in 0..32 {
        root = pair(&mut ar, root, root).unwrap();
    }
    let data = axis(&mut ar, 2);
    let addr = axis(&mut ar, 3);
    let zero = constant(&mut ar, 0);
    let continuation = binary(&mut ar, 3, zero, addr);
    let formula = binary(&mut ar, 2, data, continuation);
    // These are raw tree positions, not a claim that a 2^32-length Seq is valid.
    for index in [0, (1u64 << 31) - 1, (1u64 << 32) - 1] {
        let address = atom(&mut ar, (1u64 << 32) + index).unwrap();
        let input = pair(&mut ar, root, address).unwrap();
        let result = run(&mut ar, input, formula).0;
        assert_eq!(value(&ar, result), Ok(77));
    }
}

#[test]
fn identity_preserves_topology_and_eq_uses_native_boolean_convention() {
    let mut ar = Reduction::<2048>::new();
    let a = atom(&mut ar, 1).unwrap();
    let b = atom(&mut ar, 2).unwrap();
    let c = atom(&mut ar, 3).unwrap();
    let ab = pair(&mut ar, a, b).unwrap();
    let bc = pair(&mut ar, b, c).unwrap();
    let left = pair(&mut ar, ab, c).unwrap();
    let right = pair(&mut ar, a, bc).unwrap();
    let identity = axis(&mut ar, 0);
    let (hash_data, _) = run(&mut ar, left, identity);
    assert_eq!(ar.read_hash_data(hash_data).as_ref(), ar.digest(left));
    assert_ne!(ar.digest(left), ar.digest(right));
    let x = axis(&mut ar, 2);
    let y = axis(&mut ar, 3);
    let eq = binary(&mut ar, 9, x, y);
    for (other, expected) in [(left, 0), (right, 1)] {
        let input = pair(&mut ar, left, other).unwrap();
        let result = run(&mut ar, input, eq).0;
        assert_eq!(value(&ar, result), Ok(expected));
    }
}

#[test]
fn every_byte_value_at_every_position_extracts_with_existing_patterns() {
    // Use a fresh arena per position: historical allocations remain charged.
    for r in 0..4 {
        let mut ar = Reduction::<2048>::new();
        let word = axis(&mut ar, 1);
        let mask = constant(&mut ar, 255u64 << (8 * r));
        let masked = binary(&mut ar, 12, word, mask);
        let inverse = constant(&mut ar, Goldilocks::new(1u64 << (8 * r)).inv().as_u64());
        let formula = binary(&mut ar, 7, masked, inverse);
        for byte in 0..=255 {
            // Other positions are deliberately nonzero, including the top byte.
            let w = (u64::from(u32::MAX) & !(255u64 << (8 * r))) | (byte << (8 * r));
            let input = atom(&mut ar, w).unwrap();
            let result = run(&mut ar, input, formula).0;
            assert_eq!(value(&ar, result), Ok(byte));
        }
        let too_wide = atom(&mut ar, 1u64 << 32).unwrap();
        assert!(matches!(
            nox::reduce(&mut ar, too_wide, formula, 100, &NullCalls, &mut NoTrace),
            Outcome::Error(ErrorKind::TypeError)
        ));
    }
}
