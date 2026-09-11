// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
pub(crate) use std::collections::BTreeSet;
pub(crate) use std::path::Path;

pub(crate) use crate::ast::{self, FileKind};
pub(crate) use crate::cost;
pub(crate) use crate::diagnostic::{render_diagnostics, Diagnostic};
use crate::ir::tree::lower::nox::NoxCompiler;
pub(crate) use crate::span;
pub(crate) use crate::target::{Arch, TerrainConfig};
pub(crate) use crate::tir::builder::TIRBuilder;
pub(crate) use crate::tir::optimize::optimize as optimize_tir;
pub(crate) use crate::typecheck::{ModuleExports, TypeChecker};
pub(crate) use crate::{format, lexer, parser, project, solve, sym};

mod test;
pub use test::{discover_tests, run_tests, TestResult};

#[cfg(test)]
mod tests;

/// Options controlling compilation: VM target + conditional compilation flags.
#[derive(Clone, Debug)]
pub struct CompileOptions {
    /// Profile name for cfg flags (e.g. "debug", "release").
    pub profile: String,
    /// Active cfg flags for conditional compilation.
    pub cfg_flags: BTreeSet<String>,
    /// Target VM configuration.
    pub target_config: TerrainConfig,
    /// Additional module search directories (from locked dependencies).
    pub dep_dirs: Vec<std::path::PathBuf>,
    /// Version-matched modules supplied by a warrior, keyed by dotted module name.
    pub module_sources: std::collections::BTreeMap<String, String>,
    /// Validated owner package; carries capability and compilation identity.
    pub target_package: Option<crate::target::TargetPackage>,
}

impl Default for CompileOptions {
    /// Defaults to **nox** — the same terrain the CLI defaults to since
    /// 0.2.0, and the one the stack proves on (joy + zheng). A caller who
    /// wants a foreign engine names it: `options.target_config =
    /// TerrainConfig::triton()`, or resolve one from `vm/<engine>/target.toml`.
    fn default() -> Self {
        Self {
            profile: "debug".to_string(),
            cfg_flags: BTreeSet::from(["debug".to_string()]),
            target_config: TerrainConfig::nox(),
            dep_dirs: Vec::new(),
            module_sources: std::collections::BTreeMap::new(),
            target_package: None,
        }
    }
}

impl CompileOptions {
    pub(crate) fn validate(&self) -> Result<(), Vec<Diagnostic>> {
        if self.target_config.name == "nox" && self.target_config != TerrainConfig::nox() {
            return Err(vec![Diagnostic::error(
                "the nox backend requires the canonical nox ABI".into(),
                span::Span::dummy(),
            )]);
        }
        if let Some(package) = &self.target_package {
            package
                .validate()
                .map_err(|e| vec![Diagnostic::error(e, span::Span::dummy())])?;
            if package.terrain != self.target_config {
                return Err(vec![Diagnostic::error(
                    "target ABI differs from selected package".into(),
                    span::Span::dummy(),
                )]);
            }
        }
        Ok(())
    }

    pub fn resolve(target: &str, profile: &str) -> Result<Self, Diagnostic> {
        let options = Self::for_profile(profile);
        if target == "nox" {
            return Ok(options);
        }
        let package = crate::target::TargetPackage::discover(target)?;
        options
            .with_package(package)
            .map_err(|e| Diagnostic::error(e, span::Span::dummy()))
    }

    pub(crate) fn library_sources(&self) -> std::collections::BTreeMap<String, String> {
        let mut sources = crate::resources::intrinsic_modules(&self.target_config);
        sources.extend(self.module_sources.clone());
        sources.insert(
            "std.target".into(),
            crate::resources::target_constants(&self.target_config),
        );
        sources.insert(
            "vm.crypto.hash".into(),
            crate::resources::native_hash(&self.target_config),
        );
        sources.insert("vm.io.io".into(), crate::resources::io(&self.target_config));
        sources
    }

    pub fn with_package(mut self, package: crate::target::TargetPackage) -> Result<Self, String> {
        package.validate()?;
        self.target_config = package.terrain.clone();
        self.module_sources = package.modules.clone();
        self.target_package = Some(package);
        Ok(self)
    }

