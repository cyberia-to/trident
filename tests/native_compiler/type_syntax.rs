use super::{
    nouns::data::{Noun, Noun::Atom},
    support,
    types::tuple,
};
use std::sync::OnceLock;

fn parse(text: &str, cap: u64) -> (u64, Vec<u8>) {
    static PROBE: OnceLock<Vec<u8>> = OnceLock::new();
    let code = PROBE.get_or_init(|| {
        trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_type_parse.tri"),
            &trident::CompileOptions::default(),
            trident::NativeArtifactProfile::RawNoun,
            trident::NATIVE_ARTIFACT_LIMITS,
        )
        .unwrap()
        .bytes
    });
    let mut arena = nox::Reduction::<{ 1 << 20 }>::try_new_boxed().unwrap();
    assert!(arena.limit_allocations(786432));
    let artifact =
        nox::artifact::decode(&mut arena, code, trident::NATIVE_ARTIFACT_LIMITS).unwrap();
    let mut fields = arena.tail(artifact).unwrap();
    for _ in 0..3 {
        fields = arena.tail(fields).unwrap();
    }
    let formula = arena.head(fields).unwrap();
    let source = support::schema::bytes(&mut arena, text.as_bytes()).unwrap();
    let cap = support::data::atom(&mut arena, cap).unwrap();
    let input = support::data::pair(&mut arena, source, cap).unwrap();
    let run = nox::sequential::reduce(
        &mut arena,
        input,
        formula,
        100_000_000,
        nox::sequential::Limits { max_frames: 65536 },
        &mut nox::NoTrace,
    )
    .unwrap();
    let root = match run.outcome {
        nox::Outcome::Ok(root, _) => root,
        other => panic!("{other:?}"),
    };
    let error = arena
        .atom_value(arena.head(root).unwrap())
        .unwrap()
        .as_u64();
    (
        error,
        nox::artifact::encode(
            &arena,
            arena.tail(root).unwrap(),
            trident::NATIVE_ARTIFACT_LIMITS,
        )
        .unwrap(),
    )
}
fn expected(ty: Noun, kind: u64, start: u64, end: u64) -> Vec<u8> {
    Noun::pair(
        Noun::pair(Atom(kind), Noun::pair(Atom(start), Atom(end))),
        ty,
    )
    .encoded()
}

#[test]
fn iterative_type_parser_keeps_ordered_descriptors_and_first_unread_token_spans() {
    support::worker(|| {
        for (text, ty, kind, start, end) in [
            ("Field", Atom(0), 0, 5, 5),
            ("Digest next", Atom(6), 1, 7, 11),
            ("(Field),", tuple(&[Atom(0)], 1, 2, 0), 26, 7, 8),
            (
                "(Field,(Bool,Noun)) next",
                tuple(&[Atom(0), tuple(&[Atom(1), Atom(4)], 1, 3, 1)], 2, 5, 1),
                1,
                20,
                24,
            ),
        ] {
            let (error, actual) = parse(text, 4096);
            assert_eq!(error, 0, "{text}");
            assert_eq!(actual, expected(ty, kind, start, end), "{text}");
        }
        for (text, code) in [
            ("", 2),
            ("()", 2),
            ("(Field,)", 2),
            ("(Field,,Bool)", 2),
            ("(Field Bool)", 2),
            ("(Field $)", 1),
            ("Unit", 5),
            ("(Field,[Field;2])", 6),
        ] {
            assert_eq!(parse(text, 4096).0, code, "{text}");
        }
        let source = "program sample fn f(x:(Field $))->Field{7} fn main()->Field{0}";
        match support::compile_only(source.as_bytes(), super::nouns::data::caps()) {
            support::Compilation::Errors(errors) => {
                assert_eq!(errors.len(), 1);
                assert_eq!(errors[0].code, 1);
                assert_eq!(
                    &source[errors[0].start as usize..errors[0].end as usize],
                    "$"
                );
            }
            other => panic!("{other:?}"),
        }
    });
}

#[test]
fn source_type_nesting_arity_and_logical_node_allowances_have_exact_boundaries() {
    support::worker(|| {
        let source = format!("{}Field{}", "(".repeat(64), ")".repeat(64));
        let mut ty = Atom(0);
        for depth in 1..=64 {
            ty = tuple(&[ty], depth, depth + 1, 0);
        }
        assert_eq!(parse(&source, 64).0, 7);
        for cap in [65, 66] {
            assert_eq!(parse(&source, cap), (0, expected(ty.clone(), 0, 133, 133)));
        }
        assert_eq!(parse(&format!("({source})"), 4096).0, 7);
        let source = format!("({})", vec!["Field"; 16].join(","));
        assert_eq!(parse(&source, 16).0, 7);
        for cap in [17, 18] {
            assert_eq!(
                parse(&source, cap),
                (
                    0,
                    expected(
                        tuple(&vec![Atom(0); 16], 1, 17, 0),
                        0,
                        source.len() as u64,
                        source.len() as u64
                    )
                )
            );
        }
        assert_eq!(
            parse(&format!("({})", vec!["Field"; 17].join(",")), 4096).0,
            7
        );
        assert_eq!(parse("Field", 0).0, 7);
        assert_eq!(parse("Field", 1).0, 0);
    });
}
