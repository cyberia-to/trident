// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
pub(crate) use std::collections::BTreeSet;
pub(crate) use std::path::{Path, PathBuf};

pub(crate) use crate::diagnostic::Diagnostic;
pub(crate) use crate::span::Span;

/// Information about a discovered module.
#[derive(Clone, Debug)]
pub(crate) struct ModuleInfo {
    /// Dotted module name (e.g. "crypto.sponge").
    pub(crate) name: String,
    /// Filesystem path to the .tri file.
    pub(crate) file_path: PathBuf,
    /// Source code and its parser-owned declaration/import metadata.
    pub(crate) source: String,
    pub(crate) file: crate::ast::File,
    /// Modules this module depends on (from `use` statements).
    pub(crate) dependencies: Vec<String>,
}

/// Resolve all modules reachable from an entry point.
/// Returns modules in topological order (dependencies first).
mod names;
pub(crate) mod scope;
mod visibility;
/// Canonical import owner used by discovery and direct import scopes.
pub use names::canonical_module_name;
mod resolver;
use resolver::*;

#[cfg(test)]
pub(crate) fn resolve_modules(entry_path: &Path) -> Result<Vec<ModuleInfo>, Vec<Diagnostic>> {
    let mut resolver = ModuleResolver::new(entry_path)?;
    resolver.discover_all()?;
    resolver.topological_sort()
}

pub(crate) fn resolve_modules_with_sources(
    entry_path: &Path,
    dep_dirs: Vec<PathBuf>,
    sources: std::collections::BTreeMap<String, String>,
) -> Result<Vec<ModuleInfo>, Vec<Diagnostic>> {
    let mut resolver = ModuleResolver::new(entry_path)?;
    resolver.dep_dirs = dep_dirs;
    resolver.sources = sources;
    resolver.discover_all()?;
    resolver.topological_sort()
}

/// Source locations for navigation only. Compiler-owned bytes are embedded;
/// custom libraries must be explicit dependencies, never ambient environment.
fn find_lib_dir(namespace: &str) -> Option<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("lib")
        .join(namespace);
    path.is_dir().then_some(path)
}

pub(crate) fn find_stdlib_dir() -> Option<PathBuf> {
    find_lib_dir("std")
}
pub(crate) fn find_os_dir() -> Option<PathBuf> {
    find_lib_dir("os")
}

/// Resolve live editor bytes before scanning imports, without writing to disk.
pub(crate) fn resolve_modules_with_overlay(
    entry_path: &Path,
    dep_dirs: Vec<PathBuf>,
    sources: std::collections::BTreeMap<String, String>,
    overlay_path: &Path,
    overlay_source: &str,
) -> Result<Vec<ModuleInfo>, Vec<Diagnostic>> {
    let mut resolver =
        ModuleResolver::with_overlay(entry_path, Some((overlay_path, overlay_source)))?;
    resolver.dep_dirs = dep_dirs;
    resolver.sources = sources;
    resolver.discover_all()?;
    resolver.topological_sort()
}

/// Legacy flat-path fallback map for backward compatibility.
/// Maps old module names to their new layered locations.
fn legacy_stdlib_fallback(name: &str) -> Option<&'static str> {
    match name {
        // Legacy flat std.* → vm.* or std.* (final destination)
        "std.assert" => Some("vm.core.assert"),
        "std.convert" => Some("vm.core.convert"),
        "std.field" => Some("vm.core.field"),
        "std.u32" => Some("vm.core.u32"),
        "std.io" => Some("vm.io.io"),
        "std.mem" => Some("vm.io.mem"),
        "std.storage" => Some("std.io.storage"),
        "std.hash" => Some("vm.crypto.hash"),
        "std.merkle" => Some("std.crypto.merkle"),
        "std.auth" => Some("std.crypto.auth"),
        // Legacy std.* intrinsics → vm.* (intrinsics moved out of std)
        "std.core.field" => Some("vm.core.field"),
        "std.core.convert" => Some("vm.core.convert"),
        "std.core.u32" => Some("vm.core.u32"),
        "std.core.assert" => Some("vm.core.assert"),
        "std.io.io" => Some("vm.io.io"),
        "std.io.mem" => Some("vm.io.mem"),
        "std.crypto.hash" => Some("vm.crypto.hash"),
        // Legacy std.xfield/kernel/utxo → os.neptune.*
        #[cfg(feature = "external-targets")]
        "std.xfield" => Some("os.neptune.xfield"),
        #[cfg(feature = "external-targets")]
        "std.kernel" => Some("os.neptune.kernel"),
        #[cfg(feature = "external-targets")]
        "std.utxo" => Some("os.neptune.utxo"),
        // Backward compatibility: ext.triton.* → os.neptune.*
        #[cfg(feature = "external-targets")]
        "ext.triton.xfield" => Some("os.neptune.xfield"),
        #[cfg(feature = "external-targets")]
        "ext.triton.kernel" => Some("os.neptune.kernel"),
        #[cfg(feature = "external-targets")]
        "ext.triton.utxo" => Some("os.neptune.utxo"),
        #[cfg(feature = "external-targets")]
        "ext.triton.proof" => Some("os.neptune.proof"),
        #[cfg(feature = "external-targets")]
        "ext.triton.recursive" => Some("os.neptune.recursive"),

        // Backward compatibility: <os>.ext.* → os.<os>.*
        #[cfg(feature = "external-targets")]
        "neptune.ext.kernel" => Some("os.neptune.kernel"),
        #[cfg(feature = "external-targets")]
        "neptune.ext.utxo" => Some("os.neptune.utxo"),
        #[cfg(feature = "external-targets")]
        "neptune.ext.xfield" => Some("os.neptune.xfield"),
        #[cfg(feature = "external-targets")]
        "neptune.ext.proof" => Some("os.neptune.proof"),
        #[cfg(feature = "external-targets")]
        "neptune.ext.recursive" => Some("os.neptune.recursive"),

        // Backward compatibility: ext.<os>.* → os.<os>.*
        _ if name.starts_with("ext.") => {
            None // handled by resolve_path directly
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests;
