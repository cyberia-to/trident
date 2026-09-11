// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use std::path::PathBuf;
use std::process;

use clap::Args;

use super::{resolve_input, resolve_options};

#[derive(Args)]
pub struct TestArgs {
    /// Input .tri file or directory with trident.toml
    pub input: PathBuf,
    /// Target VM (default: nox)
    #[arg(long)]
    pub target: Option<String>,
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

pub fn cmd_test(args: TestArgs) {
    let TestArgs {
        input,
        target,
        engine,
        terrain,
        network,
        union_flag,
        profile,
    } = args;
    let ri = resolve_input(&input);
    let target = super::source_target(target.as_deref(), ri.project.as_ref());
    let selected = engine
        .as_deref()
        .or(terrain.as_deref())
        .or(network.as_deref())
        .or(union_flag.as_deref())
        .unwrap_or(&target);
    if selected != "nox" {
        eprintln!("warrior test command is not implemented for '{selected}'");
        process::exit(1);
    }
    let bf = super::resolve_battlefield_compile(&target, &engine, &terrain, &network, &union_flag);
    let target = bf.target;

    let options = resolve_options(&target, &profile, ri.project.as_ref());
    let result = trident::run_tests(&ri.entry, &options);

    match result {
        Ok(report) => {
            eprintln!("{}", report);
        }
        Err(errors) => {
            for error in errors {
                eprintln!("{}", error.message);
            }
            process::exit(1);
        }
    }
}
