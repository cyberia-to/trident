//! Execute project tests on nox; compilation alone never counts as a pass.

use std::path::Path;

use nebu::Goldilocks;
use nox::{NoTrace, NullCalls, Outcome, Reduction};

use crate::ast;
use crate::diagnostic::Diagnostic;
use crate::ir::tree::lower::{nox::NoxCompiler, Noun};
use crate::pipeline::PreparedProject;
use crate::span::Span;
use crate::CompileOptions;

const ARENA: usize = 1 << 18;
const STACK_SIZE: usize = 256 * 1024 * 1024;
const TEST_BUDGET: u64 = 1_000_000;

/// Discover syntactically annotated tests. Execution additionally filters cfg.
pub fn discover_tests(file: &ast::File) -> Vec<String> {
    file.items
        .iter()
        .filter_map(|item| match &item.node {
            ast::Item::Fn(f) if f.is_test => Some(f.name.node.clone()),
            _ => None,
        })
        .collect()
}

// Discovery is syntactic; execution selects each name's final active annotation.
fn selected_tests<'a>(
    file: &'a ast::File,
    flags: &std::collections::BTreeSet<String>,
) -> (Vec<&'a ast::FnDef>, usize) {
    let skipped = file
        .items
        .iter()
        .filter(|item| {
            matches!(&item.node, ast::Item::Fn(f)
        if f.is_test && f.cfg.as_ref().is_some_and(|flag| !flags.contains(&flag.node)))
        })
        .count();
    let functions = file
        .final_functions(flags)
        .into_iter()
        .filter(|f| f.is_test)
        .collect();
    (functions, skipped)
}

/// A single executed test result.
#[derive(Clone, Debug)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
}

/// Execute active `#[test]` functions with isolated arenas and bounded budgets.
///
/// Imports and profile flags follow normal project preparation; `test` is also
/// active. Each selected function is the entry, independent of `main`. Every
/// compile or execution failure makes the returned result an error containing
/// the complete report. Non-nox tests require a warrior test implementation.
pub fn run_tests(entry_path: &Path, options: &CompileOptions) -> Result<String, Vec<Diagnostic>> {
    if options.target_config.name != "nox" {
        return Err(vec![Diagnostic::error(
            format!("test execution for '{}' is unsupported: execution belongs to the target warrior; use prepare_test_programs or trident test", options.target_config.name),
            Span::dummy(),
        )]);
    }
    let mut options = options.clone();
    options.cfg_flags.insert("test".to_string());
    let project = PreparedProject::build_tests(entry_path, &options)?;
    let files: Vec<_> = project.modules.iter().map(|m| &m.file).collect();
    let mut results = Vec::new();
    let mut skipped = 0;
    for (module_index, module) in project.modules.iter().enumerate() {
        let (functions, excluded) = selected_tests(&module.file, &options.cfg_flags);
        skipped += excluded;
        for function in functions {
            let name = format!("{}.{}", module.file.name.node, function.name.node);
            let result = if !function.params.is_empty()
                || !function.type_params.is_empty()
                || function.return_ty.is_some()
                || function.body.is_none()
            {
                Err(
                    "tests require a body, no parameters, no generics and no return type"
                        .to_string(),
                )
            } else {
                // Entry selection uses a separate view. All actual module
                // definitions (including main and private helpers) stay intact.
                let mut selected = function.clone();
                selected.is_pub = true;
                let selected =
                    crate::span::Spanned::new(ast::Item::Fn(selected), function.name.span);
                let mut entry = module.file.clone();
                entry.kind = ast::FileKind::Module;
                entry.items = vec![selected];
                project.exports[module_index].check_entry_requirements(&entry, &options)?;
                let mut compiler = NoxCompiler::new();
                compiler
                    .compile_modules(&files, &entry, &options.cfg_flags)
                    .and_then(execute)
            };
            results.push(TestResult {
                name,
                passed: result.is_ok(),
                error: result.err(),
            });
        }
    }
    if results.is_empty() {
        return Ok(format!(
            "No active #[test] functions found. {skipped} skipped.\n"
        ));
    }
    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.len() - passed;
    let mut report = format!(
        "running {} test{} on nox\n",
        results.len(),
        if results.len() == 1 { "" } else { "s" }
    );
    for result in &results {
        report.push_str(&format!(
            "  test {} ... {}\n",
            result.name,
            if result.passed { "ok" } else { "FAILED" }
        ));
        if let Some(error) = &result.error {
            report.push_str(&format!("    error: {error}\n"));
        }
    }
    report.push_str(&format!(
        "\ntest result: {}. {passed} passed; {failed} failed; {skipped} skipped\n",
        if failed == 0 { "ok" } else { "FAILED" }
    ));
    if failed == 0 {
        Ok(report)
    } else {
        Err(vec![Diagnostic::error(report, Span::dummy())])
    }
}

fn load(arena: &mut Reduction<ARENA>, noun: &Noun) -> Result<nox::Order, String> {
    let value = match noun {
        Noun::Atom(v) => arena.atom(Goldilocks::new(*v)),
        Noun::Cell(head, tail) => {
            let head = load(arena, head)?;
            let tail = load(arena, tail)?;
            arena.pair(head, tail)
        }
    };
    value.ok_or_else(|| "test formula exceeds the nox arena capacity".to_string())
}

