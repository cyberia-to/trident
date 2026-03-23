// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use std::path::PathBuf;

use clap::Args;

use super::{load_and_parse, resolve_input};

#[derive(Args)]
pub struct HashArgs {
    /// Input .tri file or directory with trident.toml
    pub input: PathBuf,
    /// Show full 256-bit hashes instead of short form
    #[arg(long)]
    pub full: bool,
}

pub fn cmd_hash(args: HashArgs) {
    let HashArgs { input, full } = args;
    let ri = resolve_input(&input);
    let (_, file) = load_and_parse(&ri.entry);

    let fn_hashes = trident::hash::hash_file(&file);
    let file_hash = trident::hash::hash_file_content(&file);

    if full {
        eprintln!("File: {} {}", file_hash.to_hex(), ri.entry.display());
    } else {
        eprintln!("File: {} {}", file_hash, ri.entry.display());
    }

    let mut sorted: Vec<_> = fn_hashes.iter().collect();
    sorted.sort_by_key(|(name, _)| (*name).clone());
    for (name, hash) in sorted {
        if full {
            println!("  {} {}", hash.to_hex(), name);
        } else {
            println!("  {} {}", hash, name);
        }
    }
}
