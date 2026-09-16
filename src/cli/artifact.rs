//! Shared target-aware packaging compilation. Foreign assembly belongs to its warrior.
use std::path::{Path, PathBuf};
use std::process;
use trident::runtime::artifact::BundleCost;

pub struct PreparedArtifact {
    pub project: Option<trident::project::Project>,
    pub entry: PathBuf,
    /// Legacy local name: target assembly, including nox formulas.
    pub tasm: String,
    pub cost: BundleCost,
    pub file: trident::ast::File,
    pub name: String,
    pub version: String,
    pub source_hash: String,
    pub resolved: trident::target::ResolvedTarget,
}

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("error: {message}");
    process::exit(1)
}

/// A unique temporary build directory, removed on normal scope exit.
struct BuildDirectory(PathBuf);
impl BuildDirectory {
    fn new() -> Result<Self, std::io::Error> {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        for _ in 0..128 {
            let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("trident-package-{}-{now}-{id}", process::id()));
            match std::fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "cannot allocate package build directory",
        ))
    }
}
impl Drop for BuildDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn prepare_artifact(
    input: &Path,
    target: &str,
    profile: &str,
    audit: bool,
) -> PreparedArtifact {
    let ri = super::resolve_input(input);
    // Preserve union identity when resolving a Neptune package; do not re-resolve Triton.
    let options = super::resolve_options(target, profile, ri.project.as_ref());
    let (entry, options) = trident::source_options(input, &options).unwrap_or_else(|errors| {
        fail(
            errors
                .iter()
                .map(|e| e.message.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        )
    });
    let resolved = trident::target::ResolvedTarget {
        vm: options.target_config.clone(),
        state: None,
        os: options
            .target_package
            .as_ref()
            .and_then(|p| p.union.clone()),
    };
    eprintln!("Compiling {}...", entry.display());
    let bundle = if options.target_config.name == "nox" {
        trident::compile_to_bundle(&entry, &options)
    } else {
        let warrior =
            super::find_warrior(target).unwrap_or_else(|| super::missing_warrior(target, "build"));
        let dir = BuildDirectory::new().unwrap_or_else(|e| fail(e));
        let output = dir
            .0
            .join(format!("program{}", options.target_config.output_extension));
        let entry_arg = entry.to_string_lossy();
        let output_arg = output.to_string_lossy();
        let delegated = super::try_delegate_to_warrior(
            &warrior,
            "build",
            &[
                &entry_arg,
                "--target",
                target,
                "--profile",
                profile,
                "-o",
                &output_arg,
            ],
        );
        if let Err(error) = delegated {
            drop(dir);
            fail(error);
        }
        let assembly = match std::fs::read_to_string(&output) {
            Ok(assembly) => assembly,
            Err(error) => {
                drop(dir);
                fail(format!(
                    "warrior did not produce the requested artifact: {error}"
                ));
            }
        };
        // The core has no foreign cost model. Absence is serialized as unknown.
        let cost = BundleCost {
            table_values: Vec::new(),
            table_names: Vec::new(),
            padded_height: 0,
            estimated_proving_ns: 0,
        };
        trident::bundle_with_assembly(&entry, &options, assembly, cost)
    }
    .unwrap_or_else(|errors| {
        fail(
            errors
                .iter()
                .map(|e| e.message.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        )
    });

    let (_, mut file) = super::load_and_parse(&entry);
    file.items.retain(|item| {
        let cfg = match &item.node {
            trident::ast::Item::Fn(value) => &value.cfg,
            trident::ast::Item::Const(value) => &value.cfg,
            trident::ast::Item::Struct(value) => &value.cfg,
            trident::ast::Item::Event(value) => &value.cfg,
        };
        cfg.as_ref()
            .is_none_or(|flag| options.cfg_flags.contains(&flag.node))
    });
    let (name, version) = ri
        .project
        .as_ref()
        .map(|p| (p.name.clone(), p.version.clone()))
        .unwrap_or_else(|| {
            (
                entry
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("program")
                    .into(),
                "0.1.0".into(),
            )
        });
    if audit {
        eprintln!("Auditing {}...", entry.display());
        match trident::verify_project_with_options(&entry, &options) {
            Ok(report) if report.is_safe() => eprintln!("Verification: OK"),
            Ok(report) => fail(format!("verification failed\n{}", report.format_report())),
            Err(errors) => fail(
                errors
                    .iter()
                    .map(|e| e.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; "),
            ),
        }
    }
    PreparedArtifact {
        project: ri.project,
        entry,
        tasm: bundle.assembly,
        cost: bundle.cost,
        file,
        name,
        version,
        source_hash: bundle.source_hash,
        resolved,
    }
}
