use super::model::*;
use nox::Reduction;

fn noun<const N: usize>(ar: &mut Reduction<N>, tag: u64, len: u64, tree: nox::Order) -> nox::Order {
    let tag = atom(ar, tag).unwrap();
    let len = atom(ar, len).unwrap();
    let body = pair(ar, len, tree).unwrap();
    pair(ar, tag, body).unwrap()
}

#[test]
fn exact_shapes_preserve_length_domain_and_padding() {
    let mut ar = Reduction::<1024>::new();
    let cases: &[(&[u8], &str)] = &[
        (&[], "[1113150513 [0 0]]"),
        (&[0], "[1113150513 [1 0]]"),
        (&[1, 2, 3, 4], "[1113150513 [4 67305985]]"),
        (&[1, 2, 3, 4, 5], "[1113150513 [5 [67305985 5]]]"),
        (
            &[0, 1, 2, 3, 4, 5, 6, 7, 255],
            "[1113150513 [9 [[50462976 117835012] [255 0]]]]",
        ),
    ];
    let mut identities = std::collections::BTreeSet::new();
    for (bytes, expected) in cases {
        let b = Bytes::from_slice(&mut ar, bytes, 16).unwrap();
        let root = b.encode(&mut ar).unwrap();
        assert_eq!(display(&ar, root, &mut 100).unwrap(), *expected);
        identities.insert(nox::data::hash::digest_bytes(ar.digest(root).unwrap()));
        let decoded = Bytes::decode(&mut ar, root, 16, 100).unwrap();
        let actual: Vec<_> = (0..decoded.len())
            .map(|i| decoded.get(&ar, i).unwrap())
            .collect();
        assert_eq!(&actual, bytes);
    }
    let empty = Seq::from_values(&mut ar, &[], 16)
        .unwrap()
        .encode(&mut ar)
        .unwrap();
    assert_eq!(display(&ar, empty, &mut 100).unwrap(), "[1397051697 [0 0]]");
    identities.insert(nox::data::hash::digest_bytes(ar.digest(empty).unwrap()));
    assert_eq!(identities.len(), cases.len() + 1);
}

#[test]
fn byte_append_and_set_match_bulk_encoding_at_every_boundary() {
    let mut ar = Reduction::<4096>::new();
    let mut data = Bytes::from_slice(&mut ar, &[], 128).unwrap();
    let mut expected = Vec::new();
    for i in 0..65 {
        let byte = ((i * 97) % 256) as u8;
        let old = data;
        let old_root = old.encode(&mut ar).unwrap();
        data = data.push(&mut ar, u32::from(byte), 128).unwrap();
        assert_eq!(old.encode(&mut ar).unwrap(), old_root);
        assert_eq!(old.len() as usize, expected.len());
        for (i, byte) in expected.iter().enumerate() {
            assert_eq!(old.get(&ar, i as u32).unwrap(), *byte);
        }
        expected.push(byte);
        let bulk = Bytes::from_slice(&mut ar, &expected, 128).unwrap();
        assert_eq!(data.encode(&mut ar).unwrap(), bulk.encode(&mut ar).unwrap());
    }
    for i in 0..data.len() {
        let mut changed = expected.clone();
        changed[i as usize] = 255;
        let changed_data = data.set(&mut ar, i, 255).unwrap();
        let bulk = Bytes::from_slice(&mut ar, &changed, 128).unwrap();
        assert_eq!(
            changed_data.encode(&mut ar).unwrap(),
            bulk.encode(&mut ar).unwrap()
        );
        assert_eq!(data.get(&ar, i).unwrap(), expected[i as usize]);
    }
}

#[test]
fn sequence_can_hold_pairs_and_persistent_updates_share_siblings() {
    let mut ar = Reduction::<1024>::new();
    let a = atom(&mut ar, 11).unwrap();
    let b = atom(&mut ar, 22).unwrap();
    let c = pair(&mut ar, a, b).unwrap();
    let values = [a, b, c, a, b];
    let bulk = Seq::from_values(&mut ar, &values, 8).unwrap();
    let mut appended = Seq::from_values(&mut ar, &[], 8).unwrap();
    for v in values {
        appended = appended.push(&mut ar, v, 8).unwrap();
    }
    let root = bulk.encode(&mut ar).unwrap();
    assert_eq!(appended.encode(&mut ar).unwrap(), root);
    let decoded = Seq::decode(&mut ar, root, 8, 32).unwrap();
    assert_eq!(decoded.len(), 5);
    assert_eq!(decoded.get(&ar, 2), Ok(c));
    let old_tree = ar.tail(ar.tail(root).unwrap()).unwrap();
    let before = ar.count();
    let updated = decoded.set(&mut ar, 0, c).unwrap();
    // A three-level update creates at most three new branch nodes.
    assert!(ar.count() - before <= 3);
    let updated_root = updated.encode(&mut ar).unwrap();
    let new_tree = ar.tail(ar.tail(updated_root).unwrap()).unwrap();
    assert_eq!(ar.tail(old_tree), ar.tail(new_tree));
    assert_eq!(decoded.get(&ar, 0), Ok(a));
    assert_eq!(updated.get(&ar, 0), Ok(c));
}

