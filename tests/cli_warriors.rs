//! Public process contract: select one terrain, and never report an action not performed.
#![cfg(unix)]

mod support;

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
        .env("TRIDENT_TARGET_PACKAGES", dir)
        .output()
        .unwrap()
}

fn stub(dir: &Path, name: &str) {
    let path = dir.join(name);
    fs::write(
        &path,
        format!("#!/bin/sh\nif [ \"$1\" = describe ]; then /bin/cat \"$3.json\"; else printf '%s\\n' '{name}' \"$@\"; fi\n"),
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
    support::write_triton_package(root);
    let mut nox = support::triton_package();
    nox.owner = "joy".into();
    nox.terrain = trident::target::TerrainConfig::nox();
    nox.intrinsics = nox.terrain.supported_intrinsics();
    fs::write(
        root.join("nox.json"),
        serde_json::to_vec(&nox.seal().unwrap()).unwrap(),
    )
    .unwrap();
    stub(root, "trident-triton");
    stub(root, "trident-nox");
    for action in ["build", "run", "prove", "verify", "test"] {
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
    for action in ["run", "prove", "verify"] {
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

#[test]
fn invalid_packages_and_unavailable_commands_never_reach_the_warrior() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("main.tri"), "program proof\nfn main() {}\n").unwrap();
    stub(root, "trident-triton");
    for incompatible_api in [false, true] {
        let mut package = support::triton_package();
        if incompatible_api {
            package.compiler_api = 999;
        } else {
            package.runtime.prove = false;
        }
        fs::write(
            root.join("triton.json"),
            serde_json::to_vec(&package).unwrap(),
        )
        .unwrap();
        let output = invoke(root, root, &["prove", "main.tri", "--target", "triton"]);
        assert!(!output.status.success());
        assert!(
            output.stdout.is_empty(),
            "unsupported request reached warrior"
        );
        let diagnostic = String::from_utf8_lossy(&output.stderr);
        assert!(
            diagnostic.contains(if incompatible_api {
                "unsupported target package"
            } else {
                "does not support"
            }),
            "{diagnostic}"
        );
    }
}

#[test]
fn describe_and_execution_use_the_same_wrapper() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    support::write_triton_package(root);
    fs::write(root.join("main.tri"), "program dispatch\nfn main() {}\n").unwrap();
    let wrapper = root.join("trident-triton");
    fs::write(&wrapper, "#!/bin/sh\nif [ \"$1\" = describe ]; then /bin/cat triton.json; else printf '%s\\n' wrapper \"$@\"; fi\n").unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    stub(root, "trisha");
    let output = Command::new(env!("CARGO_BIN_EXE_trident"))
        .args(["run", "main.tri", "--target", "triton"])
        .current_dir(root)
        .env("PATH", root)
        .env_remove("TRIDENT_TARGET_PACKAGES")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("wrapper\nrun\n"));
}

#[test]
fn witness_transport_reaches_the_owner_without_reading_or_expanding_its_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    support::write_triton_package(root);
    stub(root, "trident-triton");
    fs::write(root.join("main.tri"), "program witness\nfn main() {}\n").unwrap();
    // The stub owns this deliberately absent path; core must only forward it.
    let witness = "private witness $(literal).json";
    for action in ["run", "prove"] {
        let args = [
            action,
            "main.tri",
            "--target",
            "triton",
            "--input-file",
            witness,
        ];
        let output = invoke(root, root, &args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains(&format!("--input-file\n{witness}\n"))
        );
        for flag in ["--input-values", "--secret", "--digests"] {
            let mut conflicting = args.to_vec();
            conflicting.extend([flag, "1"]);
            let rejected = invoke(root, root, &conflicting);
            assert_eq!(rejected.status.code(), Some(2));
            assert!(rejected.stdout.is_empty());
        }
        let output = invoke(
            root,
            root,
            &[
                action,
                "main.tri",
                "--target",
                "triton",
                "--digests",
                "1,2,3,4,5",
            ],
        );
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("--digests\n1,2,3,4,5\n"));
    }
}
