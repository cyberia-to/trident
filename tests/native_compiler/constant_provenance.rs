use super::{
    codegen,
    support::{self, data, schema},
};
use nox::{artifact, Order, Reduction};
use std::sync::OnceLock;
use trident::NATIVE_ARTIFACT_LIMITS as LIMITS;

type Arena = Reduction<{ 1 << 20 }>;

struct Module<'a> {
    owner: &'a str,
    body: &'a str,
    imports: &'a [u64],
}

const FIELD_LITERAL: &str = "18446744069414584321";
const U32_LITERAL: &str = "0004294967295";
const ORIGINAL: &str = "// original ж\nconst RAW:Field=18446744069414584321\nconst UINT:U32=0004294967295\npub const SourceField:Field=LocalField\nconst LocalField:Field=RAW\npub const SourceUint:U32=UINT";
const BRIDGE: &str = "// different declaration offsets\nconst CachedField:Field=origin.SourceField\npub const ExportField:Field=CachedField\nconst CachedUint:U32=alpha.origin.SourceUint\npub const ExportUint:U32=CachedUint";
const CONSUMER: &str = "// final 😀\npub const FinalField:Field=bridge.ExportField\npub const FinalUint:U32=beta.bridge.ExportUint\nconst PrivateAlias:Field=FinalField";

fn modules<'a>(bridge: &'a str) -> [Module<'a>; 3] {
    [
        Module {
            owner: "alpha.origin",
            body: ORIGINAL,
            imports: &[],
        },
        Module {
            owner: "beta.bridge",
            body: bridge,
            imports: &[0],
        },
        Module {
            owner: "gamma.consumer",
            body: CONSUMER,
            imports: &[1],
        },
    ]
}

fn fixture() -> &'static [u8] {
    static FIXTURE: OnceLock<Vec<u8>> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_constant_publish.tri"),
            &Default::default(),
            trident::NativeArtifactProfile::RawNoun,
            LIMITS,
        )
        .unwrap_or_else(|errors| panic!("provenance fixture: {errors:?}"))
        .bytes
    })
}

fn execute(modules: &[Module<'_>]) -> Result<Vec<u8>, u32> {
    // generate_with_input allocates a fresh arena on every invocation. Metadata
    // is produced by guest parse/resolve/publish; the host supplies no bindings.
    codegen::generate_with_input::<{ 1 << 20 }>(fixture(), |arena| {
        let mut rows = Vec::new();
        for module in modules {
            let name = schema::bytes(arena, module.owner.as_bytes()).unwrap();
            let source = schema::bytes(arena, module.body.as_bytes()).unwrap();
            let mut uses = schema::atom(arena, 0).unwrap();
            for &target in module.imports.iter().rev() {
                let zero = schema::atom(arena, 0).unwrap();
                let path = schema::pair(arena, zero, zero).unwrap();
                let location = schema::pair(arena, zero, path).unwrap();
                let target = schema::atom(arena, target).unwrap();
                let occurrence = schema::pair(arena, target, location).unwrap();
                uses = schema::pair(arena, occurrence, uses).unwrap();
            }
            let body = schema::pair(arena, source, uses).unwrap();
            rows.push(schema::pair(arena, name, body).unwrap());
        }
        schema::seq(arena, &rows).unwrap()
    })
}

fn read_rows<const WIDTH: usize>(arena: &mut Arena, root: Order) -> Vec<[u64; WIDTH]> {
    let rows = data::Seq::decode(arena, root, 4096, 1_000_000).unwrap();
    (0..rows.len())
        .map(|index| {
            let mut cursor = rows.get(arena, index).unwrap();
            let mut result = [0; WIDTH];
            for value in &mut result[..WIDTH - 1] {
                *value = data::value(arena, arena.head(cursor).unwrap()).unwrap();
                cursor = arena.tail(cursor).unwrap();
            }
            result[WIDTH - 1] = data::value(arena, cursor).unwrap();
            result
        })
        .collect()
}

fn declaration_span(body: &str, name: &str) -> (u64, u64) {
    let prefix = format!("const {name}:");
    let start = body.find(&prefix).unwrap() + "const ".len();
    (start as u64, (start + name.len()) as u64)
}

fn expected(
    modules: &[Module<'_>],
    owner: usize,
    declaration: u64,
    name: &str,
    public: bool,
    unsigned: bool,
) -> [u64; 10] {
    let (start, end) = declaration_span(modules[owner].body, name);
    let literal = if unsigned { U32_LITERAL } else { FIELD_LITERAL };
    let literal_start = ORIGINAL.find(literal).unwrap() as u64;
    [
        owner as u64,
        declaration,
        start,
        end,
        if unsigned { 3 } else { 0 },
        u64::from(public),
        if unsigned { u64::from(u32::MAX) } else { 0 },
        0,
        literal_start,
        literal_start + literal.len() as u64,
    ]
}

#[test]
fn parsed_and_published_aliases_preserve_terminal_owner_and_exact_source_spans() {
    support::worker(|| {
        let modules = modules(BRIDGE);
        let output = execute(&modules).unwrap();
        let mut arena = Arena::try_new_boxed().unwrap();
        let result = artifact::decode(&mut arena, &output, LIMITS).unwrap();
        let declarations = arena.head(result).unwrap();
        let exports = arena.tail(result).unwrap();
        let actual_declarations = read_rows::<10>(&mut arena, declarations);
        let actual_exports = read_rows::<8>(&mut arena, exports);
        let expected_declarations: Vec<_> = [
            (0, 0, "RAW", false, false),
            (0, 1, "UINT", false, true),
            (0, 2, "SourceField", true, false),
            (0, 3, "LocalField", false, false),
            (0, 4, "SourceUint", true, true),
            (1, 0, "CachedField", false, false),
            (1, 1, "ExportField", true, false),
            (1, 2, "CachedUint", false, true),
            (1, 3, "ExportUint", true, true),
            (2, 0, "FinalField", true, false),
            (2, 1, "FinalUint", true, true),
            (2, 2, "PrivateAlias", false, false),
        ]
        .into_iter()
        .map(|(owner, id, name, public, unsigned)| {
            expected(&modules, owner, id, name, public, unsigned)
        })
        .collect();
        assert_eq!(actual_declarations, expected_declarations);
        let expected_exports: Vec<_> = expected_declarations
            .iter()
            .filter(|row| row[5] == 1)
            .map(|row| {
                [
                    row[0], row[2], row[3], row[4], row[6], row[7], row[8], row[9],
                ]
            })
            .collect();
        assert_eq!(actual_exports, expected_exports);

        // Declaration names belong to three owners, while both terminal raw
        // literals stay at the first source's byte spans despite normalization.
        for row in actual_exports {
            let owner = row[0] as usize;
            assert!(modules[owner].body.is_char_boundary(row[1] as usize));
            assert!(!modules[owner].body[row[1] as usize..row[2] as usize].is_empty());
            let origin = &modules[row[5] as usize].body[row[6] as usize..row[7] as usize];
            assert_eq!(
                origin,
                if row[3] == 3 {
                    U32_LITERAL
                } else {
                    FIELD_LITERAL
                }
            );
        }
    });
}

#[test]
fn failed_private_import_resolution_cannot_reuse_a_previous_publish_result() {
    support::worker(|| {
        let valid = execute(&modules(BRIDGE)).unwrap();
        let invalid = format!("{BRIDGE}\nconst BROKEN:Field=origin.MISSING");
        assert_eq!(execute(&modules(&invalid)), Err(5));
        assert_eq!(execute(&modules(BRIDGE)).unwrap(), valid);
    });
}
