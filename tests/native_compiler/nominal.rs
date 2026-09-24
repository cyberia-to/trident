use super::{nouns::data, support, types::tuple};
use data::{Noun, Noun::Atom};
use std::sync::OnceLock;

fn pair(a: Noun, b: Noun) -> Noun {
    Noun::pair(a, b)
}
fn tree(values: &[Noun]) -> Noun {
    if values.is_empty() {
        return Atom(0);
    }
    let mut padded = values.to_vec();
    padded.resize(values.len().next_power_of_two(), Atom(0));
    while padded.len() > 1 {
        padded = padded
            .chunks_exact(2)
            .map(|v| pair(v[0].clone(), v[1].clone()))
            .collect();
    }
    padded.remove(0)
}
fn seq(values: &[Noun]) -> Noun {
    pair(
        Atom(1397051697),
        pair(Atom(values.len() as u64), tree(values)),
    )
}
fn bytes(s: &str) -> Noun {
    let words: Vec<_> = s
        .as_bytes()
        .chunks(4)
        .map(|chunk| {
            Atom(
                chunk
                    .iter()
                    .enumerate()
                    .map(|(i, b)| (*b as u64) << (8 * i))
                    .sum(),
            )
        })
        .collect();
    pair(Atom(1113150513), pair(Atom(s.len() as u64), tree(&words)))
}
fn member(name: &str, ty: Noun, public: bool) -> Noun {
    pair(bytes(name), pair(ty, Atom(u64::from(public))))
}
fn descriptor(owner: &str, name: &str, fields: &[Noun], depth: u64, nodes: u64, flag: u64) -> Noun {
    pair(
        Atom(17),
        pair(
            pair(bytes(owner), pair(bytes(name), seq(fields))),
            pair(Atom(depth), pair(Atom(nodes), Atom(flag))),
        ),
    )
}
fn result(status: u64, index: u64, ty: Noun) -> Noun {
    pair(Atom(status), pair(Atom(index), ty))
}
fn failure(status: u64, index: u64) -> Vec<u8> {
    result(status, index, Atom(5)).encoded()
}
fn compile(path: &str) -> Vec<u8> {
    trident::compile_native_artifact_project(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path),
        &trident::CompileOptions::default(),
        trident::NativeArtifactProfile::RawNoun,
        trident::NATIVE_ARTIFACT_LIMITS,
    )
    .unwrap()
    .bytes
}
fn probe(
    mode: u64,
    owner: &str,
    name: &str,
    fields: &[Noun],
    nodes: u64,
    depth: u64,
    extra: Noun,
) -> Vec<u8> {
    static PROBE: OnceLock<Vec<u8>> = OnceLock::new();
    let program = PROBE.get_or_init(|| compile("tests/fixtures/native_nominal.tri"));
    let input = pair(
        Atom(mode),
        pair(
            bytes(owner),
            pair(
                bytes(name),
                pair(seq(fields), pair(pair(Atom(nodes), Atom(depth)), extra)),
            ),
        ),
    );
    data::run_traced(
        program,
        &input,
        50_000_000,
        65536,
        196608,
        &mut nox::NoTrace,
    )
    .unwrap()
    .bytes
}
fn build(owner: &str, name: &str, fields: &[Noun], nodes: u64, depth: u64) -> Vec<u8> {
    probe(0, owner, name, fields, nodes, depth, Atom(0))
}

#[test]
fn nominal_identity_uses_complete_owner_name_order_visibility_and_child_types() {
    support::worker(|| {
        let fields = [
            member("common_prefix_first", Atom(0), true),
            member("common_prefix_second", Atom(4), false),
        ];
        let mut identities = Vec::new();
        for (owner, name, fields) in [
            ("compiler.alpha", "Record", fields.to_vec()),
            ("compiler.alphb", "Record", fields.to_vec()),
            ("compiler.alpha", "RecordOther", fields.to_vec()),
            (
                "compiler.alpha",
                "Record",
                fields.iter().rev().cloned().collect(),
            ),
            (
                "compiler.alpha",
                "Record",
                vec![
                    member("common_prefix_first", Atom(0), false),
                    fields[1].clone(),
                ],
            ),
            (
                "compiler.alpha",
                "Record",
                vec![
                    member("common_prefix_first", Atom(6), true),
                    fields[1].clone(),
                ],
            ),
        ] {
            let expected = descriptor(owner, name, &fields, 1, 3, 1);
            let actual = build(owner, name, &fields, 3, 1);
            assert_eq!(actual, result(0, 0, expected.clone()).encoded());
            assert_eq!(actual, build(owner, name, &fields, 4096, 64));
            assert!(!identities.contains(&actual));
            identities.push(actual);
            let inspection = pair(
                result(0, 0, expected),
                pair(
                    bytes(owner),
                    pair(
                        bytes(name),
                        pair(
                            seq(&fields),
                            pair(Atom(1), pair(Atom(3), pair(Atom(1), Atom(0)))),
                        ),
                    ),
                ),
            );
            assert_eq!(
                probe(1, owner, name, &fields, 3, 1, Atom(0)),
                inspection.encoded()
            );
        }
    });
}

