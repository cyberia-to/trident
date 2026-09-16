// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use std::path::PathBuf;

use clap::Args;

use super::resolve_input;

#[derive(Args)]
pub struct RunArgs {
    /// Input .tri file or directory with trident.toml
    pub input: PathBuf,
    /// Target VM or OS (default: nox)
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
    /// Vimputer (geeky for state/chain instance)
    #[arg(long, conflicts_with = "state")]
    pub vimputer: Option<String>,
    /// State (gamy for vimputer/chain instance)
    #[arg(long, conflicts_with = "vimputer")]
    pub state: Option<String>,
    /// Compilation profile (debug or release)
    #[arg(long, default_value = "debug")]
    pub profile: String,
    /// Public input values (comma-separated field elements)
    #[arg(long, value_delimiter = ',')]
    pub input_values: Option<Vec<u64>>,
    /// Secret/divine input values (comma-separated field elements)
    #[arg(long, value_delimiter = ',')]
    pub secret: Option<Vec<u64>>,
    /// Nondeterministic digest queue (comma-separated field elements)
    #[arg(long, value_delimiter = ',')]
    pub digests: Option<Vec<u64>>,
    /// Input/witness file in the selected warrior's format
    #[arg(long, conflicts_with_all = ["input_values", "secret", "digests"])]
    pub input_file: Option<PathBuf>,
}

pub fn cmd_run(args: RunArgs) {
    let ri = resolve_input(&args.input);
    let target = super::source_target(args.target.as_deref(), ri.project.as_ref());
    let bf = super::resolve_battlefield(
        &target,
        &args.engine,
        &args.terrain,
        &args.network,
        &args.union_flag,
        &args.vimputer,
        &args.state,
    );
    let target = bf.target;
    let state_for_warrior = bf.state;

    if let Some(warrior_bin) = super::find_warrior(&target) {
        let mut extra: Vec<String> = vec![
            ri.entry.display().to_string(),
            "--target".to_string(),
            target.clone(),
            "--profile".to_string(),
            args.profile.clone(),
        ];
        if let Some(ref vals) = args.input_values {
            extra.push("--input-values".to_string());
            let s: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
            extra.push(s.join(","));
        }
        if let Some(ref vals) = args.secret {
            extra.push("--secret".to_string());
            let s: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
            extra.push(s.join(","));
        }
        if let Some(ref state_name) = state_for_warrior {
            extra.push("--state".to_string());
            extra.push(state_name.clone());
        }
        if let Some(ref vals) = args.digests {
            extra.push("--digests".to_string());
            extra.push(
                vals.iter()
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
            );
        }
        if let Some(ref path) = args.input_file {
            extra.push("--input-file".to_string());
            extra.push(path.display().to_string());
        }
        let refs: Vec<&str> = extra.iter().map(|s| s.as_str()).collect();
        super::delegate_to_warrior(&warrior_bin, "run", &refs);
        return;
    }

    super::missing_warrior(&target, "run");
}
