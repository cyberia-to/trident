//! Execute source collections on nox and compare against the independent model.
#[allow(dead_code)]
#[path = "../examples/selfhost_data/model.rs"]
mod model;
#[path = "native_control/support.rs"]
mod support;
use model::{atom, pair, Bytes, Seq};
use nox::{artifact, sequential, NoTrace, Order, Outcome, Reduction};
use trident::{CompileOptions, RAW_ARTIFACT_LIMITS as LIMITS};
type Arena = Reduction<{ support::ARENA }>;

fn compile(body: &str) -> Vec<u8> {
    compile_source(&format!("program collections\nuse vm.nox.noun\nuse vm.core.convert\nuse std.nox.tree\nuse std.nox.seq\nuse std.nox.bytes\nfn main(input: Noun) -> Noun {{ {body} }}"))
}

fn compile_source(source: &str) -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("main.tri");
    std::fs::write(&path, source).unwrap();
    trident::compile_raw_artifact_project(&path, &CompileOptions::default(), LIMITS)
        .unwrap()
        .bytes
}

fn wrap(ar: &mut Arena, tag: u64, length: u64, root: Order) -> Order {
    let tag = atom(ar, tag).unwrap();
    let length = atom(ar, length).unwrap();
    let body = pair(ar, length, root).unwrap();
    pair(ar, tag, body).unwrap()
}

fn empty_tree(ar: &mut Arena, height: usize) -> Order {
    let mut value = atom(ar, 0).unwrap();
    for _ in 0..height {
        value = pair(ar, value, value).unwrap();
    }
    value
}

#[test]
fn malformed_wrappers_tags_branches_and_padding_fail_admission() {
    support::worker(|| {
        let sequence = compile("seq.to_noun(seq.from_noun(input, as_u32(1000), as_u32(10000)))");
        let bytes = compile("bytes.to_noun(bytes.from_noun(input, as_u32(1000), as_u32(10000)))");
        let mut ar = Arena::new();
        let zero = atom(&mut ar, 0).unwrap();
        let one = atom(&mut ar, 1).unwrap();
        let pair_leaf = pair(&mut ar, one, zero).unwrap();
        let nonzero_padding = pair(&mut ar, one, one).unwrap();
        let bad_padding = pair(&mut ar, pair_leaf, nonzero_padding).unwrap();
        let wrong_length = pair(&mut ar, pair_leaf, zero).unwrap();
        let tag = atom(&mut ar, model::SEQ).unwrap();
        let invalid_length = pair(&mut ar, tag, wrong_length).unwrap();
        let cases = [
            zero,
            pair_leaf,
            invalid_length,
            wrap(&mut ar, model::BYTES, 1, zero),
            wrap(&mut ar, model::SEQ, 0, one),
            wrap(&mut ar, model::SEQ, 2, zero),
            wrap(&mut ar, model::SEQ, 3, bad_padding),
            wrap(&mut ar, model::SEQ, 1 << 32, zero),
            wrap(&mut ar, model::SEQ, 1001, zero),
        ];
        for input in cases {
            assert!(Seq::decode(&mut ar, input, 1000, 10000).is_err());
            assert!(run(&mut ar, &sequence, input).is_err());
        }
        // Height alone marks a leaf: a pair is valid as one Seq element.
        let valid = wrap(&mut ar, model::SEQ, 1, pair_leaf);
        let actual = run(&mut ar, &sequence, valid).unwrap();
        equal(&ar, actual, valid);
        for (length, word) in [(1, 256), (2, 65536), (3, 16777216), (4, 1 << 32)] {
            let word = atom(&mut ar, word).unwrap();
            let input = wrap(&mut ar, model::BYTES, length, word);
            assert!(Bytes::decode(&mut ar, input, 1000, 10000).is_err());
            assert!(run(&mut ar, &bytes, input).is_err());
        }
        for input in [
            valid,
            wrap(&mut ar, model::BYTES, 4, pair_leaf),
            wrap(&mut ar, model::BYTES, 0, one),
            wrap(&mut ar, model::BYTES, 1 << 32, zero),
            wrap(&mut ar, model::BYTES, 1001, zero),
        ] {
            assert!(Bytes::decode(&mut ar, input, 1000, 10000).is_err());
            assert!(run(&mut ar, &bytes, input).is_err());
        }
    });
}