fn execute(formula: Noun) -> Result<(), String> {
    std::thread::Builder::new()
        .name("trident-test".to_string())
        .stack_size(STACK_SIZE)
        .spawn(move || {
            let mut arena = Reduction::<ARENA>::new();
            let entry = load(&mut arena, &formula)?;
            let subject = arena
                .atom(Goldilocks::ZERO)
                .ok_or_else(|| "test arena full".to_string())?;
            match nox::reduce(
                &mut arena,
                subject,
                entry,
                TEST_BUDGET,
                &NullCalls,
                &mut NoTrace,
            ) {
                Outcome::Ok(_, _) => Ok(()),
                Outcome::Halt(_) => Err(format!(
                    "nox test exhausted its {TEST_BUDGET}-reduction budget"
                )),
                Outcome::Error(error) => Err(format!("nox execution failed: {error:?}")),
            }
        })
        .map_err(|e| format!("cannot start nox test worker: {e}"))?
        .join()
        .map_err(|_| "nox test worker panicked".to_string())?
}

/// One executable test entry, with all original module definitions retained.
#[derive(Clone, Debug)]
pub struct TestProgram {
    pub name: String,
    pub modules: Vec<crate::ModuleTir>,
}

/// Target-independent test preparation for warrior-owned execution.
#[derive(Clone, Debug)]
pub struct TestPrograms {
    pub tests: Vec<TestProgram>,
    pub skipped: usize,
}

/// Resolve and check the project once, then give each active test its own entry.
/// The caller supplies project-resolved options, as for `build_tir_modules`.
/// No application entry is executed or used to infer a test's capabilities.
pub fn prepare_test_programs(
    entry_path: &Path,
    options: &CompileOptions,
) -> Result<TestPrograms, Vec<Diagnostic>> {
    use crate::tir::{builder::TIRBuilder, optimize::optimize as optimize_tir, TIROp};
    let mut options = options.clone();
    options.cfg_flags.insert("test".to_string());
    let mut project = PreparedProject::build_tests(entry_path, &options)?;
    let mut entries = Vec::new();
    let mut skipped = 0;
    for (index, module) in project.modules.iter().enumerate() {
        let (functions, excluded) = selected_tests(&module.file, &options.cfg_flags);
        skipped += excluded;
        for function in functions {
            if !function.params.is_empty()
                || !function.type_params.is_empty()
                || function.return_ty.is_some()
                || function.body.is_none()
            {
                return Err(vec![Diagnostic::error(
                    "tests require a body, no parameters, no generics and no return type"
                        .to_string(),
                    function.name.span,
                )]);
            }
            let mut selected = function.clone();
            selected.is_pub = true;
            let selected = crate::span::Spanned::new(ast::Item::Fn(selected), function.name.span);
            let mut view = module.file.clone();
            view.kind = ast::FileKind::Module;
            view.items = vec![selected];
            project.exports[index].check_entry_requirements(&view, &options)?;
            entries.push((module.file.name.node.clone(), function.name.node.clone()));
        }
    }
    // Emit test bodies through the normal builder, retaining main and helpers.
    for module in &mut project.modules {
        module.file.kind = ast::FileKind::Module;
        for item in &mut module.file.items {
            if let ast::Item::Fn(function) = &mut item.node {
                function.is_test = false;
            }
        }
    }
    let mut harness = "__trident_tests".to_string();
    while project
        .modules
        .iter()
        .any(|m| m.file.name.node.replace('.', "_") == harness)
    {
        harness.push('_');
    }
    let files: Vec<_> = project.modules.iter().map(|m| &m.file).collect();
    let modules: Vec<_> = project
        .modules
        .iter()
        .zip(&project.exports)
        .enumerate()
        .map(|(i, (pm, exports))| {
            let ops = TIRBuilder::new(options.target_config.clone())
                .with_target_intrinsics(options.target_intrinsic_widths())
                .with_cfg_flags(options.cfg_flags.clone())
                .with_module_types(&files)
                .with_intrinsics(project.intrinsic_map(i))
                .with_function_aliases(project.function_aliases(i))
                .with_module_aliases(project.module_aliases(i))
                .with_mono_instances(exports.mono_instances.clone())
                .with_call_resolutions(exports.call_resolutions.clone())
                .build_file(&pm.file)?;
            Ok(crate::ModuleTir {
                name: pm.file.name.node.clone(),
                is_program: false,
                ops: optimize_tir(ops),
            })
        })
        .collect::<Result<Vec<_>, Vec<Diagnostic>>>()?;
    let tests = entries
        .into_iter()
        .map(|(module, function)| {
            let mut modules = modules.clone();
            modules.push(crate::ModuleTir {
                name: harness.clone(),
                is_program: true,
                ops: vec![
                    TIROp::Entry("main".into()),
                    TIROp::FnStart("main".into()),
                    TIROp::Call(format!("@{}__{}", module.replace('.', "_"), function)),
                    TIROp::Return,
                    TIROp::FnEnd,
                ],
            });
            TestProgram {
                name: format!("{module}.{function}"),
                modules,
            }
        })
        .collect();
    Ok(TestPrograms { tests, skipped })
}
