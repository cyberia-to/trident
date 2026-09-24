//! Validated native wrappers must retain their defining module across APIs.
#[path = "native_control/support.rs"]
mod support;
use std::path::Path;
use trident::{CompileOptions, RawArtifact, RAW_ARTIFACT_LIMITS};

const VAULT: &str = "module bank.vault
pub struct Seal { tree: Noun, pub revision: Field }
pub struct Envelope { pub item: Seal }
pub fn create(x: Noun) -> Seal { Seal { tree: x, revision: 1 } }
pub fn wrap(x: Noun) -> Envelope { Envelope { item: create(x) } }
pub fn first<N>(x: [Noun; N]) -> Seal { create(x[0]) }
pub fn read(x: Seal) -> Noun { x.tree }
pub fn replace(x: Seal, y: Noun) -> Seal {
    let mut s = x
    s.tree = y
    s
}
";

fn write(root: &Path, name: &str, source: &str) {
    let path = root.join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, source).unwrap();
}

fn compile(root: &Path, source: &str) -> Result<RawArtifact, String> {
    write(root, "entry.tri", source);
    trident::compile_raw_artifact_project(
        &root.join("entry.tri"),
        &CompileOptions::default(),
        RAW_ARTIFACT_LIMITS,
    )
    .map_err(|errors| format!("{errors:?}"))
}

#[test]
fn imported_private_fields_reject_construction_read_and_mutation() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "bank/vault.tri", VAULT);
    for body in [
        "let s = vault.Seal { tree: input, revision: 1 }",
        "let s = bank.vault.Seal { tree: input, revision: 1 }",
        "let s = vault.create(input)\nlet stolen = s.tree",
        "let stolen = vault.create(input).tree",
        "let mut s = vault.create(input)\ns.tree = input",
        "let mut s = [vault.create(input)]\ns[0].tree = input",
        "let s = [vault.create(input)]\nlet stolen = s[0].tree",
        "let s = vault.wrap(input)\nlet stolen = s.item.tree",
        "let mut s = vault.wrap(input)\ns.item.tree = input",
        "let stolen = vault.wrap(input).item.tree",
        "let s = vault.first([input])\nlet stolen = s.tree",
        "let mut s = vault.first<1>([input])\ns.tree = input",
    ] {
        let source = format!(
            "program entry\nuse bank.vault\nfn main(input: Noun) -> Noun {{\n{body}\ninput\n}}"
        );
        let error = compile(dir.path(), &source).unwrap_err();
        assert!(
            error.contains("bank.vault.Seal.tree' is private"),
            "{body}: {error}"
        );
    }
}

#[test]
fn opaque_forwarded_and_generic_values_keep_the_original_owner() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "bank/vault.tri", VAULT);
    write(
        dir.path(),
        "relay.tri",
        "module relay\nuse bank.vault
pub fn forward(x: Noun) -> vault.Seal { vault.create(x) }
pub fn forward_many<N>(x: [Noun; N]) -> vault.Seal { vault.first(x) }",
    );
    for call in ["relay.forward(input)", "relay.forward_many([input])"] {
        let source = format!(
            "program entry\nuse relay\nfn main(input: Noun) -> Noun {{ let x = {call}\nx.tree }}"
        );
        let error = compile(dir.path(), &source).unwrap_err();
        assert!(
            error.contains("bank.vault.Seal.tree' is private"),
            "{error}"
        );
    }
}

#[test]
fn same_shape_from_another_module_cannot_forge_or_destructure_a_wrapper() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "bank/vault.tri", VAULT);
    for body in [
        "vault.read(Seal { tree: input, revision: 1 })",
        "let s: Seal = vault.create(input)\ns.tree",
        "let xs = [Seal { tree: input, revision: 1 }, vault.create(input)]\ninput",
        "let mut s = Seal { tree: input, revision: 1 }\ns = vault.create(input)\ns.tree",
        "match vault.create(input) { Seal { tree, revision: _ } => { return tree } }",
    ] {
        let source = format!(
            "program entry\nuse bank.vault\nstruct Seal {{ tree: Noun, pub revision: Field }}\nfn main(input: Noun) -> Noun {{ {body} }}"
        );
        let error = compile(dir.path(), &source).unwrap_err();
        assert!(error.contains("bank.vault.Seal"), "{error}");
        assert!(
            error.contains("entry.Seal") || error.contains("struct pattern"),
            "{error}"
        );
    }
}

#[test]
fn module_name_spoofing_is_rejected_before_granting_private_access() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "bank/vault.tri", VAULT);
    write(
        dir.path(),
        "spoof.tri",
        "module bank.vault\nuse bank.vault
pub fn steal(x: Noun) -> Noun { vault.create(x).tree }",
    );
    let source = "program entry\nuse spoof\nfn main(input: Noun) -> Noun { vault.steal(input) }";
    assert!(compile(dir.path(), source)
        .unwrap_err()
        .contains("duplicate module 'bank.vault'"));
    write(
        dir.path(),
        "owner.tri",
        &VAULT.replace("bank.vault", "owner"),
    );
    for header in ["program owner // comment", "program\towner"] {
        let source = format!(
            "{header}\nuse owner\nfn main(input: Noun) -> Noun {{ owner.create(input).tree }}"
        );
        let error = compile(dir.path(), &source).unwrap_err();
        assert!(error.contains("duplicate module 'owner'"), "{error}");
    }
}

#[test]
fn owner_functions_and_public_fields_preserve_native_values() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "bank/vault.tri", VAULT);
    let source = "program entry\nuse bank.vault
fn main(input: Noun) -> Noun {
    let mut s = vault.first<2>([input, nox_noun_atom(9)])
    s.revision = 7
    let mut outer = vault.Envelope { item: s }
    outer.item = vault.replace(s, nox_noun_atom(s.revision))
    vault.read(outer.item)
}";
    let program = compile(dir.path(), source).unwrap();
    let result = support::worker(move || {
        support::run(
            &program.bytes,
            23,
            1_000_000,
            65536,
            RAW_ARTIFACT_LIMITS.max_nodes,
        )
        .unwrap()
        .0
    });
    assert_eq!(result, 7);
}

#[test]
fn fixed_word_lowering_and_editor_use_the_same_private_field_gate() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "vault.tri", "module vault\npub struct Seal { value: Field }\npub fn create() -> Seal { Seal { value: 7 } }");
    let source = "program entry\nuse vault\nfn main() -> Field { vault.create().value }";
    write(dir.path(), "entry.tri", source);
    let path = dir.path().join("entry.tri");
    let error = trident::build_tir_project(&path, &CompileOptions::default()).unwrap_err();
    assert!(error
        .iter()
        .any(|e| e.message.contains("vault.Seal.value' is private")));
    let error = trident::check_file_in_project(source, &path).unwrap_err();
    assert!(error
        .iter()
        .any(|e| e.message.contains("vault.Seal.value' is private")));

    write(dir.path(), "vault.tri", "module vault\npub struct Seal { pub value: Field }\npub fn create() -> Seal { Seal { value: 7 } }");
    assert!(trident::build_tir_project(&path, &CompileOptions::default()).is_ok());
    assert!(trident::check_file_in_project(source, &path).is_ok());
}
