use std::{env, fs, io, path::Path};

fn collect(root: &Path, dir: &Path, entries: &mut Vec<(String, String)>) -> io::Result<()> {
    let mut paths = fs::read_dir(dir)?
        .map(|e| e.map(|e| e.path()))
        .collect::<io::Result<Vec<_>>>()?;
    paths.sort();
    for path in paths {
        if path.is_dir() {
            collect(root, &path, entries)?;
        } else if path.extension().is_some_and(|e| e == "tri" || e == "toml") {
            let relative = path.strip_prefix(root).map_err(io::Error::other)?;
            entries.push((
                relative.to_string_lossy().replace('\\', "/"),
                fs::read_to_string(path)?,
            ));
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = env::var("CARGO_MANIFEST_DIR")?;
    let mut entries = Vec::new();
    for dir in ["lib", "catalog"] {
        println!("cargo:rerun-if-changed={dir}");
        collect(Path::new(&root), &Path::new(&root).join(dir), &mut entries)?;
    }
    let mut code = String::from("pub(crate) static FILES: &[(&str, &str)] = &[\n");
    for (path, content) in entries {
        code.push_str(&format!("({path:?}, {content:?}),\n"));
    }
    code.push_str("];\n");
    fs::write(Path::new(&env::var("OUT_DIR")?).join("resources.rs"), code)?;
    Ok(())
}
