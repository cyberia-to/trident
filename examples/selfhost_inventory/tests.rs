use super::*;

fn put(root: &Path, name: &str, source: &str) {
    let path = root.join(module_path(name).unwrap());
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, source).unwrap();
}

#[test]
fn comments_do_not_invent_features_and_nested_expressions_are_counted() {
    let source = "module demo\n// match for pub_write(9)\nfn f(n: U32) -> Field { let mut x: Field = 0 for i in 0..n bounded 5000 { x = x + 1 } x }";
    let file = trident::parse_source_silent(source, "demo").unwrap();
    let module = walk::inspect(&file, source, "demo.tri");
    assert_eq!(module.features["stmt.for"].count, 1);
    assert_eq!(module.features["operator.+"].count, 1);
    assert_eq!(module.features["operator.+"].first_line, 3);
    assert_eq!(module.features["binding.mutable"].count, 1);
    assert!(!module.features.contains_key("stmt.match"));
    assert!(!module.features.contains_key("expr.call"));
}

#[test]
fn transitive_diamond_imports_are_counted_once_and_paths_are_relocatable() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    for root in [a.path(), b.path()] {
        put(
            root,
            "root",
            "module root\nuse left\nuse right\nfn f() {}\n",
        );
        put(root, "left", "module left\nuse shared\n");
        put(root, "right", "module right\nuse shared\n");
        put(root, "shared", "module shared\nfn g() -> Field { 7 }\n");
    }
    let roots = BTreeSet::from(["root".to_string()]);
    let first = collect(a.path(), roots.clone()).unwrap();
    let second = collect(b.path(), roots).unwrap();
    assert_eq!(first.module_count, 4);
    assert_eq!(first.functions, 2);
    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    );
}

#[test]
fn incomplete_or_misnamed_source_closures_are_errors() {
    let dir = tempfile::tempdir().unwrap();
    put(dir.path(), "root", "module root\nuse missing\n");
    assert!(collect(dir.path(), BTreeSet::from(["root".into()]))
        .err()
        .unwrap()
        .contains("cannot read missing"));
    put(dir.path(), "missing", "module wrong\n");
    assert!(collect(dir.path(), BTreeSet::from(["root".into()]))
        .err()
        .unwrap()
        .contains("declares wrong"));
    put(dir.path(), "missing", "module missing\nfn broken(\n");
    assert!(collect(dir.path(), BTreeSet::from(["root".into()]))
        .err()
        .unwrap()
        .contains("cannot parse missing"));
}

#[test]
fn source_identity_changes_even_when_only_a_comment_changes() {
    let dir = tempfile::tempdir().unwrap();
    let source = "module root\nfn f() {}\n";
    put(dir.path(), "root", source);
    let roots = BTreeSet::from(["root".into()]);
    let first = collect(dir.path(), roots.clone()).unwrap();
    put(
        dir.path(),
        "root",
        &(source.to_owned() + "// different bytes\n"),
    );
    let second = collect(dir.path(), roots).unwrap();
    assert_eq!(first.features, second.features);
    assert_ne!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    );
}

#[test]
fn untrusted_module_names_cannot_escape_the_source_root() {
    for name in ["../secret", "/tmp/data", "a..b", "a/b", "a\\b"] {
        assert!(module_path(name).is_err(), "{name}");
    }
}
