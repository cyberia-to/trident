//! Execute the guest ABI registry on nox against the seed declaration contract.
#[allow(dead_code)]
#[path = "../examples/selfhost_data/model.rs"]
mod model;
#[path = "native_control/support.rs"]
mod support;
use model::{atom, pair, Bytes, Seq};
use nox::{artifact, sequential, NoTrace, Order, Outcome, Reduction};
use std::sync::OnceLock;
use trident::{CompileOptions, NativeArtifactProfile, NATIVE_ARTIFACT_LIMITS as LIMITS};

// Component allowance includes admitting a full 4096-byte source. Production
// JOB quotas are independent and remain unchanged by this registry fixture.
type Arena = Reduction<{ 1 << 20 }>;
const NODES: u32 = 786432;
// Primitive tags are the canonical descriptor ABI. Return tag 16 below means
// the complete two-U32 tuple, constructed independently in tuple().
const ABIS: &[(&str, u64, &[u64], u64)] = &[
    ("as_u32", 1, &[0], 3),
    ("as_field", 2, &[3], 0),
    ("sub", 3, &[0, 0], 0),
    ("nox_noun_atom", 4, &[0], 4),
    ("nox_noun_pair", 5, &[4, 4], 4),
    ("nox_noun_head", 6, &[4], 4),
    ("nox_noun_tail", 7, &[4], 4),
    ("nox_noun_as_field", 8, &[4], 0),
    ("nox_noun_eq", 9, &[4, 4], 1),
    ("nox_noun_identity", 10, &[4], 6),
    ("assert", 11, &[1], 2),
    ("assert_eq", 12, &[0, 0], 2),
    ("field_add", 13, &[0, 0], 0),
    ("field_mul", 14, &[0, 0], 0),
    ("neg", 15, &[0], 0),
    ("inv", 16, &[0], 0),
    ("split", 17, &[0], 16),
];

fn code() -> &'static [u8] {
    static CODE: OnceLock<Vec<u8>> = OnceLock::new();
    CODE.get_or_init(|| {
        trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_intrinsic_abi.tri"),
            &CompileOptions::default(),
            NativeArtifactProfile::RawNoun,
            LIMITS,
        )
        .unwrap()
        .bytes
    })
}

fn run(mode: u64, input: impl FnOnce(&mut Arena) -> Order) -> (Box<Arena>, Order) {
    try_run(mode, input).unwrap()
}

fn try_run(
    mode: u64,
    input: impl FnOnce(&mut Arena) -> Order,
) -> Result<(Box<Arena>, Order), String> {
    let mut arena = Box::new(Arena::new());
    assert!(arena.limit_allocations(NODES));
    let artifact = artifact::decode(&mut arena, code(), LIMITS).unwrap();
    let mut body = arena.tail(artifact).unwrap();
    for _ in 0..3 {
        body = arena.tail(body).unwrap();
    }
    let formula = arena.head(body).unwrap();
    let payload = input(&mut arena);
    let source_bytes = if mode == 0 {
        let encoded = arena.head(payload).unwrap();
        let length_root = arena.tail(encoded).unwrap();
        model::value(&arena, arena.head(length_root).unwrap()).unwrap()
    } else {
        0
    };
    let mode = atom(&mut arena, mode).unwrap();
    let input = pair(&mut arena, mode, payload).unwrap();
    let execution = sequential::reduce(
        &mut arena,
        input,
        formula,
        100_000_000,
        sequential::Limits { max_frames: 65536 },
        &mut NoTrace,
    )
    .map_err(|error| format!("{error:?}"))?;
    let result = match execution.outcome {
        Outcome::Ok(result, _) => result,
        other => {
            return Err(format!(
                "{other:?}; source_bytes={source_bytes}; nodes={}",
                arena.count()
            ))
        }
    };
    Ok((arena, result))
}

fn two(ar: &mut Arena, a: u64, b: u64) -> Order {
    let a = atom(ar, a).unwrap();
    let b = atom(ar, b).unwrap();
    pair(ar, a, b).unwrap()
}

fn bytes(ar: &mut Arena, value: &[u8]) -> Order {
    Bytes::from_slice(ar, value, 4096)
        .unwrap()
        .encode(ar)
        .unwrap()
}

