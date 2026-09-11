//! Process-level regression: `trident test` must execute the selected tests.

mod support;

use std::process::{Command, Output};

fn invoke(source: &str, extra: &[&str]) -> Output {
    let dir = tempfile::tempdir().unwrap();
    support::write_triton_package(dir.path());
    let entry = dir.path().join("main.tri");
    std::fs::write(&entry, source).unwrap();
    Command::new(env!("CARGO_BIN_EXE_trident"))
        .arg("test")
        .arg(&entry)
        .args(extra)
        .env("TRIDENT_TARGET_PACKAGES", dir.path())
        .output()
        .unwrap()
}

fn report(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn failing_assertion_in_test_fails_even_when_main_succeeds() {
    let output = invoke(
        "program bad\nfn main() {}\n#[test]\nfn fails() { assert(false) }",
        &[],
    );
    assert!(!output.status.success(), "{}", report(&output));
    assert!(
        report(&output).contains("bad.fails ... FAILED"),
        "{}",
        report(&output)
    );
    assert!(report(&output).contains("0 passed; 1 failed"));
}

#[test]
fn passing_test_does_not_run_the_program_main() {
    let output = invoke(
        "program good\nfn main() { assert(false) }\n#[test]\nfn passes() { assert(3 + 5 == 8) }",
        &[],
    );
    assert!(output.status.success(), "{}", report(&output));
    assert!(report(&output).contains("good.passes ... ok"));
    assert!(report(&output).contains("1 passed; 0 failed"));
}

#[test]
fn all_tests_run_and_the_process_reports_aggregate_failure() {
    let output = invoke("program mixed\nfn main() {}\n#[test]\nfn first() { assert(false) }\n#[test]\nfn second() { assert(true) }", &[]);
    assert!(!output.status.success());
    assert!(
        report(&output).contains("mixed.second ... ok"),
        "{}",
        report(&output)
    );
    assert!(report(&output).contains("1 passed; 1 failed"));
}

#[test]
fn profile_cfg_filters_tests_and_test_cfg_is_enabled() {
    let source = "program configured\nfn main() {}\n#[cfg(debug)]\n#[test]\nfn debug_passes() { assert(true) }\n#[cfg(release)]\n#[test]\nfn release_fails() { assert(false) }\n#[cfg(test)]\n#[test]\nfn test_enabled() { assert(true) }";
    let debug = invoke(source, &[]);
    assert!(debug.status.success(), "{}", report(&debug));
    assert!(report(&debug).contains("2 passed; 0 failed"));
    assert!(!report(&debug).contains("release_fails"));
    let release = invoke(source, &["--profile", "release"]);
    assert!(!release.status.success());
    assert!(
        report(&release).contains("1 passed; 1 failed"),
        "{}",
        report(&release)
    );
    assert!(!report(&release).contains("debug_passes"));
}

#[test]
fn imported_module_tests_and_private_helpers_execute_in_their_own_context() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("helper.tri"), "module helper\nfn private_value() -> Field { 7 }\npub fn value() -> Field { private_value() }\n#[test]\nfn dependency_test() { assert(value() == 7) }").unwrap();
    let entry = dir.path().join("main.tri");
    std::fs::write(&entry, "program importing\nuse helper\nfn main() {}\n#[test]\nfn entry_test() { assert(helper.value() == 7) }").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_trident"))
        .arg("test")
        .arg(&entry)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", report(&output));
    assert!(report(&output).contains("helper.dependency_test ... ok"));
    assert!(report(&output).contains("importing.entry_test ... ok"));
    assert!(report(&output).contains("2 passed; 0 failed"));
}

#[test]
fn non_nox_library_api_refuses_to_claim_test_success() {
    let options = trident::CompileOptions::default().with_package(support::triton_package()).unwrap();
    let errors = trident::run_tests(std::path::Path::new("unused.tri"), &options).unwrap_err();
    assert!(errors[0]
        .message
        .contains("warrior test command is not implemented"));
}

#[test]
fn triton_cli_reports_unsupported_test_execution_and_exits_nonzero() {
    let output = invoke(
        "program foreign\nfn main() {}\n#[test]\nfn passes() { assert(true) }",
        &["--target", "triton"],
    );
    assert!(!output.status.success());
    assert!(
        report(&output).contains("warrior test command is not implemented"),
        "{}",
        report(&output)
    );
    assert!(!report(&output).contains("test result: ok"));
}