    pub(crate) fn checker(&self) -> TypeChecker {
        let checker = TypeChecker::with_target(self.target_config.clone())
            .with_cfg_flags(self.cfg_flags.clone());
        match &self.target_package {
            Some(package) => checker.with_intrinsics(&package.intrinsics),
            None => checker,
        }
    }

    /// Create options for a named profile (debug/release/custom).
    ///
    /// Terrain is nox, as in [`Default`].
    pub fn for_profile(profile: &str) -> Self {
        Self {
            profile: profile.to_string(),
            cfg_flags: BTreeSet::from([profile.to_string()]),
            target_config: TerrainConfig::nox(),
            dep_dirs: Vec::new(),
            module_sources: std::collections::BTreeMap::new(),
            target_package: None,
        }
    }

    /// Create options for a named built-in target (backward compat alias).
    pub fn for_target(target: &str) -> Self {
        Self::for_profile(target)
    }
}

/// Compile a single Trident source string for the default terrain (nox).
///
/// Returns a nox formula in bracket notation. Foreign engines use the TIR
/// API and their warrior's lowering.
pub fn compile(source: &str, filename: &str) -> Result<String, Vec<Diagnostic>> {
    compile_with_options(source, filename, &CompileOptions::default())
}

/// A tree-shaped ISA is not necessarily nox. Its opcodes and subject rules
/// may differ, so the nox backend must never stand in for an unknown target.
pub(crate) fn require_nox_target(options: &CompileOptions) -> Result<(), Vec<Diagnostic>> {
    if options.target_config.name == "nox" && options.target_config.architecture == Arch::Tree {
        Ok(())
    } else {
        Err(vec![Diagnostic::error(
            format!(
                "the nox backend cannot compile target '{}'; this target requires its own lowering",
                options.target_config.name
            ),
            span::Span::dummy(),
        )])
    }
}

/// Compile a single Trident source string with options.
/// Nox produces noun formulas. Other targets require their warrior's lowering.
pub fn compile_with_options(
    source: &str,
    filename: &str,
    options: &CompileOptions,
) -> Result<String, Vec<Diagnostic>> {
    options.validate()?;
    let file = crate::parse_source(source, filename)?;

    // Type check
    let exports = match options.checker().check_file(&file) {
        Ok(exports) => exports,
        Err(errors) => {
            render_diagnostics(&errors, filename, source);
            return Err(errors);
        }
    };
    exports.check_entry_requirements(&file, options)?;

    // Tree targets: direct AST → Noun (bypass TIR)
    if options.target_config.architecture == Arch::Tree {
        require_nox_target(options)?;
        let mut compiler = NoxCompiler::new();
        let noun = compiler
            .compile_modules(&[&file], &file, &options.cfg_flags)
            .map_err(|e| vec![Diagnostic::error(e, span::Span::dummy())])?;
        return Ok(format!("{}", noun));
    }

    // Stack targets: the core stops at TIR. Instruction selection and
    // linking are the warrior's (reference/warrior-api.md,
    // .claude/plans/warrior-owns-lowering.md S3) — `build_tir`/
    // `build_tir_modules` still produce the TIR a warrior lowers.
    let _ = (exports.mono_instances, exports.call_resolutions);
    Err(vec![stack_lowering_moved_error(
        &options.target_config.name,
    )])
}

/// The core no longer lowers TIR to any stack target's assembly — that
/// moved to the warrior (trident/.claude/plans/warrior-owns-lowering.md
/// S3). `trident build`/`run`/`prove`/`verify` delegate to the warrior
/// process for stack targets; a caller of this library function directly
/// gets an honest error instead of silently-missing output.
pub(crate) fn stack_lowering_moved_error(target_name: &str) -> Diagnostic {
    Diagnostic::error(
        format!(
            "trident no longer lowers to '{target_name}' assembly in-process — \
             the warrior does (`trident::build_tir`/`build_tir_modules` still \
             produce the TIR it lowers). Use `trident build --target {target_name}` \
             (delegates to the installed warrior), or link the warrior crate and \
             call its own build function directly."
        ),
        span::Span::dummy(),
    )
}