fn tuple(ar: &mut Arena, children: &[u64], depth: u64, nodes: u64, flag: u64) -> Order {
    let children: Vec<_> = children.iter().map(|&tag| atom(ar, tag).unwrap()).collect();
    let children = Seq::from_values(ar, &children, 16)
        .unwrap()
        .encode(ar)
        .unwrap();
    let extent = two(ar, nodes, flag);
    let depth = atom(ar, depth).unwrap();
    let metadata = pair(ar, depth, extent).unwrap();
    let data = pair(ar, children, metadata).unwrap();
    let tag = atom(ar, 16).unwrap();
    pair(ar, tag, data).unwrap()
}

fn return_type(ar: &mut Arena, tag: u64) -> Order {
    if tag == 16 {
        tuple(ar, &[3, 3], 1, 3, 0)
    } else {
        atom(ar, tag).unwrap()
    }
}

fn lookup(source: &[u8], start: u64, end: u64) -> (u64, u64) {
    let (ar, result) = run(0, |ar| {
        let source = bytes(ar, source);
        let span = two(ar, start, end);
        pair(ar, source, span).unwrap()
    });
    (
        model::value(&ar, ar.head(result).unwrap()).unwrap(),
        model::value(&ar, ar.tail(result).unwrap()).unwrap(),
    )
}

// The binding and Function wire shapes are independent test constructors.
fn signature(
    ar: &mut Arena,
    kind: u64,
    parameters: &[u64],
    result: Order,
    offset: u64,
    count: u64,
    name: u64,
) -> Order {
    let bindings: Vec<_> = parameters
        .iter()
        .enumerate()
        .map(|(i, &ty)| {
            let span = two(ar, name + i as u64, name + i as u64 + 1);
            let payload = two(ar, name % 2, ty);
            pair(ar, span, payload).unwrap()
        })
        .collect();
    let bindings = Seq::from_values(ar, &bindings, 4096)
        .unwrap()
        .encode(ar)
        .unwrap();
    let span = two(ar, name, name + 1);
    let range = two(ar, offset, count);
    let body = two(ar, 37, 91);
    let result = pair(ar, result, body).unwrap();
    let metadata = pair(ar, range, result).unwrap();
    let function = pair(ar, span, metadata).unwrap();
    let data = pair(ar, function, bindings).unwrap();
    let kind = atom(ar, kind).unwrap();
    pair(ar, kind, data).unwrap()
}

fn valid(kind: u64, params: &[u64], result: u64, offset: u64, count: u64, name: u64) -> bool {
    let (ar, result) = run(2, |ar| {
        let result = return_type(ar, result);
        signature(ar, kind, params, result, offset, count, name)
    });
    model::value(&ar, result).unwrap() == 1
}

#[test]
fn every_known_name_preserves_ids_and_every_signature_is_exact() {
    support::worker(|| {
        for &(name, kind, parameters, returned) in ABIS {
            assert_eq!(
                lookup(name.as_bytes(), 0, name.len() as u64),
                (kind, if kind <= 12 { kind } else { 0 }),
                "{name}"
            );
            let (mut ar, result) = run(1, |ar| atom(ar, kind).unwrap());
            let return_ty = return_type(&mut ar, returned);
            let mut expected = return_ty;
            for index in (0..3).rev() {
                let tag = parameters.get(index).copied().unwrap_or(5);
                let ty = atom(&mut ar, tag).unwrap();
                expected = pair(&mut ar, ty, expected).unwrap();
            }
            let lowerable = atom(&mut ar, u64::from(kind <= 12)).unwrap();
            expected = pair(&mut ar, lowerable, expected).unwrap();
            let arity = atom(&mut ar, parameters.len() as u64).unwrap();
            expected = pair(&mut ar, arity, expected).unwrap();
            assert_eq!(ar.digest(result), ar.digest(expected), "{name}");
            assert!(valid(
                kind,
                parameters,
                returned,
                0,
                parameters.len() as u64,
                1
            ));
        }
    });
}

#[test]
fn the_actual_compiler_closure_declares_only_registered_intrinsic_names() {
    support::worker(|| {
        let mut declared = std::collections::BTreeSet::new();
        for source in [
            include_str!("../lib/vm/core/convert.tri"),
            include_str!("../lib/vm/core/field.tri"),
            include_str!("../lib/vm/nox/noun.tri"),
        ] {
            for line in source.lines() {
                if let Some(name) = line.trim().strip_prefix("#[intrinsic(") {
                    let name = name.strip_suffix(")]").unwrap();
                    let expected = ABIS.iter().find(|abi| abi.0 == name).unwrap();
                    assert_eq!(lookup(name.as_bytes(), 0, name.len() as u64).0, expected.1);
                    assert!(declared.insert(name));
                }
            }
        }
        let expected: std::collections::BTreeSet<_> = ABIS
            .iter()
            .filter(|abi| abi.1 != 11 && abi.1 != 12)
            .map(|abi| abi.0)
            .collect();
        assert_eq!(declared, expected);
    });
}

