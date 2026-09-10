// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use std::path::Path;

use crate::diagnostic::Diagnostic;
use crate::target::TerrainConfig;
use crate::CompileOptions;

/// Compile a source string for the **Triton** terrain.
///
/// These suites assert on TASM, so they name the engine. `compile()` and
/// `compile_project()` follow the CLI and default to nox; a test that
/// wants stack assembly says so, and keeps saying so when the Triton
/// lowering moves to trisha.
pub(crate) fn compile_triton(source: &str, filename: &str) -> Result<String, Vec<Diagnostic>> {
    let mut options = CompileOptions::default();
    options.target_config = TerrainConfig::triton();
    crate::compile_with_options(source, filename, &options)
}

/// Options for the Triton terrain at a named profile — see
/// [`compile_triton`].
pub(crate) fn triton_options(profile: &str) -> CompileOptions {
    let mut options = CompileOptions::for_profile(profile);
    options.target_config = TerrainConfig::triton();
    options
}

/// `compile_project` for the Triton terrain — see [`compile_triton`].
pub(crate) fn compile_project_triton(entry: &Path) -> Result<String, Vec<Diagnostic>> {
    let mut options = CompileOptions::default();
    options.target_config = TerrainConfig::triton();
    crate::compile_project_with_options(entry, &options)
}

mod check;
mod compile;
mod cost;
mod docs;
mod features;
mod format;
mod neptune;
mod prove;
