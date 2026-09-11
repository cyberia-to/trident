//! Explicit package files or the installed owner's side-effect-free describe command.
use super::{Diagnostic, Span, TargetPackage};
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub fn owner_for(target: &str) -> Option<&'static str> {
    match target {
        "triton" | "neptune" => Some("trisha"),
        "nox" | "cyber" => Some("joy"),
        _ => None,
    }
}

/// One provider lookup shared by descriptor discovery and CLI delegation.
pub fn warrior_path(target: &str) -> Option<PathBuf> {
    let owner = owner_for(target)?;
    let path = std::env::var_os("PATH")?;
    for name in [
        format!("trident-{target}"),
        format!("trident-{owner}"),
        owner.into(),
    ] {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
            let Ok(metadata) = candidate.metadata() else {
                continue;
            };
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if metadata.permissions().mode() & 0o111 == 0 {
                    continue;
                }
            }
            if metadata.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

const MAX: usize = 32 * 1024 * 1024;

fn describe(target: &str) -> Result<Vec<u8>, String> {
    let provider = warrior_path(target).ok_or_else(|| {
        format!("target '{target}' requires an installed warrior with describe support")
    })?;
    let mut child = Command::new(&provider)
        .args(["describe", "--target", target])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("{}: {e}", provider.display()))?;
    let stdout = child.stdout.take().ok_or("missing describe output pipe")?;
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    // Bound both output allocation and the time a provider can hold the compiler.
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stdout
            .take(MAX as u64 + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = send.send(result);
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    let mut output = None;
    loop {
        if output.is_none() {
            match receive.try_recv() {
                Ok(Ok(bytes)) if bytes.len() <= MAX => output = Some(bytes),
                Ok(result) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(match result {
                        Ok(_) => "target package exceeds 32 MiB".into(),
                        Err(e) => format!("cannot read target package: {e}"),
                    });
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("target package reader disconnected".into());
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }
        match child.try_wait() {
            Ok(Some(status)) if !status.success() => {
                return Err(format!(
                    "{} describe --target {target} failed: {status}",
                    provider.display()
                ))
            }
            Ok(Some(_)) if output.is_some() => {
                return output.ok_or("missing target package".into())
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e.to_string());
            }
            _ => {}
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("target package describe timed out after 15 seconds".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

impl TargetPackage {
    /// Read the actual executable's package, regardless of an offline lock override.
    pub fn discover_installed(target: &str) -> Result<Self, Diagnostic> {
        let err = |message: String| Diagnostic::error(message, Span::dummy());
        let owner = owner_for(target).ok_or_else(|| err(format!("unknown target: {target}")))?;
        Self::decode(&describe(target).map_err(err)?, target, owner)
    }

    pub fn discover(target: &str) -> Result<Self, Diagnostic> {
        let err = |message: String| Diagnostic::error(message, Span::dummy());
        if !super::package::identifier(target) {
            return Err(err("invalid target package name".into()));
        }
        let owner = owner_for(target).ok_or_else(|| {
            err(format!(
                "target '{target}' is declared only; no installed implementation is registered"
            ))
        })?;
        let bytes = if let Some(dir) = std::env::var_os("TRIDENT_TARGET_PACKAGES") {
            let path = PathBuf::from(dir).join(format!("{target}.json"));
            let file =
                std::fs::File::open(&path).map_err(|e| err(format!("{}: {e}", path.display())))?;
            let mut bytes = Vec::new();
            file.take(MAX as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| err(e.to_string()))?;
            bytes
        } else {
            describe(target).map_err(err)?
        };
        Self::decode(&bytes, target, owner)
    }

    fn decode(bytes: &[u8], target: &str, owner: &str) -> Result<Self, Diagnostic> {
        let err = |message: String| Diagnostic::error(message, Span::dummy());
        if bytes.len() > MAX {
            return Err(err("target package exceeds 32 MiB".into()));
        }
        let package: Self = serde_json::from_slice(&bytes)
            .map_err(|e| err(format!("invalid {owner} target package: {e}")))?;
        package.validate().map_err(err)?;
        let correct_target = match target {
            "triton" | "nox" => package.terrain.name == target && package.union.is_none(),
            "neptune" => {
                package.terrain.name == "triton"
                    && package.union.as_ref().map(|u| u.name.as_str()) == Some("neptune")
            }
            "cyber" => package.terrain.name == "nox" && package.union.is_none(),
            _ => false,
        };
        if package.owner != owner || !correct_target {
            return Err(err(
                "target package identity differs from requested owner/target".into(),
            ));
        }
        Ok(package)
    }
}
