// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use std::path::PathBuf;
use std::process;

use clap::Args;

#[derive(Args)]
pub struct InitArgs {
    /// Project name (defaults to current directory name)
    pub name: Option<String>,
}

pub fn cmd_init(args: InitArgs) {
    let InitArgs { name } = args;
    let (project_dir, project_name) = if let Some(ref name) = name {
        let dir = PathBuf::from(name);
        (dir, name.clone())
    } else {
        let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("my_project")
            .to_string();
        (dir, name)
    };

    // Create directory if name was provided
    if name.is_some() {
        if let Err(e) = std::fs::create_dir_all(&project_dir) {
            eprintln!(
                "error: cannot create directory '{}': {}",
                project_dir.display(),
                e
            );
            process::exit(1);
        }
    }

    let toml_path = project_dir.join("trident.toml");
    if toml_path.exists() {
        eprintln!("error: '{}' already exists", toml_path.display());
        process::exit(1);
    }

    let toml_content = format!(
        "[project]\nname = \"{}\"\nversion = \"0.1.0\"\nentry = \"main.tri\"\n",
        project_name
    );

    let main_content = format!(
        "program {}\n\nfn main() {{\n    let x: Field = pub_read()\n    pub_write(x)\n}}\n",
        project_name
    );

    if let Err(e) = std::fs::write(&toml_path, &toml_content) {
        eprintln!("error: cannot write '{}': {}", toml_path.display(), e);
        process::exit(1);
    }

    let main_path = project_dir.join("main.tri");
    if let Err(e) = std::fs::write(&main_path, &main_content) {
        eprintln!("error: cannot write '{}': {}", main_path.display(), e);
        process::exit(1);
    }

    eprintln!(
        "Created project '{}' in {}",
        project_name,
        project_dir.display()
    );
    eprintln!("  {}", toml_path.display());
    eprintln!("  {}", main_path.display());
}
