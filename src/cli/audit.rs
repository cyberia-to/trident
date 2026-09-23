// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use std::path::PathBuf;
use std::process;

use clap::Args;

use super::{load_and_parse, resolve_input};

#[derive(Args)]
pub struct AuditArgs {
    /// Input .tri file to audit
    pub input: Option<PathBuf>,
    /// Compilation target (otherwise the project target, then nox).
    #[arg(long)]
    pub target: Option<String>,
    /// Compilation profile for conditional declarations.
    #[arg(long, default_value = "debug")]
    pub profile: String,
    /// Show detailed output
    #[arg(long)]
    pub verbose: bool,
    /// Output SMT-LIB2 encoding to file (for external solvers)
    #[arg(long, value_name = "PATH")]
    pub smt: Option<PathBuf>,
    /// Run Z3 solver (if available) for formal verification
    #[arg(long)]
    pub z3: bool,
    /// Output machine-readable JSON report (for LLM/CI consumption)
    #[arg(long)]
    pub json: bool,
    /// Synthesize and suggest specifications (invariants, pre/postconditions)
    #[arg(long)]
    pub synthesize: bool,
}

pub fn cmd_audit(args: AuditArgs) {
    if args.input.is_none() {
        // Execution-correctness auditing (classic/hand/neural vs baselines,
        // via trisha) moved to the warrior — `trident bench --full` covered
        // the same four dimensions and lived in the compiler only because
        // Triton did. See .claude/plans/warrior-owns-lowering.md.
        eprintln!("error: `trident audit` needs a .tri file to audit");
        eprintln!("       execution-correctness auditing is the warrior's own bench");
        process::exit(1);
    }
    cmd_audit_symbolic(args);
}

// ── Symbolic audit ──────────────────────────────────────────────────