#[test]
fn malformed_layouts_and_noncanonical_padding_fail() {
    let mut ar = Reduction::<1024>::new();
    let zero = atom(&mut ar, 0).unwrap();
    let one = atom(&mut ar, 1).unwrap();
    let two = atom(&mut ar, 2).unwrap();
    let high_byte = atom(&mut ar, 256).unwrap();
    let cases = [
        (noun(&mut ar, SEQ, 0, zero), Error::Tag),
        (noun(&mut ar, BYTES, 0, one), Error::Padding),
        (noun(&mut ar, BYTES, 5, one), Error::Shape),
        (noun(&mut ar, BYTES, 65, zero), Error::Limit),
        (noun(&mut ar, BYTES, 1u64 << 32, zero), Error::Range),
        (noun(&mut ar, BYTES, 1, high_byte), Error::Padding),
    ];
    for (root, error) in cases {
        assert_eq!(Bytes::decode(&mut ar, root, 64, 100).unwrap_err(), error);
    }
    let wide = atom(&mut ar, 1u64 << 32).unwrap();
    let root = noun(&mut ar, BYTES, 4, wide);
    assert_eq!(
        Bytes::decode(&mut ar, root, 64, 100).unwrap_err(),
        Error::Range
    );
    let payload_pair = pair(&mut ar, zero, zero).unwrap();
    let root = noun(&mut ar, BYTES, 1, payload_pair);
    assert_eq!(
        Bytes::decode(&mut ar, root, 64, 100).unwrap_err(),
        Error::Shape
    );
    let left = pair(&mut ar, one, two).unwrap();
    let wrong_right = pair(&mut ar, one, one).unwrap();
    let tree = pair(&mut ar, left, wrong_right).unwrap();
    let root = noun(&mut ar, SEQ, 3, tree);
    assert_eq!(
        Seq::decode(&mut ar, root, 64, 100).unwrap_err(),
        Error::Padding
    );
    // Length three must have a two-level root; atom zero is not a zero subtree.
    let root = noun(&mut ar, SEQ, 3, zero);
    assert_eq!(
        Seq::decode(&mut ar, root, 64, 100).unwrap_err(),
        Error::Shape
    );
}

#[test]
fn validation_work_and_operations_obey_explicit_limits() {
    let mut ar = Reduction::<1024>::new();
    let data = Bytes::from_slice(&mut ar, &[1, 2, 3, 4, 5], 5).unwrap();
    let root = data.encode(&mut ar).unwrap();
    // Three shape visits, then two two-node word reads.
    assert_eq!(
        Bytes::decode(&mut ar, root, 5, 6).unwrap_err(),
        Error::Visits
    );
    assert!(Bytes::decode(&mut ar, root, 5, 7).is_ok());
    assert_eq!(data.push(&mut ar, 0, 5).unwrap_err(), Error::Limit);
    assert_eq!(data.push(&mut ar, 256, 6).unwrap_err(), Error::Range);
    assert_eq!(data.set(&mut ar, 0, 256).unwrap_err(), Error::Range);
    assert_eq!(data.set(&mut ar, 5, 0).unwrap_err(), Error::Index);
    assert_eq!(data.get(&ar, 5), Err(Error::Index));
    assert_eq!(
        Bytes::from_slice(&mut ar, &[1, 2], 1).unwrap_err(),
        Error::Limit
    );
    assert_eq!(height(u32::MAX), 32);
    assert_eq!(word_count(u32::MAX), 1 << 30);
    assert_eq!(height(word_count(u32::MAX)), 30);
    let zero = atom(&mut ar, 0).unwrap();
    let root = noun(&mut ar, SEQ, u64::from(u32::MAX), zero);
    assert_eq!(
        Seq::decode(&mut ar, root, 100, 1).unwrap_err(),
        Error::Limit
    );
    assert_eq!(
        Seq::decode(&mut ar, root, u32::MAX, 0).unwrap_err(),
        Error::Visits
    );
    assert_eq!(atom(&mut ar, 18_446_744_069_414_584_321), Err(Error::Range));
    assert!(atom(&mut ar, 18_446_744_069_414_584_320).is_ok());
}

#[test]
fn arena_exhaustion_preserves_old_root() {
    let mut ar = Reduction::<64>::new();
    let data = Bytes::from_slice(&mut ar, &[1, 2, 3, 4], 8).unwrap();
    let root = data.encode(&mut ar).unwrap();
    let identity = *ar.digest(root).unwrap();
    for v in 100..1000 {
        if atom(&mut ar, v) == Err(Error::Allocation) {
            break;
        }
    }
    assert_eq!(data.set(&mut ar, 0, 255).unwrap_err(), Error::Allocation);
    assert_eq!(data.get(&ar, 0), Ok(1));
    assert_eq!(ar.digest(root), Some(&identity));
}

