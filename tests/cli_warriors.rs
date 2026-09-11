//! Public process contract: select one terrain, and never report an action not performed.
#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Output},
};

fn invoke(dir: &Path, path: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_trident"))
        .args(args)
        .current_dir(dir)
        .env("PATH", path)
        .output()
        .unwrap()
}

fn stub(dir: &Path, name: &str) {
    let path = dir.join(name);
    fs::write(
        &path,
        format!("#!/bin/sh\nprintf '%s\\n' '{name}' \"$@\"\n"),
    )
    .unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn absent_warrior_never_reports_success() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.tri"),
        "program empty\nfn main() -> Field { 42 }\n",
    )
    .unwrap();
    for command in ["run", "prove", "verify"] {
        let output = invoke(dir.path(), dir.path(), &[command, "main.tri"]);
        assert!(!output.status.success(), "{command} falsely succeeded");
        let diagnostic = String::from_utf8_lossy(&output.stderr);
        assert!(diagnostic.contains("no warrior found"), "{diagnostic}");
        assert!(
            diagnostic.contains("cargo install cyber-joy"),
            "{diagnostic}"
        );
    }
}

#[test]
fn project_target_and_explicit_target_select_the_same_warrior_for_all_actions() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(
        root.join("main.tri"),
        "program dispatch\nfn main() -> Field { 42 }\n",
    )
    .unwrap();
    fs::write(root.join("trident.toml"), "[project]\nname = \"dispatch\"\nversion = \"0.1.0\"\nentry = \"main.tri\"\ntarget = \"triton\"\n").unwrap();
    stub(root, "trident-triton");
    stub(root, "trident-nox");
    for action in ["build", "run", "prove"] {
        let output = invoke(root, root, &[action, "."]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.starts_with("trident-triton\n"), "{text}");
        assert!(text.contains("--target\ntriton\n"), "{text}");
    }
    for action in ["run", "prove"] {
        let output = invoke(root, root, &[action, ".", "--target", "nox"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8(output.stdout)
            .unwrap()
            .starts_with("trident-nox\n"));
    }
    let output = invoke(root, root, &["build", ".", "--target", "nox"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(root.join("dispatch.nox").exists());

    fs::write(
        root.join("alternate.tri"),
        "program alternate\nfn main() -> Field { 7 }\n",
    )
    .unwrap();
    for action in ["build", "run", "prove"] {
        let output = invoke(root, root, &[action, "alternate.tri"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(
            text.lines().any(|line| line == "alternate.tri"),
            "explicit file replaced by project entry: {text}"
        );
    }
}

#[test]
fn installed_compiler_resolves_its_own_libraries_outside_checkout() {
    let dir = tempfile::tempdir().unwrap();
    let binary = dir.path().join("trident");
    fs::copy(env!("CARGO_BIN_EXE_trident"), &binary).unwrap();
    fs::write(
        dir.path().join("main.tri"),
        "program portable\nuse vm.core.field\nfn main() -> Field { 3 }\n",
    )
    .unwrap();
    let output = Command::new(binary)
        .args(["check", "main.tri"])
        .current_dir(dir.path())
        .env_remove("TRIDENT_STDLIB")
        .env_remove("TRIDENT_OSLIB")
        .env_remove("TRIDENT_EXTLIB")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
