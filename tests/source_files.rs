//! Filesystem transport limits apply before parsing and remain shared by callers.
use std::{fs, path::Path};
use trident::{compile_to_bundle, CompileOptions};

fn write(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
}

#[test]
fn source_transport_accepts_its_exact_bound_and_rejects_one_more_byte() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("entry.tri");
    fs::write(&path, vec![b' '; 4 * 1024 * 1024]).unwrap();
    assert_eq!(
        trident::read_source_file(&path).unwrap().len(),
        4 * 1024 * 1024
    );
    fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(4 * 1024 * 1024 + 1)
        .unwrap();
    let errors = compile_to_bundle(&path, &CompileOptions::default()).unwrap_err();
    assert!(errors
        .iter()
        .any(|e| e.message.contains("4194304 byte limit")));
}

#[test]
fn imported_sources_project_manifests_and_lockfiles_use_their_transport_bounds() {
    let dir = tempfile::tempdir().unwrap();
    let entry = dir.path().join("main.tri");
    write(
        &entry,
        "program app use helper fn main()->Field{helper.value()}",
    );
    let module = dir.path().join("helper.tri");
    fs::File::create(&module)
        .unwrap()
        .set_len(4 * 1024 * 1024 + 1)
        .unwrap();
    let errors = compile_to_bundle(&entry, &CompileOptions::default()).unwrap_err();
    assert!(errors
        .iter()
        .any(|e| e.message.contains("4194304 byte limit")));

    let manifest = dir.path().join("trident.toml");
    fs::File::create(&manifest)
        .unwrap()
        .set_len(1024 * 1024 + 1)
        .unwrap();
    assert!(trident::project::Project::load(&manifest)
        .unwrap_err()
        .message
        .contains("1048576 byte limit"));
    let lockfile = dir.path().join("trident.lock");
    fs::File::create(&lockfile)
        .unwrap()
        .set_len(1024 * 1024 + 1)
        .unwrap();
    assert!(trident::manifest::load_lockfile(&lockfile)
        .unwrap_err()
        .contains("1048576 byte limit"));
}

#[cfg(unix)]
#[test]
fn regular_source_and_project_symlinks_keep_their_existing_semantics() {
    let dir = tempfile::tempdir().unwrap();
    let entry = dir.path().join("main.tri");
    let source = dir.path().join("real-entry.tri");
    let module = dir.path().join("helper.tri");
    let real_module = dir.path().join("real-helper.tri");
    write(
        &source,
        "program app use helper fn main()->Field{helper.value()}",
    );
    write(&real_module, "module helper pub fn value()->Field{7}");
    std::os::unix::fs::symlink(&source, &entry).unwrap();
    std::os::unix::fs::symlink(&real_module, &module).unwrap();
    let options = CompileOptions::default();
    let linked = compile_to_bundle(&entry, &options).unwrap();
    let direct = compile_to_bundle(&source, &options).unwrap();
    assert_eq!(linked.assembly, direct.assembly);
    let project = dir.path().join("real-project.toml");
    write(&project, "[project]\nname=\"app\"\nentry=\"main.tri\"\n");
    let manifest = dir.path().join("trident.toml");
    std::os::unix::fs::symlink(&project, &manifest).unwrap();
    let (resolved, options) = trident::source_options(dir.path(), &options).unwrap();
    assert_eq!(resolved, entry);
    assert_eq!(
        compile_to_bundle(&resolved, &options).unwrap().assembly,
        linked.assembly
    );
}

#[cfg(unix)]
#[test]
fn cli_rejects_source_entry_import_and_manifest_streams_without_waiting_for_writers() {
    use std::{
        process::Command,
        thread,
        time::{Duration, Instant},
    };
    fn fifo(path: &Path) {
        assert!(Command::new("mkfifo").arg(path).status().unwrap().success());
    }
    fn rejected(path: &Path) {
        let mut child = Command::new(env!("CARGO_BIN_EXE_trident"))
            .arg("check")
            .arg(path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                let _ = child.wait();
                panic!("compiler blocked on stream input");
            }
            thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("regular file"));
    }
    let dir = tempfile::tempdir().unwrap();
    let entry = dir.path().join("entry.tri");
    fifo(&entry);
    rejected(&entry);
    fs::remove_file(&entry).unwrap();
    write(
        &entry,
        "program app use helper fn main()->Field{helper.value()}",
    );
    fifo(&dir.path().join("helper.tri"));
    rejected(&entry);
    fs::remove_file(dir.path().join("helper.tri")).unwrap();
    write(&entry, "program app fn main()->Field{7}");
    fifo(&dir.path().join("trident.toml"));
    rejected(&entry);
}