#[test]
fn lookup_uses_the_complete_bounded_span_and_leaves_unknown_abis_unknown() {
    support::worker(|| {
        for &(name, kind, _, _) in ABIS {
            let source = format!("prefix {name} suffix");
            assert_eq!(lookup(source.as_bytes(), 7, 7 + name.len() as u64).0, kind);
            for changed in [format!("{name}x"), format!("x{name}")] {
                assert_eq!(lookup(changed.as_bytes(), 0, changed.len() as u64).0, 0);
            }
        }
        for name in [
            "",
            "nox_noun_",
            "nox_noun_identitx",
            "field_div",
            "log2",
            "pow",
            "assert_digest",
            "splitx",
            "NEG",
        ] {
            assert_eq!(lookup(name.as_bytes(), 0, name.len() as u64).0, 0, "{name}");
        }
        // Exact long spellings reject a changed byte at every position, even
        // synthetic non-ASCII inputs that never arise from identifier lexing.
        for name in [b"field_add", b"field_mul", b"assert_eq"] {
            for i in 0..9 {
                let mut changed = *name;
                changed[i] = 255;
                assert_eq!(lookup(&changed, 0, 9).0, 0);
            }
        }
        let mut source = vec![b'x'; 4096];
        assert_eq!(lookup(&source, 0, 4096).0, 0);
        source[4087..].copy_from_slice(b"field_add");
        assert_eq!(lookup(&source, 4087, 4096).0, 13);
        assert_eq!(lookup(&source, 4096, 4096).0, 0);
        for kind in [0, 18, 19, u64::from(u32::MAX)] {
            let (ar, result) = run(1, |ar| atom(ar, kind).unwrap());
            let mut current = result;
            for expected in [0, 0, 5, 5, 5] {
                assert_eq!(
                    model::value(&ar, ar.head(current).unwrap()).unwrap(),
                    expected
                );
                current = ar.tail(current).unwrap();
            }
            assert_eq!(model::value(&ar, current).unwrap(), 5);
            assert!(!valid(kind, &[], 5, 0, 0, 0));
        }
    });
}

#[test]
fn declaration_validation_checks_all_parameter_positions_ranges_and_return_types() {
    support::worker(|| {
        for &(_, kind, params, returned) in ABIS {
            let count = params.len() as u64;
            assert!(!valid(kind, params, returned, 0, count - 1, 0));
            assert!(!valid(kind, params, returned, 0, count + 1, 0));
            assert!(!valid(
                kind,
                &params[..params.len() - 1],
                returned,
                0,
                count,
                0
            ));
            assert!(!valid(kind, params, 5, 0, count, 0));
            assert!(!valid(kind, params, returned, count, count, 0));
            assert!(!valid(
                kind,
                params,
                returned,
                u64::from(u32::MAX),
                count,
                0
            ));
            for i in 0..params.len() {
                let mut changed = params.to_vec();
                changed[i] = (changed[i] + 1) % 5;
                assert!(!valid(kind, &changed, returned, 0, count, 0));
            }
            let mut shifted = vec![5];
            shifted.extend_from_slice(params);
            shifted.push(5);
            assert!(valid(kind, &shifted, returned, 1, count, 3071));
            assert!(valid(kind, &shifted, returned, 1, count, 4080));
        }
        // Two-parameter intrinsic ABIs are homogeneous. Check both placements
        // of a foreign type so validating only the first/last cannot pass.
        assert!(!valid(3, &[0, 3], 0, 0, 2, 0));
        assert!(!valid(3, &[3, 0], 0, 0, 2, 0));
    });
}

#[test]
fn split_requires_the_canonical_two_u32_tuple_including_metadata() {
    support::worker(|| {
        assert!(valid(17, &[0], 16, 0, 1, 0));
        for (children, depth, nodes, flag) in [
            (&[3][..], 1, 2, 0),
            (&[3, 3, 3][..], 1, 4, 0),
            (&[0, 3][..], 1, 3, 0),
            (&[3, 0][..], 1, 3, 0),
            (&[3, 3][..], 0, 3, 0),
            (&[3, 3][..], 1, 2, 0),
            (&[3, 3][..], 1, 3, 1),
        ] {
            let (ar, result) = run(2, |ar| {
                let ty = tuple(ar, children, depth, nodes, flag);
                signature(ar, 17, &[0], ty, 0, 1, 0)
            });
            assert_eq!(model::value(&ar, result).unwrap(), 0);
        }
        for returned in [0, 3, 4, 5, 6] {
            assert!(!valid(17, &[0], returned, 0, 1, 0));
        }
    });
}

