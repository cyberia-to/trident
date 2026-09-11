// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use super::*;

/// Compute the honest nox reduction cost of a project by lowering the entry
/// module to a Noun formula and walking it. This is the tree-target
/// counterpart of [`analyze_costs_project`], whose AET-table model does not
/// apply to nox (its cost unit is reductions, not trace-table heights).
pub fn nox_cost_project(
    entry_path: &Path,
    options: &CompileOptions,
) -> Result<cost::nox::NoxCost, Vec<Diagnostic>> {
    use crate::ast::FileKind;
    use crate::ir::tree::lower::nox::NoxCompiler;
    use crate::pipeline::PreparedProject;

    // Quiet build: many programs fall outside the nox surface; the caller
    // (e.g. the bench nox column) handles the error without terminal spam.
    let project = PreparedProject::build_quiet(entry_path, options)?;
    let entry_module = project
        .modules
        .iter()
        .find(|pm| pm.file.kind == FileKind::Program)
        .or_else(|| project.modules.first())
        .ok_or_else(|| {
            vec![Diagnostic::error(
                "no entry module found".to_string(),
                span::Span::dummy(),
            )]
        })?;

    let mut compiler = NoxCompiler::new();
    let noun = compiler
        .compile_file(&entry_module.file)
        .map_err(|e| vec![Diagnostic::error(e, span::Span::dummy())])?;
    Ok(cost::nox::NoxCost::analyze(&noun))
}

/// Parse, type-check, and verify a project using symbolic execution + solver.
///
/// Analyzes all functions across all modules, not just `main`.
/// Returns a `VerificationReport` with static analysis, random testing (Schwartz-Zippel),
/// and bounded model checking results.
pub fn verify_project(entry_path: &Path) -> Result<solve::VerificationReport, Vec<Diagnostic>> {
    use crate::pipeline::PreparedProject;

    let project = PreparedProject::build_default(entry_path)?;

    // Collect constraint systems from all functions in all modules
    let mut combined = sym::ConstraintSystem::new();
    for pm in &project.modules {
        for (_, system) in sym::analyze_all(&pm.file) {
            combined.constraints.extend(system.constraints);
            combined.num_variables += system.num_variables;
            for (k, v) in system.variables {
                combined.variables.insert(k, v);
            }
            combined.pub_inputs.extend(system.pub_inputs);
            combined.pub_outputs.extend(system.pub_outputs);
            combined.divine_inputs.extend(system.divine_inputs);
        }
    }

    Ok(solve::verify(&combined))
}

/// Verify all functions in a project, returning per-function results.
///
/// Each entry in the returned vec is `(module_name, fn_name, report)`.
pub fn verify_project_per_function(
    entry_path: &Path,
) -> Result<Vec<(String, String, solve::VerificationReport)>, Vec<Diagnostic>> {
    use crate::pipeline::PreparedProject;

    let project = PreparedProject::build_default(entry_path)?;

    let mut results = Vec::new();
    for pm in &project.modules {
        let module_name = pm.file.name.node.clone();
        for (fn_name, system) in sym::analyze_all(&pm.file) {
            let report = solve::verify(&system);
            results.push((module_name.clone(), fn_name, report));
        }
    }

    Ok(results)
}

/// Format Trident source code, preserving comments.
pub fn format_source(source: &str, _filename: &str) -> Result<String, Vec<Diagnostic>> {
    let (tokens, comments, lex_errors) = lexer::Lexer::new(source, 0).tokenize();
    if !lex_errors.is_empty() {
        return Err(lex_errors);
    }
    let file = parser::Parser::new(tokens).parse_file()?;
    Ok(format::format_file(&file, &comments))
}

/// Type-check only, without rendering diagnostics to stderr.
/// Used by the LSP server to get structured errors.
pub fn check_silent(source: &str, filename: &str) -> Result<(), Vec<Diagnostic>> {
    let file = crate::parse_source_silent(source, filename)?;
    TypeChecker::new().check_file(&file)?;
    Ok(())
}

/// Project-aware type-check for the LSP.
/// Finds trident.toml, resolves dependencies, and type-checks
/// the given file with full module context.
/// Falls back to single-file check if no project is found.
pub fn check_file_in_project(source: &str, file_path: &Path) -> Result<(), Vec<Diagnostic>> {
    let dir = file_path.parent().unwrap_or(Path::new("."));
    let entry = match project::Project::find(dir) {
        Some(toml_path) => match project::Project::load(&toml_path) {
            Ok(p) => p.entry,
            Err(_) => file_path.to_path_buf(),
        },
        None => file_path.to_path_buf(),
    };

    // Resolve all modules from the entry point (handles std.* even without project)
    let modules = match resolve_modules(&entry) {
        Ok(m) => m,
        Err(_) => return check_silent(source, &file_path.to_string_lossy()),
    };

    // Parse and type-check all modules in dependency order
    let mut all_exports: Vec<ModuleExports> = Vec::new();
    let file_path_canon = file_path
        .canonicalize()
        .unwrap_or_else(|_| file_path.to_path_buf());

    for module in &modules {
        let mod_path_canon = module
            .file_path
            .canonicalize()
            .unwrap_or_else(|_| module.file_path.clone());
        let is_target = mod_path_canon == file_path_canon;

        // Use live buffer for the file being edited
        let src = if is_target { source } else { &module.source };
        let parsed = crate::parse_source_silent(src, &module.file_path.to_string_lossy())?;

        let mut tc = TypeChecker::new();
        for exports in &all_exports {
            tc.import_module(exports);
        }

        match tc.check_file(&parsed) {
            Ok(exports) => {
                all_exports.push(exports);
            }
            Err(errors) => {
                if is_target {
                    return Err(errors);
                }
                // Dep has errors — stop, but don't report
                // dep errors as if they're in this file
                return Ok(());
            }
        }
    }

    Ok(())
}