fn cmd_audit_symbolic(args: AuditArgs) {
    let input = args.input.expect("symbolic audit requires input");
    let AuditArgs {
        verbose,
        smt: smt_output,
        z3: run_z3,
        json,
        synthesize,
        ..
    } = args;
    let ri = resolve_input(&input);
    let target = super::source_target(args.target.as_deref(), ri.project.as_ref());
    let options = super::resolve_options(&target, &args.profile, ri.project.as_ref());
    let (entry, options) = trident::source_options(&ri.entry, &options).unwrap_or_else(|errors| {
        for error in errors {
            eprintln!("error: {}", error.message);
        }
        process::exit(1)
    });
    if let Err(error) = trident::sym::validate_audit_target(&options.target_config) {
        eprintln!("error: {error}");
        process::exit(1);
    }
    if let Err(errors) = trident::check_project_with_options(&entry, &options) {
        for error in errors {
            eprintln!("error: {}", error.message);
        }
        process::exit(1);
    }

    eprintln!("Auditing {}...", input.display());

    let (_source, mut file) = load_and_parse(&entry);
    file.items.retain(|item| {
        let cfg = match &item.node {
            trident::ast::Item::Fn(value) => &value.cfg,
            trident::ast::Item::Const(value) => &value.cfg,
            trident::ast::Item::Struct(value) => &value.cfg,
            trident::ast::Item::Event(value) => &value.cfg,
        };
        cfg.as_ref()
            .is_none_or(|flag| options.cfg_flags.contains(&flag.node))
    });
    let per_fn = trident::sym::analyze_all_with_target(&file, &options.target_config)
        .unwrap_or_else(|error| {
            eprintln!("error: {error}");
            process::exit(1)
        });
    let mut exit_code = if per_fn.is_empty() { 2 } else { 0 };
    let mut reports = Vec::new();
    let mut scripts = String::new();
    for (name, system) in &per_fn {
        let report = trident::solve::verify(system);
        let mut verdict = match report.verdict {
            trident::solve::Verdict::Safe => "safe",
            trident::solve::Verdict::Unknown => "unknown",
            _ => "unsafe",
        };
        let supported = system.unsupported.is_empty() && !system.constraints.is_empty();
        if supported {
            let script = trident::smt::encode_system(system, trident::smt::QueryMode::SafetyCheck);
            scripts.push_str(&format!("(reset)\n; Function: {name}\n{script}\n"));
            if run_z3 {
                verdict = match trident::smt::run_z3(&script) {
                    Ok(result) => match result.status {
                        trident::smt::SmtStatus::Unsat => "safe",
                        trident::smt::SmtStatus::Sat => "unsafe",
                        _ => "unknown",
                    },
                    Err(error) => {
                        eprintln!("Z3 {name}: {error}");
                        "unknown"
                    }
                };
            }
        } else {
            verdict = "unknown";
        }
        if verdict == "unsafe" {
            exit_code = 1;
        } else if verdict == "unknown" && exit_code == 0 {
            exit_code = 2;
        }
        if verbose || !json {
            eprintln!(
                "{name}: {} ({} obligations)",
                verdict.to_uppercase(),
                system.constraints.len()
            );
            for reason in &system.unsupported {
                eprintln!("  unsupported: {reason}");
            }
            if system.constraints.is_empty() {
                eprintln!("  no supported obligations");
            }
        }
        let detail: serde_json::Value = serde_json::from_str(
            &trident::report::generate_json_report(name, system, &report),
        )
        .unwrap();
        reports.push(serde_json::json!({"function": name, "verdict": verdict, "unsupported": system.unsupported, "analysis": detail}));
    }
    if let Some(path) = smt_output {
        if let Err(error) = std::fs::write(&path, scripts) {
            eprintln!("cannot write SMT: {error}");
            process::exit(1);
        }
    }
    if synthesize {
        eprintln!(
            "{}",
            trident::synthesize::format_report(&trident::synthesize::synthesize_specs(&file))
        );
    }
    if json {
        println!(
            "{}",
            serde_json::json!({"file": entry, "verdict": if exit_code == 0 {"safe"} else if exit_code == 1 {"unsafe"} else {"unknown"}, "functions": reports})
        );
    }
    if per_fn.is_empty() {
        eprintln!("UNKNOWN: no analyzable function obligations");
    }
    if exit_code != 0 {
        process::exit(exit_code);
    }
}

// ── Equivalence checking ──────────────────────────────────────────

#[derive(Args)]
pub struct EquivArgs {
    /// Input .tri file containing both functions
    pub input: PathBuf,
    /// First function name
    pub fn_a: String,
    /// Second function name
    pub fn_b: String,
    /// Show detailed symbolic analysis
    #[arg(long)]
    pub verbose: bool,
}

pub fn cmd_equiv(args: EquivArgs) {
    let EquivArgs {
        input,
        fn_a,
        fn_b,
        verbose,
    } = args;
    if !input.extension().is_some_and(|e| e == "tri") {
        eprintln!("error: input must be a .tri file");
        process::exit(1);
    }

    let (_, file) = load_and_parse(&input);

    eprintln!(
        "Checking equivalence: {} vs {} in {}",
        fn_a,
        fn_b,
        input.display()
    );

    if verbose {
        let fn_hashes = trident::hash::hash_file(&file);
        if let Some(h) = fn_hashes.get(fn_a.as_str()) {
            eprintln!("  {} hash: {}", fn_a, h);
        }
        if let Some(h) = fn_hashes.get(fn_b.as_str()) {
            eprintln!("  {} hash: {}", fn_b, h);
        }
    }

    let result = trident::equiv::check_equivalence(&file, &fn_a, &fn_b);

    eprintln!("\n{}", result.format_report());

    match result.verdict {
        trident::equiv::EquivalenceVerdict::Equivalent => {}
        trident::equiv::EquivalenceVerdict::NotEquivalent => {
            process::exit(1);
        }
        trident::equiv::EquivalenceVerdict::Unknown => {
            process::exit(2);
        }
    }
}
