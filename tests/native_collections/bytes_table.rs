use super::{atom, compile, equal, model, pair, run, support, Arena, Bytes, LIMITS};
use nox::artifact;

#[test]
fn byte_tables_retain_validated_handles_after_the_shared_allowance_is_spent() {
    support::worker(|| {
        let program = compile(
            "let (value, left) = bytes.from_noun_budget(noun.head(input), as_u32(1000), as_u32(noun.as_field(noun.tail(input))))
             assert_eq(as_field(left), 0)
             let empty = bytes.table_empty()
             let first = bytes.table_push(empty, value, as_u32(3))
             let second = bytes.table_push(first, bytes.empty(), as_u32(3))
             let mut changed = value
             if as_u32(0) < bytes.len(value) { changed = bytes.set(value, as_u32(0), as_u32(255)) }
             let third = bytes.table_push(second, changed, as_u32(3))
             assert_eq(as_field(bytes.table_len(empty)), 0)
             assert_eq(as_field(bytes.table_len(first)), 1)
             assert_eq(as_field(bytes.table_len(second)), 2)
             assert_eq(as_field(bytes.table_len(third)), 3)
             for repeat in 0..17 {
                 assert(noun.eq(bytes.to_noun(bytes.table_get(third, as_u32(0))), bytes.to_noun(value)))
                 assert_eq(as_field(bytes.len(bytes.table_get(third, as_u32(1)))), 0)
                 assert(noun.eq(bytes.to_noun(bytes.table_get(third, as_u32(2))), bytes.to_noun(changed)))
             }
             bytes.to_noun(bytes.table_get(first, as_u32(0)))",
        );
        for length in [0, 1, 4, 5, 17, 65] {
            let mut arena = Arena::new();
            let content: Vec<_> = (0..length).map(|i| (i * 37) as u8).collect();
            let encoded = Bytes::from_slice(&mut arena, &content, 1000)
                .unwrap()
                .encode(&mut arena)
                .unwrap();
            let mut remaining = 10000;
            Bytes::decode_budget(&mut arena, encoded, 1000, &mut remaining).unwrap();
            let allowance = atom(&mut arena, u64::from(10000 - remaining)).unwrap();
            let input = pair(&mut arena, encoded, allowance).unwrap();
            let result = run(&mut arena, &program, input).unwrap();
            equal(&arena, result, encoded);
            // The returned wrapper remains independently decodable.
            let canonical = artifact::encode(&arena, result, LIMITS).unwrap();
            assert_eq!(
                canonical,
                artifact::encode(&arena, encoded, LIMITS).unwrap()
            );
        }
    });
}

#[test]
fn byte_table_reads_and_appends_enforce_their_bounds() {
    support::worker(|| {
        for expression in [
            "bytes.to_noun(bytes.table_get(bytes.table_empty(), as_u32(0)))",
            "let table = bytes.table_push(bytes.table_empty(), bytes.empty(), as_u32(0)) noun.atom(0)",
            "let table = bytes.table_push(bytes.table_empty(), bytes.empty(), as_u32(1)) bytes.to_noun(bytes.table_get(table, as_u32(1)))",
            "let table = bytes.table_push(bytes.table_empty(), bytes.empty(), as_u32(1)) let full = bytes.table_push(table, bytes.empty(), as_u32(1)) noun.atom(0)",
        ] {
            let mut arena = Arena::new();
            let input = atom(&mut arena, model::BYTES).unwrap();
            assert!(run(&mut arena, &compile(expression), input).is_err(), "{expression}");
        }
    });
}

#[test]
fn callers_cannot_forge_or_replace_a_byte_tables_validated_storage() {
    for body in [
        "let t = bytes.BytesTable { values: seq.empty() } noun.atom(0)",
        "let t = bytes.table_empty() seq.to_noun(t.values)",
        "let mut t = bytes.table_empty() t.values = seq.empty() noun.atom(0)",
    ] {
        let source = format!("program private_table use vm.nox.noun use std.nox.bytes use std.nox.seq fn main(input:Noun)->Noun {{ {body} }}");
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("main.tri");
        std::fs::write(&path, source).unwrap();
        let errors =
            trident::compile_raw_artifact_project(&path, &Default::default(), LIMITS).unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("private")),
            "{body}: {errors:?}"
        );
    }
}
