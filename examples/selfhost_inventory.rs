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
    /// Inventory this program and its canonical source imports instead of all RAM compiler modules.
    #[arg(long)]
    entry: Option<PathBuf>,
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
    let paths = roots
        .into_iter()
        .map(|name| module_path(&name).map(|path| (name, path)))
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    collect_paths(root, paths)
}

fn collect_paths(root: &Path, roots: BTreeMap<String, PathBuf>) -> Result<Inventory, String> {
    if roots.is_empty() {
        return Err("no compiler modules found".into());
    }
    let mut pending = roots.clone();
    let mut modules = BTreeMap::new();
    let mut programs = BTreeSet::new();
    while let Some((name, path)) = pending.pop_first() {
        if modules.contains_key(&name) {
            continue;
        }
        let source = std::fs::read_to_string(root.join(&path))
            .map_err(|e| format!("cannot read {name}: {e}"))?;
        let file = trident::parse_source_silent(&source, &name)
            .map_err(|errors| format!("cannot parse {name}: {errors:?}"))?;
        if file.name.node != name {
            return Err(format!("module {name} declares {}", file.name.node));
        }
        if matches!(file.kind, trident::ast::FileKind::Program) {
            if !roots.contains_key(&name) {
                return Err(format!(
                    "import {name} declares a program instead of a module"
                ));
            }
            programs.insert(name.clone());
        }
        let module = walk::inspect(&file, &source, &path.to_string_lossy().replace('\\', "/"));
        for dependency in &module.imports {
            if programs.contains(dependency) {
                return Err(format!(
                    "program entry {dependency} cannot also resolve as an imported module"
                ));
            }
            if !modules.contains_key(dependency) {
                pending
                    .entry(dependency.clone())
                    .or_insert(module_path(dependency)?);
            }
        }
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
        roots: roots.into_keys().collect(),
        module_count: modules.len(),
        source_bytes: modules.values().map(|m| m.source_bytes).sum(),
        source_lines: modules.values().map(|m| m.source_lines).sum(),
        functions: modules.values().map(|m| m.functions.len()).sum(),
        features,
        modules,
    })
}

fn collect_entry(root: &Path, entry: &Path) -> Result<Inventory, String> {
    let mut relative = PathBuf::new();
    for part in entry.components() {
        match part {
            std::path::Component::Normal(name) => relative.push(name),
            std::path::Component::CurDir => {}
            _ => {
                return Err("entry must be a relative source path without parent traversal".into())
            }
        }
    }
    if relative.as_os_str().is_empty() {
        return Err("entry must name a source file".into());
    }
    let source = std::fs::read_to_string(root.join(&relative))
        .map_err(|error| format!("cannot read entry {}: {error}", relative.display()))?;
    let file = trident::parse_source_silent(&source, &relative.to_string_lossy())
        .map_err(|errors| format!("cannot parse entry {}: {errors:?}", relative.display()))?;
    if !matches!(file.kind, trident::ast::FileKind::Program) {
        return Err("inventory entry must declare a program".into());
    }
    let mut inventory = collect_paths(root, BTreeMap::from([(file.name.node, relative)]))?;
    inventory.scope = "all declarations in the selected program and transitive canonical source imports; no cfg pruning, call reachability, inferred types or target-support claim";
    Ok(inventory)
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
    let inventory = match &args.entry {
        Some(entry) => collect_entry(&args.root, entry)?,
        None => collect(&args.root, discover(&args.root)?)?,
    };
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
