#[path = "support/mod.rs"]
mod target_support;
use trident::ir::tree::lower::nox::NoxCompiler;
use trident::{parse_source_silent, CompileOptions};

fn rejects(files: &[&str]) {
    let files: Vec<_> = files
        .iter()
        .map(|s| parse_source_silent(s, "source.tri").unwrap())
        .collect();
    let refs: Vec<_> = files.iter().collect();
    let entry = files.last().unwrap();
    assert!(NoxCompiler::new()
        .compile_modules(&refs, entry, &Default::default())
        .is_err());
    let options = CompileOptions::default()
        .with_package(target_support::triton_package())
        .unwrap();
    assert!(
        trident::tir::builder::TIRBuilder::new(options.target_config)
            .with_module_types(&refs)
            .build_file(entry)
            .is_err()
    );
}
#[test]
fn supplied_definitions_do_not_grant_source_visibility() {
    rejects(&[
        "module app.hidden pub const SIZE:U32=2",
        "program app fn main(p:[Field;SIZE])->Field{p[0]}",
    ]);
    rejects(&[
        "module app.hidden pub const VALUE:Field=7",
        "program app fn main()->Field{hidden.VALUE}",
    ]);
    rejects(&[
        "module app.hidden pub fn secret()->Field{7}",
        "program app fn main()->Field{hidden.secret()}",
    ]);
    rejects(&[
        "module app.hidden pub struct S{pub x:Field}",
        "program app fn main()->Field{let s=hidden.S{x:7} s.x}",
    ]);
    for uses in ["", "use hidden"] {
        rejects(&[
            "module hidden fn secret()->Field{7}",
            &format!("program app {uses} fn main()->Field{{hidden.secret()}}"),
        ]);
        rejects(&[
            "module hidden struct S{pub x:Field}",
            &format!("program app {uses} fn main()->Field{{let s=hidden.S{{x:7}} s.x}}"),
        ]);
    }
    rejects(&[
        "module hidden pub fn secret()->Field{7}",
        "program app fn main()->Field{hidden.secret()}",
    ]);
    rejects(&[
        "module hidden pub const VALUE:Field=7",
        "program app fn main()->Field{hidden.VALUE}",
    ]);
    rejects(&[
        "module hidden pub struct S{pub x:Field}",
        "program app fn main()->Field{let s=hidden.S{x:7} s.x}",
    ]);
}
#[test]
fn supplied_programs_and_missing_imports_are_rejected() {
    rejects(&[
        "program hidden pub fn f()->Field{7}",
        "program app use hidden fn main()->Field{hidden.f()}",
    ]);
    rejects(&["program app use missing fn main()->Field{7}"]);
}
#[test]
fn opaque_noun_layouts_are_rejected_by_direct_stack_builder() {
    let files: Vec<_> = [
        "module hidden pub struct S{pub value:Noun} pub fn opaque()->S{S{value:nox_noun_atom(0)}}",
        "module facade use hidden pub fn opaque()->hidden.S{hidden.opaque()}",
        "program app use facade fn main(){let s=facade.opaque()}",
    ]
    .into_iter()
    .map(|s| parse_source_silent(s, "source.tri").unwrap())
    .collect();
    let options = CompileOptions::default()
        .with_package(target_support::triton_package())
        .unwrap();
    assert!(
        trident::tir::builder::TIRBuilder::new(options.target_config)
            .with_module_types(&files.iter().collect::<Vec<_>>())
            .build_file(files.last().unwrap())
            .is_err()
    );
}
