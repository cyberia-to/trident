//! trident mir — compile Rust MIR JSON to nox formulas
//!
//! Moved from nox CLI. The wasm/arm64/etc native-silicon targets moved to
//! the `silicon` binary (`trident/silicon/`); this stays in the core
//! because it produces trident's own representation (a nox formula), not
//! a translation out to a machine.

use clap::Args;

#[derive(Args)]
pub struct MirArgs {
    /// MIR JSON file (or - for stdin)
    #[arg()]
    file: Option<String>,

    /// Compile only this function
    #[arg(short = 'f', long = "function")]
    function: Option<String>,

    /// Output directory for .nox files
    #[arg(short = 'o')]
    output_dir: Option<String>,
}

pub fn cmd_mir(args: MirArgs) {
    let mut raw_args: Vec<String> = Vec::new();
    if let Some(f) = args.file {
        raw_args.push(f);
    }
    if let Some(ref func) = args.function {
        raw_args.push("-f".to_string());
        raw_args.push(func.clone());
    }
    if let Some(ref dir) = args.output_dir {
        raw_args.push("-o".to_string());
        raw_args.push(dir.clone());
    }
    trident::import::mir2nox::run_mir2nox(&raw_args);
}
