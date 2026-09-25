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
    use crate::pipeline::PreparedProject;

    // Quiet build: many programs fall outside the nox surface; the caller
    // (e.g. the bench nox column) handles the error without terminal spam.
    let project = PreparedProject::build_quiet(entry_path, options)?;
    let (noun, _) = project.lower_nox(options)?;
    Ok(cost::nox::NoxCost::analyze(&noun))
}

/// Parse, type-check, and verify a project using symbolic execution + solver.
///
/// Analyzes all functions across all modules, not just `main`.
/// Returns a `VerificationReport` with static analysis, random testing (Schwartz-Zippel),
/// and bounded model checking results.
pub fn verify_project(entry_path: &Path) -> Result<solve::VerificationReport, Vec<Diagnostic>> {
    let (_, options) = options_for_project(entry_path)?;
    verify_project_with_options(entry_path, &options)
}

pub fn verify_project_with_options(
    entry_path: &Path,
    options: &CompileOptions,
) -> Result<solve::VerificationReport, Vec<Diagnostic>> {
    sym::validate_audit_target(&options.target_config)
        .map_err(|e| vec![Diagnostic::error(e, crate::span::Span::dummy())])?;
    let project = crate::pipeline::PreparedProject::build_quiet(entry_path, options)?;

    // Collect constraint systems from all functions in all modules
    let mut combined = sym::ConstraintSystem::new();
    for pm in &project.modules {
        for (_, system) in
            sym::analyze_all_with_target(&active_file(&pm.file, options), &options.target_config)
                .map_err(|e| vec![Diagnostic::error(e, crate::span::Span::dummy())])?
        {
            let namespace = format!("function_{}", combined.num_variables);
            combined.append_independent(system, &namespace);
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
    let (_, options) = options_for_project(entry_path)?;
    verify_project_per_function_with_options(entry_path, &options)
}

pub fn verify_project_per_function_with_options(
    entry_path: &Path,
    options: &CompileOptions,
) -> Result<Vec<(String, String, solve::VerificationReport)>, Vec<Diagnostic>> {
    sym::validate_audit_target(&options.target_config)
        .map_err(|e| vec![Diagnostic::error(e, crate::span::Span::dummy())])?;
    let project = crate::pipeline::PreparedProject::build_quiet(entry_path, options)?;

    let mut results = Vec::new();
    for pm in &project.modules {
        let module_name = pm.file.name.node.clone();
        for (fn_name, system) in
            sym::analyze_all_with_target(&active_file(&pm.file, options), &options.target_config)
                .map_err(|e| vec![Diagnostic::error(e, crate::span::Span::dummy())])?
        {
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
    let options = CompileOptions::resolve("nox", "debug").map_err(|e| vec![e])?;
    super::pipeline::PreparedProject::source(file, source, filename, &options)?;
    Ok(())
}

/// Project-aware type-check for the LSP.
/// Finds trident.toml, resolves dependencies, and type-checks
/// the given file with full module context.
/// Falls back to single-file check if no project is found.
pub fn check_file_in_project(source: &str, file_path: &Path) -> Result<(), Vec<Diagnostic>> {
    let (entry, options) = options_for_project(file_path)?;
    let mut modules = crate::resolve::resolve_modules_with_overlay(
        &entry,
        options.dep_dirs.clone(),
        options.library_sources(),
        file_path,
        source,
    )?;

    // An open module need not be reachable from the project's current entry.
    if !modules.iter().any(|m| {
        m.file_path
            .canonicalize()
            .unwrap_or_else(|_| m.file_path.clone())
            == file_path
                .canonicalize()
                .unwrap_or_else(|_| file_path.to_path_buf())
    }) {
        modules = crate::resolve::resolve_modules_with_overlay(
            file_path,
            options.dep_dirs.clone(),
            options.library_sources(),
            file_path,
            source,
        )?;
    }

    // Parse and type-check all modules in dependency order.
    let mut concrete_modules = Vec::new();
    let mut all_exports: Vec<ModuleExports> = Vec::new();
    let file_path_canon = file_path
        .canonicalize()
        .unwrap_or_else(|_| file_path.to_path_buf());

    let files: Vec<_> = modules.iter().map(|m| &m.file).collect();
    let scopes = crate::resolve::scope::scopes(&files)?;
    for (module, scope) in modules.iter().zip(&scopes) {
        let mod_path_canon = module
            .file_path
            .canonicalize()
            .unwrap_or_else(|_| module.file_path.clone());
        let is_target = mod_path_canon == file_path_canon;

        // Use live buffer for the file being edited
        let src = if is_target { source } else { &module.source };
        let parsed = module.file.clone();

        concrete_modules.push(super::pipeline::ParsedModule {
            file_path: module.file_path.clone(),
            source: src.to_string(),
            file: parsed.clone(),
        });
        let mut tc = options.checker();
        tc.import_scope(scope, &all_exports)?;

        match tc.check_file(&parsed) {
            Ok(exports) => {
                all_exports.push(exports);
            }
            Err(errors) => {
                if is_target {
                    return Err(errors);
                }
                return Err(errors
                    .into_iter()
                    .map(|error| {
                        Diagnostic::error(
                            format!("dependency {}: {}", module.name, error.message),
                            span::Span::dummy(),
                        )
                    })
                    .collect());
            }
        }
    }

    super::pipeline::PreparedProject::check_and_specialize(
        &mut concrete_modules,
        &options,
        false,
        true,
    )?;
    Ok(())
}

/// Shared project context for checking, editor tools and static verification.
/// Invalid manifests, owner packages and lockfiles remain visible diagnostics.
pub(crate) fn options_for_project(
    file_path: &Path,
) -> Result<(std::path::PathBuf, CompileOptions), Vec<Diagnostic>> {
    let dir = if file_path.is_dir() {
        file_path
    } else {
        file_path.parent().unwrap_or(Path::new("."))
    };
    let project = project::Project::find(dir)
        .map(|path| project::Project::load(&path))
        .transpose()
        .map_err(|e| vec![e])?;
    let target = project
        .as_ref()
        .and_then(|p| p.target.as_deref())
        .unwrap_or("nox");
    let options = CompileOptions::resolve(target, "debug").map_err(|e| vec![e])?;
    let entry = project
        .map(|p| p.entry)
        .unwrap_or_else(|| file_path.to_path_buf());
    let (_, options) = crate::source_options(file_path, &options)?;
    Ok((entry, options))
}

#[cfg(test)]
mod editor_tests {
    use super::*;

    fn project_dir(name: &str) -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(format!("trident-editor-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn live_buffer_imports_and_default_nox_abi_are_checked() {
        let root = project_dir("overlay");
        let file = root.join("main.tri");
        std::fs::write(&file, "program editor\nfn main() {}\n").unwrap();
        std::fs::write(
            root.join("dep.tri"),
            "module dep\npub fn value() -> Field { 7 }\n",
        )
        .unwrap();
        assert!(check_file_in_project(
            "program editor\nuse dep\nfn main() -> Field { dep.value() }\n",
            &file
        )
        .is_ok());
        assert!(check_file_in_project(
            "program editor\nfn main() -> Digest { pub_read5() }\n",
            &file
        )
        .is_err());
        assert!(
            check_file_in_project("program editor\nuse missing_dep\nfn main() {}\n", &file)
                .is_err()
        );
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "program editor\nfn main() {}\n"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_flags_dependencies_and_unreachable_live_module_share_context() {
        let root = project_dir("context");
        let dep = root.join("vendor");
        std::fs::create_dir_all(&dep).unwrap();
        std::fs::write(root.join("trident.toml"), "[project]\nname = \"editor\"\ntarget = \"nox\"\n[targets.debug]\nflags = [\"special\"]\n[dependencies]\nvendor = { path = \"vendor\" }\n").unwrap();
        std::fs::write(root.join("main.tri"), "program editor\nfn main() {}\n").unwrap();
        std::fs::write(
            dep.join("helper.tri"),
            "module helper\n#[cfg(special)]\npub fn value() -> Field { 7 }\n",
        )
        .unwrap();
        let file = root.join("other.tri");
        let source = "module other\nuse helper\npub fn value() -> Field { helper.value() }\n";
        // This editor-only file does not exist on disk or in the entry graph.
        assert!(check_file_in_project(source, &file).is_ok());
        let (_, options) = options_for_project(&file).unwrap();
        assert!(options.cfg_flags.contains("special"));
        assert!(options.dep_dirs.contains(&dep));
        std::fs::write(
            dep.join("helper.tri"),
            "module helper\npub fn value() -> Field { unknown() }\n",
        )
        .unwrap();
        let errors = check_file_in_project(source, &file).unwrap_err();
        assert!(errors[0].message.contains("dependency helper"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn symbolic_verification_skips_inactive_functions() {
        let root = project_dir("verification-cfg");
        let file = root.join("main.tri");
        std::fs::write(&file, "module editor\n#[cfg(release)]\npub fn inactive() { assert(false) }\npub fn active() {}\n").unwrap();
        let reports =
            verify_project_per_function_with_options(&file, &CompileOptions::default()).unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].1, "active");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_project_target_never_falls_back_to_default_checker() {
        let root = project_dir("invalid-target");
        let file = root.join("main.tri");
        std::fs::write(&file, "program editor\nfn main() {}\n").unwrap();
        std::fs::write(
            root.join("trident.toml"),
            "[project]\nname = \"editor\"\ntarget = \"missing_terrain\"\n",
        )
        .unwrap();
        assert!(check_file_in_project("program editor\nfn main() {}\n", &file).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// Static verification must analyze the same cfg-selected declarations as checking.
fn active_file(file: &ast::File, options: &CompileOptions) -> ast::File {
    let mut file = file.clone();
    file.items.retain(|item| {
        let cfg = match &item.node {
            ast::Item::Fn(value) => &value.cfg,
            ast::Item::Const(value) => &value.cfg,
            ast::Item::Struct(value) => &value.cfg,
            ast::Item::Event(value) => &value.cfg,
        };
        cfg.as_ref()
            .is_none_or(|flag| options.cfg_flags.contains(&flag.node))
    });
    file
}

#[cfg(test)]
mod audit_target_tests {
    use super::*;

    #[test]
    fn public_audit_apis_reject_non_goldilocks_before_analyzing_source() {
        let mut options = CompileOptions::default();
        options.target_config.field_prime = "17".into();
        let input = Path::new("not-read-for-an-unsupported-field.tri");
        let errors = verify_project_with_options(input, &options).unwrap_err();
        assert!(errors[0].message.contains("Goldilocks"));
        let errors = verify_project_per_function_with_options(input, &options).unwrap_err();
        assert!(errors[0].message.contains("Goldilocks"));
    }
}