#[test]
fn nominal_fields_resolve_full_names_and_private_visibility_by_defining_owner() {
    support::worker(|| {
        let fields = [
            member("common_prefix_first", Atom(0), true),
            member("common_prefix_second", Atom(4), false),
        ];
        for (owner, field, ty, index, exists, visible) in [
            ("compiler.alpha", "common_prefix_first", 0, 0, 1, 1),
            ("compiler.beta", "common_prefix_first", 0, 0, 1, 1),
            ("compiler.alpha", "common_prefix_second", 4, 1, 1, 1),
            ("compiler.alphb", "common_prefix_second", 4, 1, 1, 0),
            ("compiler.alpha", "common_prefix_seconx", 5, 0, 0, 0),
            ("compiler.alpha", "", 5, 0, 0, 0),
        ] {
            let expected = pair(
                Atom(ty),
                pair(Atom(index), pair(Atom(exists), Atom(visible))),
            );
            assert_eq!(
                probe(
                    2,
                    "compiler.alpha",
                    "Record",
                    &fields,
                    3,
                    1,
                    pair(bytes(owner), bytes(field))
                ),
                expected.encoded()
            );
        }
        let absent = compile("tests/fixtures/native_nominal_absent.tri");
        for ty in (0..7).map(Atom).chain([tuple(&[Atom(0)], 1, 2, 0)]) {
            assert_eq!(
                data::run(&absent, &ty).unwrap().bytes,
                pair(Atom(5), pair(Atom(0), pair(Atom(0), Atom(0)))).encoded()
            );
        }
    });
}

#[test]
fn nominal_empty_records_and_independent_field_node_and_depth_caps_are_exact() {
    support::worker(|| {
        let empty = result(0, 0, descriptor("m", "Empty", &[], 1, 1, 0)).encoded();
        assert_eq!(build("m", "Empty", &[], 1, 1), empty);
        assert_eq!(build("m", "Empty", &[], 0, 1), failure(7, 0));
        assert_eq!(build("m", "Empty", &[], 1, 0), failure(7, 0));
        let fields: Vec<_> = (0..32)
            .map(|i| member(&format!("f{i}"), Atom(0), i % 2 == 0))
            .collect();
        assert_eq!(
            build("m", "Wide", &fields, 33, 1),
            result(0, 0, descriptor("m", "Wide", &fields, 1, 33, 0)).encoded()
        );
        assert_eq!(build("m", "Wide", &fields, 32, 1), failure(7, 31));
        let mut too_wide = fields;
        too_wide.push(member("f32", Atom(0), false));
        assert_eq!(build("m", "Wide", &too_wide, 4096, 64), failure(7, 0));
    });
}

#[test]
fn nominal_invalid_names_duplicate_fields_and_invalid_child_types_report_the_member() {
    support::worker(|| {
        assert_eq!(build("", "Record", &[], 4096, 64), failure(5, 0));
        assert_eq!(build("m", "", &[], 4096, 64), failure(5, 0));
        for bad in [
            member("", Atom(0), true),
            member("x", Atom(3), false),
            member("y", Atom(5), true),
        ] {
            assert_eq!(
                build("m", "Record", &[member("x", Atom(0), true), bad], 4096, 64),
                failure(5, 1)
            );
        }
    });
}

#[test]
fn nominal_nested_metadata_counts_shared_children_and_propagates_noun_recursively() {
    support::worker(|| {
        let child_fields = [member("value", tuple(&[Atom(2), Atom(4)], 1, 3, 1), false)];
        let child = descriptor("inner", "Child", &child_fields, 2, 4, 1);
        assert_eq!(
            build("inner", "Child", &child_fields, 4, 2),
            result(0, 0, child.clone()).encoded()
        );
        let fields = [
            member("left", child.clone(), true),
            member("right", child, false),
        ];
        let expected = result(0, 0, descriptor("outer", "Parent", &fields, 3, 9, 1)).encoded();
        assert_eq!(build("outer", "Parent", &fields, 9, 3), expected);
        let parent = descriptor("outer", "Parent", &fields, 3, 9, 1);
        assert_eq!(
            probe(4, "outer", "Parent", &fields, 9, 3, Atom(0)),
            pair(tuple(&[parent.clone(), Atom(6)], 4, 11, 1), parent).encoded()
        );
        assert_eq!(build("outer", "Parent", &fields, 8, 3), failure(7, 1));
        assert_eq!(build("outer", "Parent", &fields, 9, 2), failure(7, 0));
    });
}

#[test]
fn nominal_recursive_construction_rejects_first_excess_depth_or_logical_occurrence() {
    support::worker(|| {
        let mut expected = Atom(0);
        for depth in 1..=64 {
            expected = descriptor(
                "m",
                "S",
                &[member("S", expected, false)],
                depth,
                depth + 1,
                0,
            );
        }
        let nested = |count, nodes, depth, shared| {
            probe(
                3,
                "m",
                "S",
                &[],
                nodes,
                depth,
                pair(Atom(count), Atom(shared)),
            )
        };
        assert_eq!(nested(64, 65, 64, 0), result(0, 0, expected).encoded());
        assert_eq!(nested(64, 64, 64, 0), failure(7, 0));
        assert_eq!(nested(65, 4096, 64, 0), failure(7, 0));
        let mut expected = Atom(0);
        let mut nodes = 1;
        for depth in 1..=11 {
            nodes = 1 + 2 * nodes;
            expected = descriptor(
                "m",
                "S",
                &[
                    member("S", expected.clone(), false),
                    member("m", expected, true),
                ],
                depth,
                nodes,
                0,
            );
        }
        assert_eq!(nested(11, 4095, 64, 1), result(0, 0, expected).encoded());
        assert_eq!(nested(11, 4094, 64, 1), failure(7, 1));
        assert_eq!(nested(12, 4096, 64, 1), failure(7, 1));
    });
}