/// Compile a multi-module project from an entry point path.
pub fn compile_project(entry_path: &Path) -> Result<String, Vec<Diagnostic>> {
    compile_project_with_options(entry_path, &CompileOptions::default())
}

/// Compile a multi-module project with options.
/// Nox produces noun formulas. Other targets require their warrior's lowering.
pub fn compile_project_with_options(
    entry_path: &Path,
    options: &CompileOptions,
) -> Result<String, Vec<Diagnostic>> {
    use crate::pipeline::PreparedProject;

    let project = PreparedProject::build(entry_path, options)?;

    // Tree targets: lower resolved module identities directly to nox nouns.
    if options.target_config.architecture == Arch::Tree {
        let (noun, _) = project.lower_nox(options)?;
        return Ok(format!("{}", noun));
    }

    // Stack targets: the core stops at TIR — see build_tir_modules and
    // stack_lowering_moved_error above. The warrior lowers and links.
    let _ = (
        project.intrinsic_map(),
        project.module_aliases(),
        project.external_constants(),
    );
    Err(vec![stack_lowering_moved_error(
        &options.target_config.name,
    )])
}

/// Type-check only (no TASM emission).
pub fn check(source: &str, filename: &str) -> Result<(), Vec<Diagnostic>> {
    let file = crate::parse_source(source, filename)?;

    if let Err(errors) = TypeChecker::new().check_file(&file) {
        render_diagnostics(&errors, filename, source);
        return Err(errors);
    }

    Ok(())
}

/// Project-aware type-check from an entry point path.
/// Resolves all modules (including std.*) and type-checks in dependency order.
pub fn check_project(entry_path: &Path) -> Result<(), Vec<Diagnostic>> {
    use crate::pipeline::PreparedProject;

    let (entry, options) = tools::options_for_project(entry_path)?;
    PreparedProject::build(&entry, &options)?;
    Ok(())
}

pub fn check_project_with_options(
    entry_path: &Path,
    options: &CompileOptions,
) -> Result<(), Vec<Diagnostic>> {
    pipeline::PreparedProject::build(entry_path, options)?;
    Ok(())
}

/// Compile a module and emit TASM for all its functions (no linking, no DCE).
/// Dependencies are resolved and type-checked, but only the target module's
/// TASM is returned. Labels use the raw `__funcname:` format.
pub fn compile_module(
    module_path: &Path,
    options: &CompileOptions,
) -> Result<String, Vec<Diagnostic>> {
    use crate::pipeline::PreparedProject;

    let project = PreparedProject::build(module_path, options)?;

    let _ = (
        project.intrinsic_map(),
        project.module_aliases(),
        project.external_constants(),
    );
    if project.modules.is_empty() {
        return Err(vec![Diagnostic::error(
            "no module found".to_string(),
            span::Span::dummy(),
        )]);
    }
    Err(vec![stack_lowering_moved_error(
        &options.target_config.name,
    )])
}

/// Build TIR (optimized intermediate representation) from a single source file.
///
/// Returns the IR ops before lowering to target assembly. Used by the
/// neural optimizer to analyze and improve the compilation.
pub fn build_tir(
    source: &str,
    filename: &str,
    options: &CompileOptions,
) -> Result<Vec<crate::tir::TIROp>, Vec<Diagnostic>> {
    options.validate()?;
    let file = crate::parse_source(source, filename)?;

    let exports = match options.checker().check_file(&file) {
        Ok(exports) => exports,
        Err(errors) => {
            render_diagnostics(&errors, filename, source);
            return Err(errors);
        }
    };
    exports.check_entry_requirements(&file, options)?;

    let ir = TIRBuilder::new(options.target_config.clone())
        .with_cfg_flags(options.cfg_flags.clone())
        .with_mono_instances(exports.mono_instances)
        .with_call_resolutions(exports.call_resolutions)
        .build_file(&file);
    Ok(optimize_tir(ir))
}

