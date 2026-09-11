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
pub(crate) use crate::resolve::resolve_modules;
pub(crate) use crate::span;
pub(crate) use crate::target::{Arch, TerrainConfig};
pub(crate) use crate::tir::builder::TIRBuilder;
pub(crate) use crate::tir::optimize::optimize as optimize_tir;
pub(crate) use crate::typecheck::{ModuleExports, TypeChecker};
use crate::ir::tree::lower::nox::NoxCompiler;
pub(crate) use crate::{format, lexer, parser, project, solve, sym};

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
        }
    }
}

impl CompileOptions {
    /// Create options for a named profile (debug/release/custom).
    ///
    /// Terrain is nox, as in [`Default`].
    pub fn for_profile(profile: &str) -> Self {
        Self {
            profile: profile.to_string(),
            cfg_flags: BTreeSet::from([profile.to_string()]),
            target_config: TerrainConfig::nox(),
            dep_dirs: Vec::new(),
        }
    }

    /// Create options for a named built-in target (backward compat alias).
    pub fn for_target(target: &str) -> Self {
        Self::for_profile(target)
    }
}

/// Compile a single Trident source string for the default terrain (nox).
///
/// Returns a nox formula in bracket notation. For a stack engine, use
/// [`compile_with_options`] with an explicit `target_config`.
pub fn compile(source: &str, filename: &str) -> Result<String, Vec<Diagnostic>> {
    compile_with_options(source, filename, &CompileOptions::default())
}

/// Compile a single Trident source string with options.
/// Stack targets produce TASM; tree targets (nox) produce noun formulas.
pub fn compile_with_options(
    source: &str,
    filename: &str,
    options: &CompileOptions,
) -> Result<String, Vec<Diagnostic>> {
    let file = crate::parse_source(source, filename)?;

    // Type check
    let exports = match TypeChecker::with_target(options.target_config.clone())
        .with_cfg_flags(options.cfg_flags.clone())
        .check_file(&file)
    {
        Ok(exports) => exports,
        Err(errors) => {
            render_diagnostics(&errors, filename, source);
            return Err(errors);
        }
    };

    // Tree targets: direct AST → Noun (bypass TIR)
    if options.target_config.architecture == Arch::Tree {
        let mut compiler = NoxCompiler::new();
        let noun = compiler.compile_file(&file).map_err(|e| {
            vec![Diagnostic::error(e, span::Span::dummy())]
        })?;
        return Ok(format!("{}", noun));
    }

    // Stack targets: the core stops at TIR. Instruction selection and
    // linking are the warrior's (reference/warrior-api.md,
    // .claude/plans/warrior-owns-lowering.md S3) — `build_tir`/
    // `build_tir_modules` still produce the TIR a warrior lowers.
    let _ = (exports.mono_instances, exports.call_resolutions);
    Err(vec![stack_lowering_moved_error(&options.target_config.name)])
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
/// Stack targets produce linked TASM; tree targets (nox) produce noun formulas.
pub fn compile_project_with_options(
    entry_path: &Path,
    options: &CompileOptions,
) -> Result<String, Vec<Diagnostic>> {
    use crate::pipeline::PreparedProject;

    let project = PreparedProject::build(entry_path, options)?;

    // Tree targets: direct AST → Noun for the entry module (bypass TIR)
    if options.target_config.architecture == Arch::Tree {
        let entry_module = project
            .modules
            .iter()
            .find(|pm| pm.file.kind == FileKind::Program)
            .or_else(|| project.modules.first())
            .ok_or_else(|| vec![Diagnostic::error(
                "no entry module found".to_string(),
                span::Span::dummy(),
            )])?;
        let mut compiler = NoxCompiler::new();
        let noun = compiler.compile_file(&entry_module.file).map_err(|e| {
            vec![Diagnostic::error(e, span::Span::dummy())]
        })?;
        return Ok(format!("{}", noun));
    }

    // Stack targets: the core stops at TIR — see build_tir_modules and
    // stack_lowering_moved_error above. The warrior lowers and links.
    let _ = (project.intrinsic_map(), project.module_aliases(), project.external_constants());
    Err(vec![stack_lowering_moved_error(&options.target_config.name)])
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

    PreparedProject::build_default(entry_path)?;
    Ok(())
}

/// Discover `#[test]` functions in a parsed file.
pub fn discover_tests(file: &ast::File) -> Vec<String> {
    let mut tests = Vec::new();
    for item in &file.items {
        if let ast::Item::Fn(func) = &item.node {
            if func.is_test {
                tests.push(func.name.node.clone());
            }
        }
    }
    tests
}

/// A single test result.
#[derive(Clone, Debug)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
}

