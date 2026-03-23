// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use super::*;

#[test]
fn test_scan_module_header_program() {
    let (name, deps) =
        scan_module_header("program my_app\n\nuse merkle\nuse crypto.sponge\n\nfn main() {}");
    assert_eq!(name, Some("my_app".to_string()));
    assert_eq!(deps, vec!["merkle", "crypto.sponge"]);
}

#[test]
fn test_scan_module_header_module() {
    let (name, deps) =
        scan_module_header("module merkle\n\nuse std.convert\n\npub fn verify() {}");
    assert_eq!(name, Some("merkle".to_string()));
    assert_eq!(deps, vec!["std.convert"]);
}

#[test]
fn test_scan_module_header_no_deps() {
    let (name, deps) = scan_module_header("program simple\n\nfn main() {}");
    assert_eq!(name, Some("simple".to_string()));
    assert!(deps.is_empty());
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
    // Should get a "cannot find module" error, NOT actually read outside project
    let has_error = diags
        .iter()
        .any(|d| d.message.contains("cannot find module"));
    assert!(
        has_error,
        "should report module not found, got: {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );

    let _ = std::fs::remove_file(&entry);
}
