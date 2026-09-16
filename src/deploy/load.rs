use std::path::{Path, PathBuf};

/// A package whose declared program exists inside the directory and matches
/// its manifest content digest. This is integrity checking, not proof verification.
pub struct ValidatedArtifact {
    pub manifest_json: String,
    pub program_path: PathBuf,
}

pub(super) fn validate_filename(name: &str) -> Result<(), String> {
    if name.is_empty()
        || matches!(name, "." | "..")
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        return Err("artifact name must be a plain filename without traversal".into());
    }
    Ok(())
}

/// Load the exact filename declared by the manifest; never guess a target's
/// assembly format from whichever files happen to be present.
pub fn load_artifact(directory: &Path) -> Result<ValidatedArtifact, String> {
    let manifest_json = std::fs::read_to_string(directory.join("manifest.json"))
        .map_err(|e| format!("cannot read package manifest: {e}"))?;
    let manifest: serde_json::Value = serde_json::from_str(&manifest_json)
        .map_err(|e| format!("invalid package manifest: {e}"))?;
    let filename = manifest
        .get("program_file")
        .and_then(|v| v.as_str())
        .ok_or("manifest lacks program_file; repackage with the current compiler")?;
    validate_filename(filename)?;
    if filename == "manifest.json" {
        return Err("manifest cannot name itself as the compiled program".into());
    }
    let directory = directory
        .canonicalize()
        .map_err(|e| format!("cannot resolve package directory: {e}"))?;
    let program_path = directory
        .join(filename)
        .canonicalize()
        .map_err(|e| format!("cannot resolve declared program '{filename}': {e}"))?;
    if program_path.parent() != Some(directory.as_path()) || !program_path.is_file() {
        return Err("declared program is not a file inside the package directory".into());
    }
    let expected = manifest
        .get("program_digest")
        .and_then(|v| v.as_str())
        .ok_or("manifest lacks program_digest")?;
    let assembly =
        std::fs::read(&program_path).map_err(|e| format!("cannot read declared program: {e}"))?;
    let actual = crate::hash::ContentHash(crate::hash::content_hash_bytes(&assembly)).to_hex();
    if expected != actual {
        return Err("declared program content does not match manifest digest".into());
    }
    Ok(ValidatedArtifact {
        manifest_json,
        program_path,
    })
}
