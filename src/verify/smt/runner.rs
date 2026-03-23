// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Z3 process runner for SMT-LIB2 scripts.
//!
//! Locates Z3 in PATH, writes the SMT script to a temp file,
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

    // Write script to temp file
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("trident_smt.smt2");
    let mut f =
        std::fs::File::create(&temp_file).map_err(|e| format!("cannot create temp file: {}", e))?;
    f.write_all(smt_script.as_bytes())
        .map_err(|e| format!("cannot write temp file: {}", e))?;
    drop(f);

    // Run Z3 with timeout
    let output = Command::new(&z3_path)
        .arg("-T:10") // 10 second timeout
        .arg(temp_file.to_string_lossy().as_ref())
        .output()
        .map_err(|e| format!("cannot run z3: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_file);

    let full_output = if stderr.is_empty() {
        stdout.clone()
    } else {
        format!("{}\n{}", stdout, stderr)
    };

    let status = if stdout.starts_with("sat") {
        SmtStatus::Sat
    } else if stdout.starts_with("unsat") {
        SmtStatus::Unsat
    } else if stdout.contains("timeout") || stdout.contains("unknown") {
        SmtStatus::Unknown
    } else {
        SmtStatus::Error(full_output.clone())
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
fn which_z3() -> Option<String> {
    use std::process::Command;

    // Try `which z3` on Unix
    if let Ok(output) = Command::new("which").arg("z3").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }

    None
}
