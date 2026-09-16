// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
pub mod audit;
pub mod build;
pub mod check;
pub mod deploy;
pub mod deps;
pub mod fmt;
pub mod generate;
pub mod hash;
pub mod init;
pub mod mir;
pub mod package;
pub mod prove;
pub mod registry;
pub mod run;
pub mod store;
pub mod test;
pub mod tree_sitter;
pub mod verify;
pub mod view;

use std::path::{Path, PathBuf};
use std::process;

// ─── Three-Register Target Resolution ──────────────────────────────

/// Resolved battlefield from three naming registers.
///
/// Users speak geeky (engine/network/vimputer), gamy
/// (terrain/union/state), or universal (target). All resolve
/// to the same battlefield.
pub struct BattlefieldSelection {
    /// Resolved target name (terrain or union name).
    pub target: String,
    /// Explicit state name, if provided.
    pub state: Option<String>,
}

/// Resolve a battlefield from three naming registers.
///
/// Priority: explicit register flags override `--target`.
/// Geeky (engine/network) and gamy (terrain/union) are synonyms.
/// At most one terrain-level and one union-level flag may be set
/// (enforced by clap `conflicts_with`).
pub fn resolve_battlefield(
    target: &str,
    engine: &Option<String>,
    terrain: &Option<String>,
    network: &Option<String>,
    union_flag: &Option<String>,
    vimputer: &Option<String>,
    state: &Option<String>,
) -> BattlefieldSelection {
    // Terrain-level: engine or terrain override target
    let resolved_target = engine
        .as_deref()
        .or(terrain.as_deref())
        .or(network.as_deref())
        .or(union_flag.as_deref())
        .unwrap_or(target)
        .to_string();

    // State-level: vimputer or state
    let resolved_state = vimputer.clone().or_else(|| state.clone());

    BattlefieldSelection {
        target: resolved_target,
        state: resolved_state,
    }
}

/// Resolve a battlefield without state flags (compilation commands).
pub fn resolve_battlefield_compile(
    target: &str,
    engine: &Option<String>,
    terrain: &Option<String>,
    network: &Option<String>,
    union_flag: &Option<String>,
) -> BattlefieldSelection {
    resolve_battlefield(target, engine, terrain, network, union_flag, &None, &None)
}

/// A supplied target wins over the project's selection, including explicit nox.
pub fn source_target(target: Option<&str>, project: Option<&trident::project::Project>) -> String {
    target
        .or_else(|| project.and_then(|p| p.target.as_deref()))
        .unwrap_or("nox")
        .to_owned()
}

// ─── Input Resolution ──────────────────────────────────────────────

/// Resolved input: entry file and optional project.
pub struct ResolvedInput {
    pub entry: PathBuf,
    pub project: Option<trident::project::Project>,
}

fn load_project(toml_path: &Path) -> trident::project::Project {
    match trident::project::Project::load(toml_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e.message);
            process::exit(1);
        }
    }
}

/// Resolve an input path (file or project directory) to an entry file and optional project.
pub fn resolve_input(input: &Path) -> ResolvedInput {
    if input.is_dir() {
        let toml_path = input.join("trident.toml");
        if !toml_path.exists() {
            eprintln!("error: no trident.toml found in '{}'", input.display());
            process::exit(1);
        }
        let project = load_project(&toml_path);
        let entry = project.entry.clone();
        return ResolvedInput {
            entry,
            project: Some(project),
        };
    }

    if !input.extension().is_some_and(|e| e == "tri") {
        eprintln!("error: input must be a .tri file or project directory");
        process::exit(1);
    }

    let toml_path = trident::project::Project::find(input.parent().unwrap_or(Path::new(".")));
    match toml_path {
        Some(p) => {
            let project = load_project(&p);
            // A manifest supplies settings; only a directory input selects its entry.
            let entry = input.to_path_buf();
            ResolvedInput {
                entry,
                project: Some(project),
            }
        }
        None => ResolvedInput {
            entry: input.to_path_buf(),
            project: None,
        },
    }
}

/// Resolve a VM target + profile to CompileOptions.
pub fn resolve_options(
    target: &str,
    profile: &str,
    project: Option<&trident::project::Project>,
) -> trident::CompileOptions {
    // Backward compat: --target debug/release → treat as profile
    let (vm_target, actual_profile) = match target {
        "debug" | "release" => {
            eprintln!(
                "warning: --target {} is deprecated; use --profile {} --target triton",
                target, target
            );
            ("triton", target)
        }
        _ => (target, profile),
    };

    let mut options = match trident::CompileOptions::resolve(vm_target, actual_profile) {
        Ok(options) => options,
        Err(e) => {
            eprintln!("error: {}", e.message);
            process::exit(1);
        }
    };
    if let Some(flags) = project.and_then(|p| p.targets.get(actual_profile)) {
        options.cfg_flags = flags.iter().cloned().collect();
    }
    options
}

mod artifact;
pub use artifact::prepare_artifact;

/// Try to load and parse a .tri file, returning None on error (prints diagnostics).
pub fn try_load_and_parse(path: &Path) -> Option<(String, trident::ast::File)> {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read '{}': {}", path.display(), e);
            return None;
        }
    };
    let filename = path.to_string_lossy().to_string();
    match trident::parse_source_silent(&source, &filename) {
        Ok(f) => Some((source, f)),
        Err(_) => {
            eprintln!("error: parse errors in '{}'", path.display());
            None
        }
    }
}

