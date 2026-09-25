#[path = "native_control/support.rs"]
mod support;
#[path = "support/mod.rs"]
mod target_support;
use std::path::Path;
use trident::CompileOptions;

fn write(root: &Path, name: &str, source: &str) {
    let path = root.join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, source).unwrap();
}
fn entry(root: &Path, uses: &str, body: &str, raw: bool) -> std::path::PathBuf {
    let main = if raw {
        "fn main(input:Noun)->Noun{nox_noun_atom(result())}"
    } else {
        "fn main()->Field{result()}"
    };
    write(
        root,
        "entry.tri",
        &format!("program app {uses} fn result()->Field{{{body}}} {main}"),
    );
    root.join("entry.tri")
}
fn accepted(root: &Path, uses: &str, body: &str, expected: u64) {
    let path = entry(root, uses, body, true);
    let program = trident::compile_raw_artifact_project(
        &path,
        &CompileOptions::default(),
        trident::RAW_ARTIFACT_LIMITS,
    )
    .unwrap();
    assert_eq!(
        support::run(&program.bytes, 0, 1_000_000, 16384, 196608)
            .unwrap()
            .0,
        expected
    );
    let path = entry(root, uses, body, false);
    let source = std::fs::read_to_string(&path).unwrap();
    trident::check_file_in_project(&source, &path).unwrap();
    let options = CompileOptions::default()
        .with_package(target_support::triton_package())
        .unwrap();
    let modules = trident::build_tir_modules(&path, &options).unwrap();
    assert!(
        !format!("{:?}", modules.iter().map(|m| &m.ops).collect::<Vec<_>>()).contains("ERROR:")
    );
}
fn rejected(root: &Path, uses: &str, body: &str) {
    let path = entry(root, uses, body, true);
    assert!(
        trident::compile_raw_artifact_project(
            &path,
            &CompileOptions::default(),
            trident::RAW_ARTIFACT_LIMITS
        )
        .is_err(),
        "{uses}: {body}"
    );
    let path = entry(root, uses, body, false);
    let source = std::fs::read_to_string(&path).unwrap();
    assert!(trident::check_file_in_project(&source, &path).is_err());
    let options = CompileOptions::default()
        .with_package(target_support::triton_package())
        .unwrap();
    assert!(trident::build_tir_modules(&path, &options).is_err());
}
fn vault(root: &Path) {
    write(root, "bank/vault.tri", "module bank.vault pub const VALUE:Field=9 pub struct Seal{pub words:[Field;2],secret:Field} pub fn make()->Seal{Seal{words:[7,9],secret:11}} pub fn read(s:Seal)->Field{s.words[1]} pub fn first<N>(x:[Field;N])->Field{x[0]}");
    write(root, "facade.tri", "module facade use bank.vault pub fn make()->vault.Seal{vault.make()} pub fn read(s:vault.Seal)->Field{vault.read(s)} pub fn first<N>(x:[Field;N])->Field{vault.first(x)}");
}

#[test]
fn only_direct_imports_confer_function_constant_and_type_names() {
    support::worker(|| {
        let dir = tempfile::tempdir().unwrap();
        vault(dir.path());
        for body in [
            "let p=bank.vault.make() bank.vault.read(p)",
            "let p=vault.make() vault.read(p)",
            "bank.vault.VALUE",
            "vault.VALUE",
            "let p:bank.vault.Seal=facade.make() 7",
            "let p:vault.Seal=facade.make() 7",
            "bank.vault.first([7,9])",
            "vault.first([7,9])",
        ] {
            rejected(dir.path(), "use facade", body);
        }
        accepted(
            dir.path(),
            "use facade use bank.vault",
            "vault.VALUE+vault.first([7,9])",
            16,
        );
    });
}

#[test]
fn sibling_order_cannot_supply_an_undeclared_dependency() {
    support::worker(|| {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.tri", "module a pub fn value()->Field{7}");
        write(
            dir.path(),
            "z.tri",
            "module z pub fn leak()->Field{a.value()}",
        );
        for uses in ["use a use z", "use z use a"] {
            rejected(dir.path(), uses, "z.leak()");
        }
    });
}