#[test]
fn collection_operations_reject_bounds_byte_range_and_append_caps() {
    support::worker(|| {
        for body in [
            "seq.get(seq.empty(), as_u32(0))",
            "seq.to_noun(seq.set(seq.empty(),as_u32(0),input))",
            "seq.to_noun(seq.push(seq.empty(),input,as_u32(0)))",
            "noun.atom(convert.as_field(bytes.get(bytes.empty(),as_u32(0))))",
            "bytes.to_noun(bytes.set(bytes.empty(),as_u32(0),as_u32(0)))",
            "bytes.to_noun(bytes.push(bytes.empty(),as_u32(0),as_u32(0)))",
            "bytes.to_noun(bytes.push(bytes.empty(),as_u32(256),as_u32(1)))",
            "let b=bytes.push(bytes.empty(),as_u32(7),as_u32(1))\nbytes.to_noun(bytes.set(b,as_u32(0),as_u32(256)))",
            "let s=seq.push(seq.empty(),input,as_u32(1))\nseq.to_noun(seq.push(s,input,as_u32(1)))",
            "let b=bytes.push(bytes.empty(),as_u32(7),as_u32(1))\nbytes.to_noun(bytes.push(b,as_u32(8),as_u32(1)))",
        ] {
            let program = compile(body);
            let mut ar = Arena::new();
            let input = atom(&mut ar, 0).unwrap();
            assert!(run(&mut ar,&program,input).is_err(),"accepted {body}");
        }
    });
}

#[test]
fn raw_tree_geometry_and_updates_cover_height_32_without_wrapping() {
    support::worker(|| {
        let program = compile("let n=as_u32(4294967295)\nlet at=as_u32(4294967294)\nlet changed=tree.set(input,n,at,noun.atom(77))\nassert_eq(noun.as_field(tree.get(changed,n,at)),77)\nassert_eq(noun.as_field(tree.get(input,n,at)),0)\nassert_eq(noun.as_field(tree.get(changed,n,as_u32(4294967293))),0)\nlet g=tree.shape(n)\nassert_eq(g.capacity,4294967296)\nassert_eq(convert.as_field(g.height),32)\nchanged");
        let mut ar = Arena::new();
        let input = empty_tree(&mut ar, 32);
        let mut expected = atom(&mut ar, 77).unwrap();
        // Index MAX-1: final bit goes left, all preceding bits go right.
        let zero = atom(&mut ar, 0).unwrap();
        expected = pair(&mut ar, expected, zero).unwrap();
        for height in 1..32 {
            let left = empty_tree(&mut ar, height);
            expected = pair(&mut ar, left, expected).unwrap();
        }
        let actual = run(&mut ar, &program, input).unwrap();
        equal(&ar, actual, expected);

        let growth = compile("let n=as_u32(2147483648)\nlet grown=tree.push(input,n,noun.atom(7),as_u32(4294967295))\nassert_eq(noun.as_field(tree.get(grown,as_u32(2147483649),n)),7)\nassert_eq(noun.as_field(tree.get(grown,as_u32(2147483649),as_u32(2147483647))),0)\ngrown");
        let input31 = empty_tree(&mut ar, 31);
        let mut right = atom(&mut ar, 7).unwrap();
        for height in 0..31 {
            let padding = empty_tree(&mut ar, height);
            right = pair(&mut ar, right, padding).unwrap();
        }
        let expected = pair(&mut ar, input31, right).unwrap();
        let actual = run(&mut ar, &growth, input31).unwrap();
        equal(&ar, actual, expected);

        let overflow =
            compile("tree.push(input,as_u32(4294967295),noun.atom(1),as_u32(4294967295))");
        assert!(run(&mut ar, &overflow, input).is_err());
        // The fixture's known-valid shared zero tree was constructed directly.
        // A bounded decoder must still visit repeated occupied children.
        let short = compile("seq.to_noun(seq.from_noun(input,as_u32(4294967295),as_u32(100)))");
        let encoded = wrap(&mut ar, model::SEQ, u64::from(u32::MAX), input);
        assert!(run(&mut ar, &short, encoded).is_err());
    });
}

