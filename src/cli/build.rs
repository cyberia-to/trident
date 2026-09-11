// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use std::path::PathBuf;
use std::process;

use clap::Args;

use super::{find_program_source, load_dep_dirs, resolve_input, resolve_options};

#[derive(Args)]
pub struct BuildArgs {
    /// Input .tri file or directory with trident.toml
    pub input: PathBuf,
    /// Output file (default: <input>.<target ext>, e.g. .tasm, .nox)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    /// Print the cost report (nox: reductions; a stack target: the
    /// warrior's own report, if it supports one)
    #[arg(long)]
    pub costs: bool,
    /// Target VM (default: nox)
    #[arg(long, default_value = "nox")]
    pub target: String,
    /// Engine (geeky for terrain/VM)
    #[arg(long, conflicts_with_all = ["terrain", "network", "union_flag"])]
    pub engine: Option<String>,
    /// Terrain (gamy for engine/VM)
    #[arg(long, conflicts_with_all = ["engine", "network", "union_flag"])]
    pub terrain: Option<String>,
    /// Network (geeky for union/OS)
    #[arg(long, conflicts_with_all = ["engine", "terrain", "union_flag"])]
    pub network: Option<String>,
    /// Union (gamy for network/OS)
    #[arg(long = "union", conflicts_with_all = ["engine", "terrain", "network"])]
    pub union_flag: Option<String>,
    /// Compilation profile for cfg flags (debug or release)
    #[arg(long, default_value = "debug")]
    pub profile: String,
}

pub fn cmd_build(args: BuildArgs) {
    let BuildArgs {
        input,
        output,
        costs,
        target,
        engine,
        terrain,
        network,
        union_flag,
        profile,
    } = args;
    let bf = super::resolve_battlefield_compile(&target, &engine, &terrain, &network, &union_flag);
    let target = bf.target;
    let ri = resolve_input(&input);

    let mut options = resolve_options(&target, &profile, ri.project.as_ref());
    if let Some(ref proj) = ri.project {
        options.dep_dirs = load_dep_dirs(proj);
    }

    // Stack targets: the core stops at TIR (build_tir/build_tir_modules are
    // still here; instruction selection and linking are the warrior's —
    // .claude/plans/warrior-owns-lowering.md S3). Delegate the way
    // run/prove/verify already do.
    if options.target_config.architecture != trident::target::Arch::Tree {
        if let Some(warrior_bin) = super::find_warrior(&target) {
            let mut extra: Vec<String> = vec![
                input.display().to_string(),
                "--target".to_string(),
                target.clone(),
                "--profile".to_string(),
                profile.clone(),
            ];
            if let Some(ref out) = output {
                extra.push("-o".to_string());
                extra.push(out.display().to_string());
            }
            if costs {
                extra.push("--costs".to_string());
            }
            let refs: Vec<&str> = extra.iter().map(|s| s.as_str()).collect();
            super::delegate_to_warrior(&warrior_bin, "build", &refs);
            return;
        }
        eprintln!("No warrior found for target '{}'.", target);
        eprintln!("Warriors handle lowering, execution, proving, and deployment for stack targets.");
        eprintln!();
        eprintln!("Install a warrior for this target:");
        eprintln!("  cargo install trisha   # Triton VM + Neptune");
        process::exit(1);
    }

    let compiled = match trident::compile_project_with_options(&ri.entry, &options) {
        Ok(t) => t,
        Err(errors) => {
            for e in &errors {
                eprintln!("{:?}", e);
            }
            process::exit(1);
        }
    };

    // Use target's output_extension (e.g. ".nox" for nox)
    let ext = options
        .target_config
        .output_extension
        .trim_start_matches('.');
    let default_output = if let Some(ref proj) = ri.project {
        proj.root_dir.join(format!("{}.{}", proj.name, ext))
    } else {
        input.with_extension(ext)
    };

    let out_path = output.unwrap_or(default_output);
    if let Err(e) = std::fs::write(&out_path, &compiled) {
        eprintln!("error: cannot write '{}': {}", out_path.display(), e);
        process::exit(1);
    }
    eprintln!("Compiled -> {}", out_path.display());

    if !costs {
        return;
    }
    let source_path = match find_program_source(&input) {
        Some(p) => p,
        None => return,
    };
    let cost_options = resolve_options(&target, &profile, None);
    match trident::nox_cost_project(&source_path, &cost_options) {
        Ok(nox_cost) => eprintln!("\n{}", nox_cost.format_report()),
        Err(_) => eprintln!("error: could not analyze nox reduction cost"),
    }
}