/// Run all `#[test]` functions in a project.
///
/// For each test function, we:
/// 1. Parse and type-check the project
/// 2. Compile a mini-program that just calls the test function
/// 3. Report pass/fail with cost summary
pub fn run_tests(
    entry_path: &std::path::Path,
    options: &CompileOptions,
) -> Result<String, Vec<Diagnostic>> {
    use crate::pipeline::PreparedProject;

    let project = PreparedProject::build(entry_path, options)?;

    // Discover all #[test] functions across all modules
    let mut test_fns: Vec<(String, String)> = Vec::new(); // (module_name, fn_name)
    for pm in &project.modules {
        for test_name in discover_tests(&pm.file) {
            test_fns.push((pm.file.name.node.clone(), test_name));
        }
    }

    if test_fns.is_empty() {
        return Ok("No #[test] functions found.\n".to_string());
    }

    // For each test function, compile a mini-program and report
    let mut results: Vec<TestResult> = Vec::new();
    for (module_name, test_name) in &test_fns {
        // Find the source file for this module
        let source_entry = project
            .modules
            .iter()
            .find(|m| m.file.name.node == *module_name);

        if let Some(pm) = source_entry {
            // Build a mini-program source that just calls the test function
            let mini_source = if module_name.starts_with("module") || module_name.contains('.') {
                // For module test functions, we'd need cross-module calls
                // For simplicity, compile in-context
                pm.source.clone()
            } else {
                pm.source.clone()
            };

            // Try to compile (type-check + emit) the source.
            // The test function itself is validated by the type checker.
            // For now, "passing" means it compiles without errors.
            match compile_with_options(&mini_source, &pm.file_path.to_string_lossy(), options) {
                Ok(compiled) => {
                    // Check if the compiled output contains an assert failure marker
                    let has_error = compiled.contains("// ERROR");
                    results.push(TestResult {
                        name: test_name.clone(),
                        passed: !has_error,
                        error: if has_error {
                            Some("compilation produced errors".to_string())
                        } else {
                            None
                        },
                    });
                }
                Err(errors) => {
                    let msg = errors
                        .iter()
                        .map(|d| d.message.clone())
                        .collect::<Vec<_>>()
                        .join("; ");
                    results.push(TestResult {
                        name: test_name.clone(),
                        passed: false,
                        error: Some(msg),
                    });
                }
            }
        }
    }

    // Format the report
    let mut report = String::new();
    let total = results.len();
    let passed = results.iter().filter(|r| r.passed).count();
    let failed = total - passed;

    report.push_str(&format!(
        "running {} test{}\n",
        total,
        if total == 1 { "" } else { "s" }
    ));

    for result in &results {
        let status = if result.passed { "ok" } else { "FAILED" };
        report.push_str(&format!("  test {} ... {}\n", result.name, status));
        if let Some(ref err) = result.error {
            report.push_str(&format!("    error: {}\n", err));
        }
    }

    report.push('\n');
    if failed == 0 {
        report.push_str(&format!("test result: ok. {} passed; 0 failed\n", passed));
    } else {
        report.push_str(&format!(
            "test result: FAILED. {} passed; {} failed\n",
            passed, failed
        ));
    }

    Ok(report)
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

    let _ = (project.intrinsic_map(), project.module_aliases(), project.external_constants());
    if project.modules.is_empty() {
        return Err(vec![Diagnostic::error(
            "no module found".to_string(),
            span::Span::dummy(),
        )]);
    }
    Err(vec![stack_lowering_moved_error(&options.target_config.name)])
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
    let file = crate::parse_source(source, filename)?;

    let exports = match TypeChecker::with_target(options.target_config.clone())
        .with_cfg_flags(options.cfg_flags.clone())
        .check_file(&file)
    {
        Ok(exports) => exports,
        Err(errors) => {
            render_diagnostics(&errors, filename, source);
            return Err(errors);
        }
    };

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

/// Compile a multi-module project to a `ProgramBundle` artifact.
///
/// This is the primary entry point for warriors: it produces a
/// self-contained bundle with compiled assembly, cost analysis,
/// function signatures, and metadata.
pub fn compile_to_bundle(
    entry_path: &Path,
    options: &CompileOptions,
) -> Result<crate::runtime::ProgramBundle, Vec<Diagnostic>> {
    use crate::runtime::artifact::{BundleCost, BundleFunction, ProgramBundle};
    use pipeline::PreparedProject;

    let tasm = compile_project_with_options(entry_path, options)?;

    // Cost: nox prices in reductions (bill.max, an upper bound — see
    // cost::nox::NoxCost); stack targets never reach this line, since
    // compile_project_with_options above already errored for them
    // (the core stops at TIR — reference/warrior-api.md). A warrior
    // that owns its own bundle assembly fills in its own cost model.
    let bundle_cost = match nox_cost_project(entry_path, options) {
        Ok(nc) => BundleCost {
            table_values: vec![nc.bill.max],
            table_names: vec!["reductions".to_string()],
            padded_height: nc.nodes,
            estimated_proving_ns: 0,
        },
        Err(_) => BundleCost {
            table_values: Vec::new(),
            table_names: Vec::new(),
            padded_height: 0,
            estimated_proving_ns: 0,
        },
    };

    // Parse entry file for function signatures + content hashes
    let project = PreparedProject::build(entry_path, options)?;
    let entry_file = project
        .modules
        .iter()
        .find(|m| m.file.kind == FileKind::Program)
        .or_else(|| project.modules.last());

    let (functions, entry_point, source_hash) = if let Some(pm) = entry_file {
        let fn_hashes = crate::hash::hash_file(&pm.file);
        let fns: Vec<BundleFunction> = pm
            .file
            .items
            .iter()
            .filter_map(|item| {
                if let ast::Item::Fn(func) = &item.node {
                    if !func.is_test {
                        let hash = fn_hashes
                            .get(&func.name.node)
                            .map(|h| h.to_hex())
                            .unwrap_or_default();
                        return Some(BundleFunction {
                            name: func.name.node.clone(),
                            hash,
                            signature: crate::deploy::format_fn_signature(func),
                        });
                    }
                }
                None
            })
            .collect();
        let ep = if fns.iter().any(|f| f.name == "main") {
            "main".to_string()
        } else {
            fns.first()
                .map(|f| f.name.clone())
                .unwrap_or_else(|| "main".to_string())
        };
        let sh = crate::hash::hash_file_content(&pm.file).to_hex();
        (fns, ep, sh)
    } else {
        (Vec::new(), "main".to_string(), String::new())
    };

    let name = entry_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("program")
        .to_string();

    // Tree targets: ask the lowering (the source of truth) whether the
    // program reads persistent state, so the bundle can declare it and the
    // runner knows to cons the BBG root onto the subject.
    let reads_state = if options.target_config.architecture == crate::target::Arch::Tree {
        entry_file
            .map(|pm| {
                let mut compiler = NoxCompiler::new();
                let _ = compiler.compile_file(&pm.file);
                compiler.reads_state()
            })
            .unwrap_or(false)
    } else {
        false
    };

    Ok(ProgramBundle {
        name,
        version: "0.1.0".to_string(),
        target_vm: options.target_config.name.clone(),
        target_os: None,
        assembly: tasm,
        entry_point,
        functions,
        cost: bundle_cost,
        source_hash,
        reads_state,
    })
}
