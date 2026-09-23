// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Z3 process runner for SMT-LIB2 scripts.
//!
//! Locates Z3 in PATH, writes the SMT script to private process stdin,
//! invokes Z3 with a timeout, and parses the result.

use std::io::Write;

use super::{SmtResult, SmtStatus};

/// Try to run Z3 on an SMT-LIB2 script.
///
/// Returns `Ok(SmtResult)` if Z3 was found and ran,
/// `Err(String)` if Z3 is not available.
pub fn run_z3(smt_script: &str) -> Result<SmtResult, String> {
    use std::process::Command;

    // Check if z3 is available
    let z3_path = which_z3().ok_or("z3 not found in PATH")?;

    // Stdin is private to this solver process: concurrent audits cannot replace
    // a shared temporary script and accidentally verify another program.
    let mut child = Command::new(&z3_path)
        .args(["-T:10", "-in"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run z3: {e}"))?;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(smt_script.as_bytes())
        .map_err(|e| format!("cannot write solver input: {e}"))?;
    let output = child
        .wait_with_output()
        .map_err(|e| format!("cannot wait for z3: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let full_output = if stderr.is_empty() {
        stdout.clone()
    } else {
        format!("{}\n{}", stdout, stderr)
    };

    let status =
        if !output.status.success() || !stderr.trim().is_empty() || stdout.contains("(error") {
            SmtStatus::Error(full_output.clone())
        } else {
            match stdout.trim() {
                "sat" => SmtStatus::Sat,
                "unsat" => SmtStatus::Unsat,
                "unknown" | "timeout" => SmtStatus::Unknown,
                _ => SmtStatus::Error(full_output.clone()),
            }
        };

    // Extract model if SAT
    let model = if status == SmtStatus::Sat {
        // Model follows "sat" line
        let lines: Vec<&str> = stdout.lines().collect();
        if lines.len() > 1 {
            Some(lines[1..].join("\n"))
        } else {
            None
        }
    } else {
        None
    };

    Ok(SmtResult {
        output: full_output,
        status,
        model,
    })
}

/// Find z3 in PATH.
fn which_z3() -> Option<std::path::PathBuf> {
    find_z3(&std::env::var_os("PATH")?)
}

fn find_z3(path: &std::ffi::OsStr) -> Option<std::path::PathBuf> {
    std::env::split_paths(path).find_map(|directory| {
        let candidate = directory.join(format!("z3{}", std::env::consts::EXE_SUFFIX));
        let metadata = candidate.metadata().ok()?;
        if !metadata.is_file() {
            return None;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o111 == 0 {
                return None;
            }
        }
        Some(candidate)
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn native_solver_discovery_needs_no_shell_and_preserves_unicode_paths() {
        let root = tempfile::tempdir().unwrap();
        let missing = root.path().join("missing");
        let directory = root.path().join("solver пробел");
        std::fs::create_dir(&directory).unwrap();
        let candidate = directory.join(format!("z3{}", std::env::consts::EXE_SUFFIX));
        let paths = std::env::join_paths([&missing, &directory]).unwrap();
        std::fs::create_dir(&candidate).unwrap();
        assert_eq!(super::find_z3(&paths), None);
        std::fs::remove_dir(&candidate).unwrap();
        std::fs::write(&candidate, b"fixture").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&candidate, std::fs::Permissions::from_mode(0o600)).unwrap();
            assert_eq!(super::find_z3(&paths), None);
            std::fs::set_permissions(&candidate, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        assert_eq!(super::find_z3(&paths), Some(candidate));
    }
}
