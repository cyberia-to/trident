//! Check failures retain their compiler diagnostics at the process boundary.

use std::process::Command;

#[test]
fn check_reports_syntax_and_type_failures_on_stderr() {
    for (source, expected) in [
        ("program entry use a.", "expected identifier"),
        ("program entry fn main()->Field{missing}", "missing"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let entry = dir.path().join("entry.tri");
        std::fs::write(&entry, source).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_trident"))
            .arg("check")
            .arg(&entry)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains(expected), "{stderr}");
        assert!(!stderr.contains("OK:"), "{stderr}");
    }
}