#[test]
fn byte_word_arithmetic_handles_the_last_u32_index() {
    support::worker(|| {
        // Test the module's exact source with a local probe over a known-valid
        // shared zero fixture. This does not claim to decode billions of bytes.
        let source = include_str!("../lib/std/nox/bytes.tri").replacen(
            "module std.nox.bytes",
            "program byte_boundary",
            1,
        );
        let source = format!("{source}\nfn main(input:Noun)->Noun {{ let b=Bytes {{ length:as_u32(4294967295),root:input }}\nlet changed=set(b,as_u32(4294967294),as_u32(255))\nassert_eq(convert.as_field(get(changed,as_u32(4294967294))),255)\nassert_eq(convert.as_field(get(changed,as_u32(4294967293))),0)\nassert_eq(convert.as_field(get(b,as_u32(4294967294))),0)\nto_noun(changed) }}");
        let program = compile_source(&source);
        let mut ar = Arena::new();
        let input = empty_tree(&mut ar, 30);
        let mut expected = atom(&mut ar, 255 << 16).unwrap();
        for height in 0..30 {
            let left = empty_tree(&mut ar, height);
            expected = pair(&mut ar, left, expected).unwrap();
        }
        let expected = wrap(&mut ar, model::BYTES, u64::from(u32::MAX), expected);
        let actual = run(&mut ar, &program, input).unwrap();
        equal(&ar, actual, expected);
    });
}

#[test]
fn only_the_library_can_construct_validated_collection_handles() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("main.tri");
    for (body, diagnostic) in [
        (
            "seq.to_noun(seq.Seq { length:as_u32(0),root:input })",
            "is private",
        ),
        (
            "bytes.to_noun(bytes.Bytes { length:as_u32(0),root:input })",
            "is private",
        ),
        ("seq.empty().root", "is private"),
        (
            "let mut b=bytes.empty()\nb.length=as_u32(5)\nbytes.to_noun(b)",
            "is private",
        ),
        (
            "noun.atom(convert.as_field(seq.len(Seq { length:as_u32(0),root:input })))",
            "expected std.nox.seq.Seq",
        ),
    ] {
        let source=format!("program collections\nuse std.nox.seq\nuse std.nox.bytes\nuse vm.nox.noun\nuse vm.core.convert\nstruct Seq {{ length:U32,root:Noun }}\nfn main(input:Noun)->Noun {{ {body} }}");
        std::fs::write(&path, source).unwrap();
        let errors =
            trident::compile_raw_artifact_project(&path, &CompileOptions::default(), LIMITS)
                .unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains(diagnostic)),
            "{body}: {errors:?}"
        );
    }
}

fn run(ar: &mut Arena, program: &[u8], input: Order) -> Result<Order, String> {
    let artifact = artifact::decode(ar, program, LIMITS).map_err(|e| format!("{e:?}"))?;
    let mut cursor = ar.tail(artifact).unwrap();
    for _ in 0..3 {
        cursor = ar.tail(cursor).unwrap();
    }
    let formula = ar.head(cursor).unwrap();
    let result = sequential::reduce(
        ar,
        input,
        formula,
        20_000_000,
        sequential::Limits { max_frames: 65536 },
        &mut NoTrace,
    )
    .map_err(|e| format!("{e:?}"))?;
    match result.outcome {
        Outcome::Ok(value, _) => Ok(value),
        other => Err(format!("{other:?}")),
    }
}

fn equal(ar: &Arena, actual: Order, expected: Order) {
    assert_eq!(ar.digest(actual), ar.digest(expected));
    assert_eq!(
        artifact::encode(ar, actual, LIMITS).unwrap(),
        artifact::encode(ar, expected, LIMITS).unwrap()
    );
}

#[test]
fn sequence_updates_keep_pair_leaves_padding_and_old_roots() {
    support::worker(|| {
        let program = compile("let mut s = seq.empty()\nfor i in 0..17 { s = seq.push(s, noun.pair(noun.atom(convert.as_field(i)), input), as_u32(17)) }\nlet old = s\ns = seq.set(s, as_u32(8), noun.pair(input, input))\nassert_eq(convert.as_field(seq.len(s)), 17)\nassert(noun.eq(seq.get(old, as_u32(8)), noun.pair(noun.atom(8), input)))\nseq.to_noun(s)");
        let mut ar = Arena::new();
        let input = atom(&mut ar, 99).unwrap();
        let values: Vec<_> = (0..17)
            .map(|i| {
                let v = atom(&mut ar, i).unwrap();
                pair(&mut ar, v, input).unwrap()
            })
            .collect();
        let original = Seq::from_values(&mut ar, &values, 17).unwrap();
        let replacement = pair(&mut ar, input, input).unwrap();
        let expected = original
            .set(&mut ar, 8, replacement)
            .unwrap()
            .encode(&mut ar)
            .unwrap();
        let actual = run(&mut ar, &program, input).unwrap();
        equal(&ar, actual, expected);
    });
}

