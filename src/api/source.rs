use std::path::{Path, PathBuf};

use super::{CompileOptions, Diagnostic};

/// Resolve source/project input and apply its named profile and dependency paths.
/// Target selection remains the caller's decision, so a warrior cannot inherit another VM.
pub fn source_options(
    input: &Path,
    options: &CompileOptions,
) -> Result<(PathBuf, CompileOptions), Vec<Diagnostic>> {
    let mut options = options.clone();
    let manifest = if input.is_dir() {
        Some(input.join("trident.toml"))
    } else {
        crate::project::Project::find(input.parent().unwrap_or(Path::new(".")))
    };
    let Some(manifest) = manifest else {
        return Ok((input.to_path_buf(), options));
    };
    let project = crate::project::Project::load(&manifest).map_err(|e| vec![e])?;
    if let Some(flags) = project.targets.get(&options.profile) {
        options.cfg_flags = flags.iter().cloned().collect();
    }
    let lock = project.root_dir.join("trident.lock");
    if lock.is_file() {
        let lock = crate::manifest::load_lockfile(&lock)
            .map_err(|error| vec![Diagnostic::error(error, crate::span::Span::dummy())])?;
        options
            .dep_dirs
            .extend(crate::manifest::dependency_search_paths(
                &project.root_dir,
                &lock,
            ));
    }
    for dep in project.dependencies.dependencies.values() {
        if let crate::manifest::Dependency::Path { path } = dep {
            options.dep_dirs.push(project.root_dir.join(path));
        }
    }
    options.dep_dirs.sort();
    options.dep_dirs.dedup();
    Ok((
        if input.is_dir() {
            project.entry
        } else {
            input.to_path_buf()
        },
        options,
    ))
}
