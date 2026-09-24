use super::{nouns::data, support};
use data::{Noun, Noun::Atom};
use std::sync::OnceLock;

fn pair(a: Noun, b: Noun) -> Noun {
    Noun::pair(a, b)
}
fn probe(mode: u64, nodes: u64, depth: u64) -> Vec<u8> {
    static PROBE: OnceLock<Vec<u8>> = OnceLock::new();
    let bytes = PROBE.get_or_init(|| {
        trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_types.tri"),
            &trident::CompileOptions::default(),
            trident::NativeArtifactProfile::RawNoun,
            trident::NATIVE_ARTIFACT_LIMITS,
        )
        .unwrap()
        .bytes
    });
    data::run(bytes, &pair(Atom(mode), pair(Atom(nodes), Atom(depth))))
        .unwrap()
        .bytes
}
fn tuple(children: &[Noun], depth: u64, nodes: u64, contains: u64) -> Noun {
    let mut leaves = children.to_vec();
    leaves.resize(children.len().next_power_of_two(), Atom(0));
    while leaves.len() > 1 {
        leaves = leaves
            .chunks_exact(2)
            .map(|v| pair(v[0].clone(), v[1].clone()))
            .collect();
    }
    let sequence = pair(
        Atom(1397051697),
        pair(Atom(children.len() as u64), leaves.remove(0)),
    );
    pair(
        Atom(16),
        pair(
            sequence,
            pair(Atom(depth), pair(Atom(nodes), Atom(contains))),
        ),
    )
}

#[test]
fn descriptor_migration_retains_primitive_ast_binding_and_signature_bytes() {
    support::worker(|| {
        let span = pair(Atom(3), Atom(7));
        let node = pair(
            pair(Atom(1), pair(Atom(2), Atom(0))),
            pair(Atom(3), span.clone()),
        );
        let binding = pair(span.clone(), pair(Atom(1), Atom(3)));
        let signature = pair(
            span,
            pair(
                pair(Atom(0), Atom(1)),
                pair(Atom(3), pair(Atom(8), Atom(9))),
            ),
        );
        assert_eq!(
            probe(0, 4096, 64),
            pair(node, pair(binding, signature)).encoded()
        );
    });
}

#[test]
fn tuple_descriptors_are_canonical_and_count_shared_children_logically() {
    support::worker(|| {
        let mixed = tuple(&[Atom(0), Atom(2), Atom(4)], 1, 4, 1);
        assert_eq!(probe(1, 4, 1), mixed.encoded());
        assert_eq!(probe(1, 5, 2), mixed.encoded());
        assert_eq!(probe(1, 3, 1), Atom(5).encoded());
        assert_eq!(probe(1, 4, 0), Atom(5).encoded());
        let inner = tuple(&[Atom(0), Atom(3)], 1, 3, 0);
        let outer = tuple(&[inner.clone(), inner], 2, 7, 0);
        assert_eq!(probe(2, 7, 2), outer.encoded());
        assert_eq!(probe(2, 8, 3), outer.encoded());
        // The repeated subtree still contributes twice to the logical bound.
        assert_eq!(probe(2, 6, 2), Atom(5).encoded());
        assert_eq!(probe(2, 7, 1), Atom(5).encoded());
    });
}

#[test]
fn tuple_descriptor_children_and_arity_reject_before_success() {
    support::worker(|| {
        for mode in [3, 5, 99] {
            assert_eq!(probe(mode, 4096, 64), Atom(5).encoded());
        }
        let wide = tuple(&vec![Atom(0); 16], 1, 17, 0);
        assert_eq!(probe(4, 17, 1), wide.encoded());
        assert_eq!(probe(4, 16, 1), Atom(5).encoded());
    });
}

#[test]
fn descriptor_nesting_and_logical_node_limits_have_separate_exact_boundaries() {
    support::worker(|| {
        let mut expected = Atom(0);
        for depth in 1..=64 {
            expected = tuple(&[expected], depth, depth + 1, 0);
        }
        assert_eq!(probe(6, 65, 64), expected.encoded());
        assert_eq!(probe(6, 64, 64), Atom(5).encoded());
        assert_eq!(probe(6, 65, 63), Atom(5).encoded());
    });
}

#[test]
fn nested_descriptors_round_trip_through_all_real_type_record_readers() {
    support::worker(|| {
        let mut prior = None;
        for (mode, children, flag) in [
            (10, vec![Atom(0), Atom(4)], 1),
            (11, vec![Atom(4), Atom(0)], 1),
            (12, vec![Atom(0), Atom(1)], 0),
        ] {
            let inner = tuple(&children, 1, 3, flag);
            let ty = tuple(&[inner, Atom(3)], 2, 5, flag);
            let expected = pair(
                Atom(flag),
                pair(ty.clone(), pair(ty.clone(), pair(ty.clone(), ty))),
            );
            let bytes = probe(mode, 4096, 64);
            assert_eq!(bytes, expected.encoded());
            if let Some(prior) = prior {
                assert_ne!(bytes, prior);
            }
            prior = Some(bytes);
        }
    });
}

#[test]
fn shared_descriptor_dags_cannot_evade_the_maximum_logical_node_allowance() {
    support::worker(|| {
        let mut expected = Atom(0);
        let mut nodes = 1;
        for depth in 1..=11 {
            nodes = 1 + 2 * nodes;
            expected = tuple(&[expected.clone(), expected], depth, nodes, 0);
        }
        assert_eq!(nodes, 4095);
        assert_eq!(probe(7, 4095, 64), expected.encoded());
        assert_eq!(probe(7, 4094, 64), Atom(5).encoded());
        expected = tuple(&[expected], 12, 4096, 0);
        assert_eq!(probe(8, 4096, 64), expected.encoded());
        assert_eq!(probe(8, 4095, 64), Atom(5).encoded());
        assert_eq!(probe(9, 4096, 64), Atom(5).encoded());
    });
}