/// One module's TIR, with the metadata a warrior needs to lower and link it.
///
/// `build_tir_project` concatenates modules, which is what the neural
/// optimizer wants; a warrior that emits its own assembly needs the
/// boundaries back — labels are mangled per module and exactly one module
/// is the program (its entry point is the linked program's entry point).
#[derive(Clone, Debug)]
pub struct ModuleTir {
    /// Module name as declared in `program <name>` / `module <name>`.
    pub name: String,
    /// True for the `program` module — the linked program's entry.
    pub is_program: bool,
    /// Typed, optimized TIR for this module alone.
    pub ops: Vec<crate::tir::TIROp>,
}

/// Build per-module TIR for a project — the warrior lowering contract.
///
/// The core stops here for a warrior that owns its target's assembly:
/// it hands over typed, optimized, monomorphized TIR with module
/// boundaries intact, and the warrior turns each module into its ISA and
/// links them. See `reference/warrior-api.md`.
pub fn build_tir_modules(
    entry_path: &Path,
    options: &CompileOptions,
) -> Result<Vec<ModuleTir>, Vec<Diagnostic>> {
    use crate::pipeline::PreparedProject;

    let project = PreparedProject::build(entry_path, options)?;

    let intrinsic_map = project.intrinsic_map();
    let module_aliases = project.module_aliases();
    let external_constants = project.external_constants();

    let mut modules = Vec::new();
    for (i, pm) in project.modules.iter().enumerate() {
        let mono = project
            .exports
            .get(i)
            .map(|e| e.mono_instances.clone())
            .unwrap_or_default();
        let call_res = project
            .exports
            .get(i)
            .map(|e| e.call_resolutions.clone())
            .unwrap_or_default();
        let ir = TIRBuilder::new(options.target_config.clone())
            .with_cfg_flags(options.cfg_flags.clone())
            .with_module_types(&project.modules.iter().map(|m| &m.file).collect::<Vec<_>>())
            .with_intrinsics(intrinsic_map.clone())
            .with_module_aliases(module_aliases.clone())
            .with_constants(external_constants.clone())
            .with_mono_instances(mono)
            .with_call_resolutions(call_res)
            .build_file(&pm.file);
        modules.push(ModuleTir {
            name: pm.file.name.node.clone(),
            is_program: pm.file.kind == FileKind::Program,
            ops: optimize_tir(ir),
        });
    }
    Ok(modules)
}

/// Build TIR from a project entry point with full module resolution.
///
/// Uses the same multi-module pipeline as `compile_project_with_options`
/// but returns combined TIR ops instead of TASM. Required for neural
/// training on files that import other modules (e.g. merkle.tri imports
/// vm.crypto.merkle).
pub fn build_tir_project(
    entry_path: &Path,
    options: &CompileOptions,
) -> Result<Vec<crate::tir::TIROp>, Vec<Diagnostic>> {
    use crate::pipeline::PreparedProject;

    let project = PreparedProject::build(entry_path, options)?;

    let intrinsic_map = project.intrinsic_map();
    let module_aliases = project.module_aliases();
    let external_constants = project.external_constants();

    let mut all_ir = Vec::new();
    for (i, pm) in project.modules.iter().enumerate() {
        let mono = project
            .exports
            .get(i)
            .map(|e| e.mono_instances.clone())
            .unwrap_or_default();
        let call_res = project
            .exports
            .get(i)
            .map(|e| e.call_resolutions.clone())
            .unwrap_or_default();
        let ir = TIRBuilder::new(options.target_config.clone())
            .with_cfg_flags(options.cfg_flags.clone())
            .with_module_types(&project.modules.iter().map(|m| &m.file).collect::<Vec<_>>())
            .with_intrinsics(intrinsic_map.clone())
            .with_module_aliases(module_aliases.clone())
            .with_constants(external_constants.clone())
            .with_mono_instances(mono)
            .with_call_resolutions(call_res)
            .build_file(&pm.file);
        all_ir.extend(optimize_tir(ir));
    }
    Ok(all_ir)
}

pub(crate) mod pipeline;
mod tools;
pub use tools::*;

mod bundle;
pub use bundle::{bundle_with_assembly, compile_to_bundle};
mod source;
pub use source::source_options;