#[test]
fn namespace_admission_matches_seed_prefixes_and_interior_ext_segments_exactly() {
    support::worker(|| {
        for owner in [
            &format!("{}.ext.", "x".repeat(4091)),
            &format!("{}ext.", "x".repeat(4092)),
            &format!("{}.extx", "x".repeat(4091)),
            "ééé.ext.foo",
            "std.compiler.nox",
            "vm.core.field",
            "os.joy",
            "ext.demo",
            "a.ext.foo",
            "a.b.ext.c",
            "std.",
            "vm.",
            "os.",
            "ext.",
            ".ext.",
            "a.ext.",
            "",
            "std",
            "vm",
            "os",
            "ext",
            "avm.foo",
            "xstd.foo",
            "xos.foo",
            "a.ext",
            "a.extx.foo",
            "a.next.foo",
            "a.ext_foo",
            "a.EXt.foo",
            "vm_foo",
            "stdx.foo",
        ] {
            let expected = owner.starts_with("std.")
                || owner.starts_with("vm.")
                || owner.starts_with("os.")
                || owner.starts_with("ext.")
                || owner.contains(".ext.");
            let (ar, result) = run(3, |ar| bytes(ar, owner.as_bytes()));
            assert_eq!(
                model::value(&ar, result).unwrap(),
                u64::from(expected),
                "{owner}"
            );
        }
    });
}

fn bank_input(ar: &mut Arena, entries: &[(u64, u64)], queries: &[u64], cap: u64) -> Order {
    let entries: Vec<_> = entries
        .iter()
        .map(|&(id, kind)| two(ar, id, kind))
        .collect();
    let entries = Seq::from_values(ar, &entries, 4096)
        .unwrap()
        .encode(ar)
        .unwrap();
    let queries: Vec<_> = queries.iter().map(|&id| atom(ar, id).unwrap()).collect();
    let queries = Seq::from_values(ar, &queries, 4096)
        .unwrap()
        .encode(ar)
        .unwrap();
    let cap = atom(ar, cap).unwrap();
    let data = pair(ar, queries, cap).unwrap();
    pair(ar, entries, data).unwrap()
}

#[test]
fn sparse_intrinsic_bindings_follow_exact_ids_and_leave_ordinary_replacements_absent() {
    support::worker(|| {
        let entries = [(0, 11), (2, 4), (7, 13), (4095, 17)];
        // Declaration 3 is an ordinary replacement of intrinsic declaration 2.
        let queries = [0, 1, 2, 3, 6, 7, 8, 4094, 4095];
        for entries in [&[][..], &entries[..]] {
            let (mut ar, result) = run(4, |ar| bank_input(ar, entries, &queries, 4096));
            let expected: Vec<_> = queries
                .iter()
                .map(|id| {
                    let kind = entries
                        .iter()
                        .find(|entry| entry.0 == *id)
                        .map_or(0, |entry| entry.1);
                    atom(&mut ar, kind).unwrap()
                })
                .collect();
            let expected = Seq::from_values(&mut ar, &expected, 4096)
                .unwrap()
                .encode(&mut ar)
                .unwrap();
            assert_eq!(ar.digest(result), ar.digest(expected));
        }
    });
}

#[test]
fn intrinsic_bindings_reject_duplicate_unsorted_out_of_range_and_over_cap_appends() {
    support::worker(|| {
        for (entries, cap) in [
            (&[(0, 1), (0, 2)][..], 4096),
            (&[(2, 1), (1, 2)][..], 4096),
            (&[(0, 0)][..], 4096),
            (&[(0, 18)][..], 4096),
            (&[(0, u64::from(u32::MAX))][..], 4096),
            (&[(4096, 1)][..], 4096),
            (&[(u64::from(u32::MAX), 1)][..], 4096),
            (&[(0, 1)][..], 0),
            (&[(0, 1), (1, 2)][..], 1),
        ] {
            let Err(error) = try_run(4, |ar| bank_input(ar, entries, &[0], cap)) else {
                panic!("accepted entries={entries:?}, cap={cap}");
            };
            assert!(error.contains("InvZero"), "{error}");
        }
    });
}
