//! Reproduce a Rust seed from an explicit frozen package, never ambient libraries.
use clap::Parser;
use serde::Deserialize;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    manifest: PathBuf,
    #[arg(long)]
    entry: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    compiler_job: bool,
}

#[derive(Deserialize)]
struct Package {
    modules: Vec<Module>,
}
#[derive(Deserialize)]
struct Module {
    logical_path: String,
    file: PathBuf,
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    const MAX: u64 = 4 << 20;
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > MAX {
        return Err("snapshot file bound".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX {
        return Err("snapshot file grew past bound".into());
    }
    Ok(bytes)
}

fn explicit_owner(name: &str) -> Result<(), String> {
    if trident::resolve::canonical_module_name(name) != name {
        return Err(format!("snapshot requires a canonical owner: {name}"));
    }
    if matches!(name, "std.target" | "vm.crypto.hash" | "vm.io.io") {
        return Err(format!("snapshot cannot override generated owner: {name}"));
    }
    Ok(())
}

fn main() -> Result<(), String> {
    let args = Args::parse();
    if args.output.exists() {
        return Err("output already exists".into());
    }
    let manifest: Package =
        serde_json::from_slice(&read(&args.manifest)?).map_err(|e| e.to_string())?;
    if manifest.modules.len() > 4096 {
        return Err("snapshot module bound".into());
    }
    let parent = args.manifest.parent().ok_or("manifest parent")?;
    let mut options = trident::CompileOptions::default();
    let mut total = 0usize;
    for module in manifest.modules {
        explicit_owner(&module.logical_path)?;
        let content = read(&parent.join(module.file))?;
        total += content.len();
        if total > 16 << 20 {
            return Err("snapshot source bound".into());
        }
        let source = String::from_utf8(content).map_err(|e| e.to_string())?;
        if options
            .module_sources
            .insert(module.logical_path, source)
            .is_some()
        {
            return Err("duplicate snapshot module".into());
        }
    }
    let entry_source = String::from_utf8(read(&args.entry)?).map_err(|e| e.to_string())?;
    for (name, source) in options
        .module_sources
        .iter()
        .map(|(n, s)| (n.as_str(), s.as_str()))
        .chain(std::iter::once(("<entry>", entry_source.as_str())))
    {
        let file = trident::parse_source_silent(source, name).map_err(|e| format!("{e:?}"))?;
        if name != "<entry>" && file.name.node != name {
            return Err(format!(
                "snapshot module {name} declares {}",
                file.name.node
            ));
        }
        if name == "<entry>"
            && options
                .module_sources
                .get(&file.name.node)
                .is_some_and(|s| s != source)
        {
            return Err("entry bytes disagree with snapshot module".into());
        }
        for dependency in file.uses {
            let dependency = dependency.node.as_dotted();
            explicit_owner(&dependency)?;
            if !options.module_sources.contains_key(&dependency) {
                return Err(format!("snapshot omits {dependency} imported by {name}"));
            }
        }
    }
    let profile = if args.compiler_job {
        trident::NativeArtifactProfile::CompilerJob
    } else {
        trident::NativeArtifactProfile::RawNoun
    };
    let artifact = trident::compile_native_artifact_project(
        &args.entry,
        &options,
        profile,
        trident::NATIVE_ARTIFACT_LIMITS,
    )
    .map_err(|errors| format!("{errors:?}"))?;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args.output)
        .and_then(|mut file| file.write_all(&artifact.bytes))
        .map_err(|e| e.to_string())
}
