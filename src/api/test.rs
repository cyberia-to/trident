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
            format!("test execution for '{}' is unsupported: the warrior test command is not implemented; use --target nox for nox tests", options.target_config.name),
            Span::dummy(),
        )]);
    }
    let mut options = options.clone();
    options.cfg_flags.insert("test".to_string());
    let project = PreparedProject::build_quiet(entry_path, &options)?;
    let files: Vec<_> = project.modules.iter().map(|m| &m.file).collect();
    let mut results = Vec::new();
    for module in &project.modules {
        for item in &module.file.items {
            let ast::Item::Fn(function) = &item.node else {
                continue;
            };
            if !function.is_test
                || function
                    .cfg
                    .as_ref()
                    .is_some_and(|f| !options.cfg_flags.contains(&f.node))
            {
                continue;
            }
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
                let mut selected = item.clone();
                if let ast::Item::Fn(f) = &mut selected.node {
                    f.is_pub = true;
                }
                let mut entry = module.file.clone();
                entry.items = vec![selected];
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
        return Ok("No active #[test] functions found.\n".to_string());
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
        "\ntest result: {}. {passed} passed; {failed} failed\n",
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