#[test]
fn packed_bytes_preserve_exact_values_and_persistent_updates() {
    support::worker(|| {
        let program = compile("let mut b = bytes.empty()\nlet values = [as_u32(0),as_u32(13),as_u32(10),as_u32(255),as_u32(128),as_u32(65),as_u32(0),as_u32(254),as_u32(1)]\nfor i in 0..9 { b = bytes.push(b, values[i], as_u32(9)) }\nlet old = b\nfor i in 0..9 { assert_eq(convert.as_field(bytes.get(b,i)), convert.as_field(values[i])) }\nb = bytes.set(b,as_u32(3),as_u32(7))\nb = bytes.set(b,as_u32(8),as_u32(255))\nassert_eq(convert.as_field(bytes.get(old,as_u32(3))),255)\nassert_eq(convert.as_field(bytes.get(old,as_u32(8))),1)\nbytes.to_noun(b)");
        let mut ar = Arena::new();
        let input = atom(&mut ar, 0).unwrap();
        let expected = Bytes::from_slice(&mut ar, &[0, 13, 10, 7, 128, 65, 0, 254, 255], 9)
            .unwrap()
            .encode(&mut ar)
            .unwrap();
        let actual = run(&mut ar, &program, input).unwrap();
        equal(&ar, actual, expected);
    });
}

#[test]
fn sequence_validation_matches_the_visit_budget_across_chunks() {
    support::worker(|| {
        let program = compile("let s = seq.from_noun(noun.head(input), as_u32(1000), as_u32(noun.as_field(noun.tail(input))))\nseq.to_noun(s)");
        for length in [0, 1, 2, 3, 4, 5, 17, 65, 129] {
            let mut ar = Arena::new();
            let values: Vec<_> = (0..length)
                .map(|i| atom(&mut ar, if length == 129 { 0 } else { i }).unwrap())
                .collect();
            let encoded = Seq::from_values(&mut ar, &values, 1000)
                .unwrap()
                .encode(&mut ar)
                .unwrap();
            let mut left = 1000;
            Seq::decode_budget(&mut ar, encoded, 1000, &mut left).unwrap();
            let required = 1000 - left;
            let fixture = artifact::encode(&ar, encoded, LIMITS).unwrap();
            for budget in [0, required - 1, required, required + 1] {
                let mut ar = Arena::new();
                let encoded = artifact::decode(&mut ar, &fixture, LIMITS).unwrap();
                let allowance = atom(&mut ar, u64::from(budget)).unwrap();
                let input = pair(&mut ar, encoded, allowance).unwrap();
                let result = run(&mut ar, &program, input);
                if budget < required {
                    assert!(result.is_err(), "len={length},budget={budget}");
                } else {
                    equal(&ar, result.unwrap(), encoded);
                }
            }
        }
    });
}

#[test]
fn bytes_validation_charges_shape_and_word_reads_to_one_allowance() {
    support::worker(|| {
        let program = compile("let b = bytes.from_noun(noun.head(input), as_u32(1000), as_u32(noun.as_field(noun.tail(input))))\nbytes.to_noun(b)");
        for length in [0, 1, 2, 3, 4, 5, 8, 9, 17, 65, 257, 513, 517] {
            let mut ar = Arena::new();
            let bytes: Vec<_> = (0..length).map(|i| (i * 37) as u8).collect();
            let encoded = Bytes::from_slice(&mut ar, &bytes, 1000)
                .unwrap()
                .encode(&mut ar)
                .unwrap();
            let mut left = 10000;
            Bytes::decode_budget(&mut ar, encoded, 1000, &mut left).unwrap();
            let required = 10000 - left;
            let fixture = artifact::encode(&ar, encoded, LIMITS).unwrap();
            for budget in [0, required - 1, required, required + 1] {
                let mut ar = Arena::new();
                let encoded = artifact::decode(&mut ar, &fixture, LIMITS).unwrap();
                let allowance = atom(&mut ar, u64::from(budget)).unwrap();
                let input = pair(&mut ar, encoded, allowance).unwrap();
                let result = run(&mut ar, &program, input);
                if budget < required {
                    assert!(result.is_err(), "len={length},budget={budget}");
                } else {
                    equal(&ar, result.unwrap(), encoded);
                }
            }
        }
    });
}
