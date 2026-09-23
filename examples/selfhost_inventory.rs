//! SH0 source inventory. Parses checkout bytes; never executes a compiler stage.
use clap::Parser;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[path = "selfhost_inventory/walk.rs"]
mod walk;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[arg(long)]
    output: PathBuf,
    /// Reject a stale receipt without modifying it.
    #[arg(long)]
    check: bool,
}

#[derive(Serialize)]
struct Inventory {
    schema: u32,
    scope: &'static str,
    roots: Vec<String>,
    module_count: usize,
    source_bytes: usize,
    source_lines: usize,
    functions: usize,
    features: BTreeMap<String, usize>,
    modules: BTreeMap<String, walk::Module>,
}

fn module_path(name: &str) -> Result<PathBuf, String> {
    if !name
        .split('.')
        .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
    {
        return Err(format!("invalid inventory module name: {name}"));
    }
    Ok(PathBuf::from("lib").join(name.replace('.', "/") + ".tri"))
}

fn collect(root: &Path, roots: BTreeSet<String>) -> Result<Inventory, String> {
    if roots.is_empty() {
        return Err("no compiler modules found".into());
    }
    let mut pending = roots.clone();
    let mut modules = BTreeMap::new();
    while let Some(name) = pending.pop_first() {
        if modules.contains_key(&name) {
            continue;
        }
        let path = module_path(&name)?;
        let source = std::fs::read_to_string(root.join(&path))
            .map_err(|e| format!("cannot read {name}: {e}"))?;
        let file = trident::parse_source_silent(&source, &name)
            .map_err(|errors| format!("cannot parse {name}: {errors:?}"))?;
        if file.name.node != name {
            return Err(format!("module {name} declares {}", file.name.node));
        }
        let module = walk::inspect(&file, &source, &path.to_string_lossy().replace('\\', "/"));
        pending.extend(module.imports.iter().cloned());
        modules.insert(name, module);
    }
    let mut features = BTreeMap::new();
    for module in modules.values() {
        for (name, occurrence) in &module.features {
            *features.entry(name.clone()).or_default() += occurrence.count;
        }
    }
    Ok(Inventory {
        schema: 1,
        scope: "all declarations in compiler modules and transitive source imports; no cfg pruning, call reachability, inferred types or target-support claim",
        roots: roots.into_iter().collect(),
        module_count: modules.len(),
        source_bytes: modules.values().map(|m| m.source_bytes).sum(),
        source_lines: modules.values().map(|m| m.source_lines).sum(),
        functions: modules.values().map(|m| m.functions.len()).sum(),
        features,
        modules,
    })
}

fn discover(root: &Path) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let mut roots = BTreeSet::new();
    for entry in std::fs::read_dir(root.join("lib/std/compiler"))? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("tri") {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or("invalid module filename")?;
            roots.insert(format!("std.compiler.{stem}"));
        }
    }
    Ok(roots)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let inventory = collect(&args.root, discover(&args.root)?)?;
    let json = serde_json::to_string_pretty(&inventory)? + "\n";
    if args.check {
        if std::fs::read_to_string(&args.output)? != json {
            return Err(
                "compiler inventory is stale; regenerate and review the subset disposition".into(),
            );
        }
    } else {
        std::fs::write(&args.output, json)?;
    }
    eprintln!(
        "{} modules, {} functions, {} source bytes",
        inventory.module_count, inventory.functions, inventory.source_bytes
    );
    Ok(())
}

#[cfg(test)]
#[path = "selfhost_inventory/tests.rs"]
mod tests;