/// Load and parse a .tri file, exiting on error.
pub fn load_and_parse(path: &Path) -> (String, trident::ast::File) {
    match try_load_and_parse(path) {
        Some(result) => result,
        None => process::exit(1),
    }
}

/// Open the codebase store, exiting on error.
pub fn open_codebase() -> trident::store::Codebase {
    match trident::store::Codebase::open() {
        Ok(cb) => cb,
        Err(e) => {
            eprintln!("error: cannot open codebase: {}", e);
            process::exit(1);
        }
    }
}

/// Create a registry client with health check, exiting on error.
pub fn registry_client(url: Option<String>) -> trident::registry::RegistryClient {
    let url = url.unwrap_or_else(trident::registry::RegistryClient::default_url);
    let client = trident::registry::RegistryClient::new(&url);
    match client.health() {
        Ok(true) => {}
        Ok(false) => {
            eprintln!("error: registry at {} is not healthy", url);
            process::exit(1);
        }
        Err(e) => {
            eprintln!("error: cannot reach registry at {}: {}", url, e);
            process::exit(1);
        }
    }
    client
}

/// Resolve a registry URL to its default if None.
pub fn registry_url(url: Option<String>) -> String {
    url.unwrap_or_else(trident::registry::RegistryClient::default_url)
}

/// Use the same executable for package discovery and runtime dispatch.
pub fn find_warrior(target: &str) -> Option<PathBuf> {
    trident::target::warrior_path(target)
}

pub fn missing_warrior(target: &str, command: &str) -> ! {
    eprintln!("error: cannot {command}: no warrior found for target '{target}'");
    match trident::target::owner_for(target) {
        Some("joy") => eprintln!("Install the nox warrior: cargo install cyber-joy"),
        Some("trisha") => eprintln!(
            "Install the Triton warrior from https://github.com/cyberia-to/trisha/releases"
        ),
        _ => {}
    }
    process::exit(1)
}

/// Delegate a CLI command; capability or child failures return a nonzero exit.
pub fn delegate_to_warrior(warrior_bin: &Path, command: &str, extra_args: &[&str]) {
    if let Err(error) = try_delegate_to_warrior(warrior_bin, command, extra_args) {
        eprintln!("error: {error}");
        process::exit(1);
    }
}

/// Fallible dispatch lets callers clean temporary artifacts before returning errors.
pub fn try_delegate_to_warrior(
    warrior_bin: &Path,
    command: &str,
    extra_args: &[&str],
) -> Result<(), String> {
    let target = extra_args
        .windows(2)
        .find(|pair| pair[0] == "--target")
        .map(|pair| pair[1])
        .unwrap_or("nox");
    let package = trident::target::TargetPackage::discover(target).map_err(|e| e.message)?;
    package.require_command(command)?;
    if std::env::var_os("TRIDENT_TARGET_PACKAGES").is_some() {
        let installed =
            trident::target::TargetPackage::discover_installed(target).map_err(|e| e.message)?;
        installed.require_command(command)?;
        if installed.compilation_hash()? != package.compilation_hash()? {
            return Err(format!(
                "locked target package differs from installed provider for '{target}'"
            ));
        }
    }
    let status = std::process::Command::new(warrior_bin)
        .arg(command)
        .args(extra_args)
        .status()
        .map_err(|e| format!("cannot run warrior '{}': {e}", warrior_bin.display()))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "warrior '{}' {command} failed: {status}",
            warrior_bin.display()
        ))
    }
}

pub fn find_program_source(input: &Path) -> Option<PathBuf> {
    if input.is_file() && input.extension().is_some_and(|e| e == "tri") {
        return Some(input.to_path_buf());
    }
    if input.is_dir() {
        let main_tri = input.join("main.tri");
        if main_tri.exists() {
            return Some(main_tri);
        }
    }
    None
}

/// Truncate a hash string to a short prefix for display.
pub fn short_hash(hash: &str) -> &str {
    &hash[..hash.len().min(16)]
}

/// Resolve an input path to a list of .tri files (file or directory), exiting on error.
pub fn resolve_tri_files(input: &Path) -> Vec<PathBuf> {
    if input.is_dir() {
        collect_tri_files(input)
    } else if input.extension().is_some_and(|e| e == "tri") {
        vec![input.to_path_buf()]
    } else {
        eprintln!("error: input must be a .tri file or directory");
        process::exit(1);
    }
}

fn collect_tri_files(dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    collect_tri_files_recursive(dir, &mut result, 0);
    result.sort();
    result
}

const MAX_DIR_DEPTH: usize = 64;

fn collect_tri_files_recursive(dir: &Path, result: &mut Vec<PathBuf>, depth: usize) {
    if depth >= MAX_DIR_DEPTH {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip hidden directories and target/
        if name_str.starts_with('.') || name_str == "target" {
            continue;
        }

        if path.is_dir() {
            collect_tri_files_recursive(&path, result, depth + 1);
        } else if path.extension().is_some_and(|e| e == "tri") {
            result.push(path);
        }
    }
}
