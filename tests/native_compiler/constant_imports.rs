use super::{
    codegen,
    support::{self, data, schema},
};
use nox::{artifact, Order, Reduction};
use std::sync::OnceLock;
use trident::NATIVE_ARTIFACT_LIMITS as LIMITS;

type Arena = Reduction<{ 1 << 20 }>;
struct Module {
    name: String,
    source: String,
    // name start/end, type, value, literal owner/start/end.
    bindings: Vec<[u64; 7]>,
}

fn words(arena: &mut Arena, values: &[u64]) -> Order {
    let mut tail = schema::atom(arena, *values.last().unwrap()).unwrap();
    for &value in values[..values.len() - 1].iter().rev() {
        let value = schema::atom(arena, value).unwrap();
        tail = schema::pair(arena, value, tail).unwrap();
    }
    tail
}

fn query(modules: &[Module], imports: &[u64], queries: &[String]) -> Vec<Option<[u64; 7]>> {
    static FIXTURE: OnceLock<Vec<u8>> = OnceLock::new();
    let fixture = FIXTURE.get_or_init(|| {
        trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_constant_imports.tri"),
            &Default::default(),
            trident::NativeArtifactProfile::RawNoun,
            LIMITS,
        )
        .unwrap_or_else(|errors| panic!("fixture: {errors:?}"))
        .bytes
    });
    let output = codegen::generate_with_input::<{ 1 << 20 }>(fixture, |arena| {
        let modules: Vec<_> = modules
            .iter()
            .map(|module| {
                let name = schema::bytes(arena, module.name.as_bytes()).unwrap();
                let source = schema::bytes(arena, module.source.as_bytes()).unwrap();
                let bindings: Vec<_> = module
                    .bindings
                    .iter()
                    .map(|binding| words(arena, binding))
                    .collect();
                let bindings = schema::seq(arena, &bindings).unwrap();
                let body = schema::pair(arena, source, bindings).unwrap();
                schema::pair(arena, name, body).unwrap()
            })
            .collect();
        let modules = schema::seq(arena, &modules).unwrap();
        let imports: Vec<_> = imports
            .iter()
            .map(|&i| schema::atom(arena, i).unwrap())
            .collect();
        let imports = schema::seq(arena, &imports).unwrap();
        let queries: Vec<_> = queries
            .iter()
            .map(|q| schema::bytes(arena, q.as_bytes()).unwrap())
            .collect();
        let queries = schema::seq(arena, &queries).unwrap();
        let body = schema::pair(arena, imports, queries).unwrap();
        schema::pair(arena, modules, body).unwrap()
    })
    .unwrap();
    let mut arena = Arena::try_new_boxed().unwrap();
    let root = artifact::decode(&mut arena, &output, LIMITS).unwrap();
    let values = data::Seq::decode(&mut arena, root, 4096, 1000000).unwrap();
    (0..values.len())
        .map(|i| {
            let mut value = values.get(&arena, i).unwrap();
            if arena.atom_value(value).is_some() {
                assert_eq!(data::value(&arena, value).unwrap(), 0);
                return None;
            }
            let mut result = [0; 7];
            for slot in &mut result[..6] {
                *slot = data::value(&arena, arena.head(value).unwrap()).unwrap();
                value = arena.tail(value).unwrap();
            }
            result[6] = data::value(&arena, value).unwrap();
            Some(result)
        })
        .collect()
}

#[test]
fn imported_constants_bind_each_symbol_in_direct_use_order_with_owner_spans() {
    support::worker(|| {
        // Distinct offsets catch comparisons that accidentally read names from
        // the caller, a sibling, or the terminal literal's original source.
        let modules = [
            Module {
                name: "alpha.same".into(),
                source: "padding A B 11 4294967295".into(),
                bindings: vec![[8, 9, 0, 11, 0, 12, 14], [10, 11, 3, 4294967295, 0, 15, 25]],
            },
            Module {
                name: "beta.same".into(),
                source: "A C 22 33".into(),
                bindings: vec![[0, 1, 0, 22, 1, 4, 6], [2, 3, 0, 33, 1, 7, 9]],
            },
            Module {
                name: "hidden.same".into(),
                source: "A 99".into(),
                bindings: vec![[0, 1, 0, 99, 2, 2, 4]],
            },
        ];
        let queries: Vec<_> = [
            "same.A",
            "same.B",
            "same.C",
            "alpha.same.A",
            "beta.same.A",
            "hidden.same.A",
            "A",
            "same.MISSING",
            "// ж😀\n same // path\n . B",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let expected = vec![
            Some([3, 1, 0, 22, 1, 4, 6]),
            Some([2, 0, 3, 4294967295, 0, 15, 25]),
            Some([4, 1, 0, 33, 1, 7, 9]),
            Some([1, 0, 0, 11, 0, 12, 14]),
            Some([3, 1, 0, 22, 1, 4, 6]),
            None,
            None,
            None,
            Some([2, 0, 3, 4294967295, 0, 15, 25]),
        ];
        assert_eq!(query(&modules, &[0, 1], &queries), expected);
        let mut repeated = expected;
        repeated[0] = Some([1, 0, 0, 11, 0, 12, 14]);
        assert_eq!(query(&modules, &[0, 1, 0], &queries), repeated);
        assert!(query(&modules, &[], &queries)
            .into_iter()
            .all(|v| v.is_none()));
    });
}

#[test]
fn exported_alias_values_keep_foreign_literal_origin_and_complete_member_names() {
    support::worker(|| {
        let long = "X".repeat(300);
        let modules = [
            Module {
                name: "original".into(),
                source: "N 18446744069414584321".into(),
                bindings: vec![[0, 1, 0, 0, 0, 2, 22]],
            },
            Module {
                name: "middle".into(),
                source: format!("{long} ALIAS"),
                bindings: vec![[0, 300, 0, 0, 0, 2, 22], [301, 306, 0, 0, 0, 2, 22]],
            },
        ];
        let queries = vec![
            format!("middle.{long}"),
            "middle.ALIAS".into(),
            "original.N".into(),
            format!("middle.{}Y", "X".repeat(299)),
        ];
        assert_eq!(
            query(&modules, &[1], &queries),
            vec![
                Some([2, 1, 0, 0, 0, 2, 22]),
                Some([3, 1, 0, 0, 0, 2, 22]),
                None,
                None
            ]
        );
        let owner = "a.".repeat(127) + "b";
        let modules = [Module {
            name: owner.clone(),
            source: "X 7".into(),
            bindings: vec![[0, 1, 0, 7, 0, 2, 3]],
        }];
        assert_eq!(owner.len(), 255);
        let queries = vec![
            format!("{owner}.X"),
            "b.X".into(),
            "c.X".into(),
            "a.b.X".into(),
        ];
        assert_eq!(
            query(&modules, &[0], &queries),
            vec![
                Some([1, 0, 0, 7, 0, 2, 3]),
                Some([1, 0, 0, 7, 0, 2, 3]),
                None,
                None
            ]
        );
    });
}