#[test]
fn opaque_nominal_values_keep_layouts_across_direct_only_scopes() {
    support::worker(|| {
        let dir = tempfile::tempdir().unwrap();
        vault(dir.path());
        accepted(
            dir.path(),
            "use facade",
            "let p=facade.make() p.words[1]+facade.read(p)+facade.first([3,5])",
            21,
        );
        rejected(dir.path(), "use facade", "let p=facade.make() p.secret");
    });
}

#[test]
fn caller_aliases_cannot_reinterpret_a_returned_canonical_type() {
    support::worker(|| {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "b.tri",
            "module b pub struct Box{pub words:[Field;2]} pub fn make()->Box{Box{words:[7,9]}}",
        );
        write(
            dir.path(),
            "facade.tri",
            "module facade use b pub fn make()->b.Box{b.make()}",
        );
        write(
            dir.path(),
            "other/b.tri",
            "module other.b pub struct Box{pub words:[Field;3]} pub fn unrelated()->Field{11}",
        );
        accepted(
            dir.path(),
            "use facade use other.b",
            "let p=facade.make() p.words[1]",
            9,
        );
    });
}

#[test]
fn later_imports_preserve_earlier_symbols_they_do_not_replace() {
    support::worker(|| {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a/common.tri", "module a.common pub struct X{pub words:[Field;2]} pub const A:Field=3 pub fn first()->Field{5} pub fn shared()->Field{7}");
        write(dir.path(), "b/common.tri", "module b.common pub struct Y{pub words:[Field;3]} pub const B:Field=11 pub fn second()->Field{13} pub fn shared()->Field{17}");
        for (uses, expected) in [
            ("use a.common use b.common", 58),
            ("use b.common use a.common", 48),
            ("use a.common use b.common use a.common", 48),
        ] {
            accepted(dir.path(), uses, "let p:common.X=common.X{words:[7,9]} p.words[1]+common.A+common.B+common.first()+common.second()+common.shared()", expected);
        }
    });
}

#[test]
fn final_struct_declaration_owns_import_visibility() {
    support::worker(|| {
        let dir = tempfile::tempdir().unwrap();
        for definitions in [
            "pub struct S{pub x:Field} struct S{pub x:Field}",
            "pub struct S{pub x:Field} #[cfg(debug)] struct S{pub x:Field}",
        ] {
            write(
                dir.path(),
                "value.tri",
                &format!("module value {definitions}"),
            );
            rejected(dir.path(), "use value", "let s=value.S{x:7} s.x");
        }
        for definitions in [
            "struct S{pub x:Field} pub struct S{pub x:Field}",
            "pub struct S{pub x:Field} #[cfg(absent)] struct S{pub x:Field}",
        ] {
            write(
                dir.path(),
                "value.tri",
                &format!("module value {definitions}"),
            );
            accepted(dir.path(), "use value", "let s=value.S{x:7} s.x", 7);
        }
    });
}

#[test]
fn source_only_api_rejects_even_unused_missing_imports() {
    let source = "program app use missing fn main()->Field{7}";
    assert!(trident::check_silent(source, "entry.tri").is_err());
    assert!(trident::build_tir(
        source,
        "entry.tri",
        &CompileOptions::default()
            .with_package(target_support::triton_package())
            .unwrap()
    )
    .is_err());
}

#[test]
fn nested_foreign_owner_cannot_replace_the_dotted_state_builtin() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "app/os/state.tri",
        "module app.os.state pub fn read(k:Field)->Field{7}",
    );
    write(
        dir.path(),
        "facade.tri",
        "module facade use app.os.state pub fn value()->Field{state.read(0)}",
    );
    let path = entry(dir.path(), "use facade", "os.state.read(1)", true);
    let errors = trident::compile_raw_artifact_project(
        &path,
        &CompileOptions::default(),
        trident::RAW_ARTIFACT_LIMITS,
    )
    .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("forbids host services")),
        "{errors:?}"
    );
}