#[test]
fn partial_path_allocation_failure_preserves_every_old_value() {
    let mut ar = Reduction::<64>::new();
    let values: Vec<_> = (1..=8).map(|v| atom(&mut ar, v).unwrap()).collect();
    let seq = Seq::from_values(&mut ar, &values, 8).unwrap();
    let old_root = seq.encode(&mut ar).unwrap();
    let old_identity = *ar.digest(old_root).unwrap();
    let replacement = atom(&mut ar, 999).unwrap();
    for v in 1000..1100 {
        if ar.count() >= 46 {
            break;
        }
        atom(&mut ar, v).unwrap();
    }
    assert_eq!(ar.count(), 46);
    assert_eq!(
        seq.set(&mut ar, 0, replacement).unwrap_err(),
        Error::Allocation
    );
    // Two new branch nodes fit; allocation of the new root fails at 3N/4.
    assert_eq!(ar.count(), 48);
    for (i, old) in values.into_iter().enumerate() {
        assert_eq!(seq.get(&ar, i as u32), Ok(old));
    }
    assert_eq!(ar.digest(old_root), Some(&old_identity));
}

#[test]
fn repeated_subtrees_consume_visits_and_zero_padding_keeps_its_shape() {
    let mut ar = Reduction::<1024>::new();
    let zero = atom(&mut ar, 0).unwrap();
    let mut tree = zero;
    for _ in 0..5 {
        tree = pair(&mut ar, tree, tree).unwrap();
    }
    let root = noun(&mut ar, SEQ, 32, tree);
    // Sharing is allocation-efficient but cannot skip logical validation work.
    assert_eq!(
        Seq::decode(&mut ar, root, 32, 62).unwrap_err(),
        Error::Visits
    );
    assert!(Seq::decode(&mut ar, root, 32, 63).is_ok());
    let pair_zero = pair(&mut ar, zero, zero).unwrap();
    let full_left = pair(&mut ar, pair_zero, pair_zero).unwrap();
    // Length5 needs [E(2) [E(1) E(1)]]. The last E(1) is wholly unused.
    let short_right = pair(&mut ar, pair_zero, zero).unwrap();
    let bad_tree = pair(&mut ar, full_left, short_right).unwrap();
    let bad = noun(&mut ar, SEQ, 5, bad_tree);
    assert_eq!(
        Seq::decode(&mut ar, bad, 5, 100).unwrap_err(),
        Error::Padding
    );
}

#[test]
fn empty_collections_and_sequence_bounds_fail_before_access() {
    let mut ar = Reduction::<1024>::new();
    let zero = atom(&mut ar, 0).unwrap();
    let seq = Seq::from_values(&mut ar, &[], 0).unwrap();
    assert_eq!(seq.get(&ar, 0), Err(Error::Index));
    assert_eq!(seq.set(&mut ar, 0, zero).unwrap_err(), Error::Index);
    assert_eq!(seq.push(&mut ar, zero, 0).unwrap_err(), Error::Limit);
    let bytes = Bytes::from_slice(&mut ar, &[], 0).unwrap();
    assert_eq!(bytes.get(&ar, 0), Err(Error::Index));
    assert_eq!(bytes.push(&mut ar, 0, 0).unwrap_err(), Error::Limit);
    assert_eq!(
        Seq::from_values(&mut ar, &[zero], 0).unwrap_err(),
        Error::Limit
    );
    let raw = seq.encode(&mut ar).unwrap();
    assert_eq!(Seq::decode(&mut ar, raw, 0, 0).unwrap_err(), Error::Visits);
    assert!(Seq::decode(&mut ar, raw, 0, 1).is_ok());
}

#[test]
fn allocation_order_does_not_change_identity_or_shape() {
    let mut a = Reduction::<1024>::new();
    let mut b = Reduction::<1024>::new();
    for i in 100..110 {
        atom(&mut b, i).unwrap();
    }
    let a_root = Bytes::from_slice(&mut a, &[0, 13, 10, 255, 0], 8)
        .unwrap()
        .encode(&mut a)
        .unwrap();
    let b_root = Bytes::from_slice(&mut b, &[0, 13, 10, 255, 0], 8)
        .unwrap()
        .encode(&mut b)
        .unwrap();
    assert_ne!(a_root, b_root);
    assert_eq!(a.digest(a_root), b.digest(b_root));
    assert_eq!(display(&a, a_root, &mut 100), display(&b, b_root, &mut 100));
}

#[test]
fn checked_in_vectors_match_the_pinned_native_identity() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("audit/self-hosting/native-data-vectors.json");
    assert_eq!(
        super::vectors().unwrap(),
        std::fs::read_to_string(path).unwrap()
    );
}
