// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use super::*;

#[test]
fn discovery_uses_parsed_headers_across_whitespace_comments_and_same_line_items() {
    for source in [
        "program entry use values fn main()->Field{values.value()}",
        "program\tentry\nuse\tvalues // library\nfn main()->Field{values.value()}",
        "// lead\nprogram entry // name\nuse // import\nvalues // body\nfn main()->Field{values.value()}",
    ] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("values.tri"), "module values pub fn value()->Field{7}").unwrap();
        let entry = dir.path().join("different_basename.tri");
        std::fs::write(&entry, source).unwrap();
        let modules = resolve_modules(&entry).unwrap();
        assert_eq!(modules.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), vec!["values", "entry"]);
        assert_eq!(modules.last().unwrap().file.uses.len(), 1);
    }
}

#[test]
fn discovery_rejects_owner_mismatch_imported_programs_and_malformed_headers() {
    for source in [
        "module other pub fn value()->Field{7}",
        "program values fn main(){}",
        "module values use ....escape fn f(){}",
    ] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("values.tri"), source).unwrap();
        let entry = dir.path().join("main.tri");
        std::fs::write(&entry, "program entry use values fn main(){}").unwrap();
        assert!(resolve_modules(&entry).is_err(), "{source}");
    }
}

#[test]
fn legacy_and_canonical_imports_discover_one_owner() {
    let dir = tempfile::tempdir().unwrap();
    let entry = dir.path().join("main.tri");
    std::fs::write(
        &entry,
        "program entry use std.convert use vm.core.convert fn main(){}",
    )
    .unwrap();
    let modules = resolve_modules(&entry).unwrap();
    assert_eq!(
        modules
            .iter()
            .filter(|m| m.name == "vm.core.convert")
            .count(),
        1
    );
    assert_eq!(modules.last().unwrap().file.uses.len(), 2);
}

// --- Error path tests ---

#[test]
fn test_error_missing_entry_file() {
    let result = resolve_modules(Path::new("/nonexistent/path/to/file.tri"));
    assert!(result.is_err(), "should error on missing entry file");
    let diags = result.unwrap_err();
    assert!(
        diags[0].message.contains("cannot read"),
        "should report file read error, got: {}",
        diags[0].message
    );
    assert!(
        diags[0].help.is_some(),
        "file-not-found error should have help text"
    );
}

#[test]
fn test_error_module_not_found_has_path() {
    // Create a temp file that uses a nonexistent module
    let dir = std::env::temp_dir().join("trident_test_resolve");
    let _ = std::fs::create_dir_all(&dir);
    let entry = dir.join("test_missing.tri");
    std::fs::write(
        &entry,
        "program test_missing\nuse nonexistent_module\nfn main() {}\n",
    )
    .unwrap();

    let result = resolve_modules(&entry);
    assert!(result.is_err(), "should error on missing module");
    let diags = result.unwrap_err();
    let has_not_found = diags.iter().any(|d| {
        d.message
            .contains("cannot find module 'nonexistent_module'")
    });
    assert!(
        has_not_found,
        "should report module not found with name, got: {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    // Check that it says where it looked
    let has_path = diags.iter().any(|d| d.message.contains("looked at"));
    assert!(has_path, "should say where it looked for the module");
    // Check help text
    let has_help = diags.iter().any(|d| d.help.is_some());
    assert!(has_help, "module-not-found error should have help text");

    // Cleanup
    let _ = std::fs::remove_file(&entry);
}

#[test]
fn test_path_traversal_rejected() {
    // A module name with ".." should not escape the project directory
    let dir = std::env::temp_dir().join("trident_test_traversal");
    let _ = std::fs::create_dir_all(&dir);
    let entry = dir.join("test_traversal.tri");
    std::fs::write(
        &entry,
        "program test_traversal\nuse ....etc.passwd\nfn main() {}\n",
    )
    .unwrap();

    let result = resolve_modules(&entry);
    assert!(result.is_err(), "path traversal module should fail");
    let diags = result.unwrap_err();
    // The parser rejects traversal before discovery can read an outside path.
    let has_error = diags
        .iter()
        .any(|d| d.message.contains("expected identifier"));
    assert!(
        has_error,
        "should reject invalid module syntax, got: {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );

    let _ = std::fs::remove_file(&entry);
}

#[test]
fn dependency_overlay_changes_discovery_and_owner_validation() {
    let dir = tempfile::tempdir().unwrap();
    let entry = dir.path().join("main.tri");
    let dependency = dir.path().join("dep.tri");
    std::fs::write(&entry, "program app use dep fn main(){}").unwrap();
    std::fs::write(&dependency, "module dep").unwrap();
    std::fs::write(dir.path().join("leaf.tri"), "module leaf").unwrap();
    let resolve = |source| {
        resolve_modules_with_overlay(&entry, Vec::new(), Default::default(), &dependency, source)
    };
    let modules = resolve("module dep use leaf").unwrap();
    assert_eq!(
        modules.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(),
        ["leaf", "dep", "app"]
    );
    assert!(resolve("module different use leaf").is_err());
    assert!(resolve("module dep use missing").is_err());
}

#[test]
fn canonical_and_legacy_spellings_share_cycle_detection() {
    let dir = tempfile::tempdir().unwrap();
    let entry = dir.path().join("main.tri");
    std::fs::write(&entry, "program app use std.convert fn main(){}").unwrap();
    let sources = std::collections::BTreeMap::from([(
        "vm.core.convert".into(),
        "module vm.core.convert use std.convert".into(),
    )]);
    let errors = resolve_modules_with_sources(&entry, Vec::new(), sources).unwrap_err();
    assert!(errors
        .iter()
        .any(|e| e.message.contains("circular") || e.message.contains("cycle")));
}

#[test]
fn canonical_namespace_paths_do_not_repeat_extension_remapping() {
    let dir = tempfile::tempdir().unwrap();
    let entry = dir.path().join("main.tri");
    std::fs::write(&entry, "program app fn main(){}").unwrap();
    let mut resolver = ModuleResolver::new(&entry).unwrap();
    resolver.stdlib_dir = Some(dir.path().join("std"));
    resolver.os_dir = Some(dir.path().join("os"));
    assert_eq!(
        resolver.resolve_path("std.library.ext.helper"),
        dir.path().join("std/library/ext/helper.tri")
    );
}
