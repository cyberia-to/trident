//! Compiler-owned filesystem transport; source symlinks resolve to regular files.
use std::{
    fs::{self, File, Metadata, OpenOptions},
    io::{self, Read},
    path::Path,
};

const MAX_SOURCE_BYTES: usize = 4 * 1024 * 1024;
const MAX_PROJECT_BYTES: usize = 1024 * 1024;

pub(crate) fn source(path: &Path) -> io::Result<String> {
    read(path, MAX_SOURCE_BYTES)
}

pub(crate) fn project(path: &Path) -> io::Result<String> {
    read(path, MAX_PROJECT_BYTES)
}

fn admit(metadata: &Metadata, limit: usize) -> io::Result<()> {
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "compiler input must resolve to a regular file",
        ));
    }
    if metadata.len() > limit as u64 {
        return Err(size_error(limit));
    }
    Ok(())
}

fn size_error(limit: usize) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("compiler input exceeds {limit} byte limit"),
    )
}

fn open_admitted(path: &Path, before: &Metadata, limit: usize) -> io::Result<File> {
    admit(before, limit)?;
    let mut options = OpenOptions::new();
    options.read(true);
    // O_NONBLOCK prevents a regular path swapped for a FIFO from blocking open.
    // Follow ordinary source symlinks, then check the opened descriptor itself.
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(0x4);
    }
    #[cfg(all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    ))]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(0x800);
    }
    let file = options.open(path)?;
    let after = file.metadata()?;
    admit(&after, limit)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if before.dev() != after.dev() || before.ino() != after.ino() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "compiler input changed during open",
            ));
        }
    }
    Ok(file)
}

fn read(path: &Path, limit: usize) -> io::Result<String> {
    // metadata follows legitimate symlinks; the descriptor checks close the
    // admission race without replacing source bytes with a separate preflight.
    let before = fs::metadata(path)?;
    decode(open_admitted(path, &before, limit)?, limit)
}

fn decode(file: File, limit: usize) -> io::Result<String> {
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(size_error(limit));
    }
    String::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "compiler input is not UTF-8"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_transport_bound_utf8_and_regular_files_are_enforced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("input.tri");
        fs::write(&path, b"12345678").unwrap();
        assert_eq!(read(&path, 8).unwrap(), "12345678");
        fs::write(&path, b"123456789").unwrap();
        assert_eq!(
            read(&path, 8).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        fs::write(&path, [0xff]).unwrap();
        assert!(read(&path, 8).unwrap_err().to_string().contains("UTF-8"));
        assert_eq!(
            read(dir.path(), 8).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }

    #[test]
    fn bytes_added_after_open_still_obey_the_transport_bound() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("input.tri");
        fs::write(&path, b"12345678").unwrap();
        let file = open_admitted(&path, &fs::metadata(&path).unwrap(), 8).unwrap();
        fs::write(&path, b"123456789").unwrap();
        assert_eq!(
            decode(file, 8).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[cfg(unix)]
    #[test]
    fn regular_source_symlinks_work_and_retargeted_paths_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first.tri");
        let second = dir.path().join("second.tri");
        let link = dir.path().join("link.tri");
        fs::write(&first, b"first").unwrap();
        fs::write(&second, b"second").unwrap();
        std::os::unix::fs::symlink(&first, &link).unwrap();
        assert_eq!(read(&link, 8).unwrap(), "first");
        let before = fs::metadata(&link).unwrap();
        fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(&second, &link).unwrap();
        assert!(open_admitted(&link, &before, 8)
            .unwrap_err()
            .to_string()
            .contains("changed during open"));
    }

    #[cfg(any(
        target_os = "macos",
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    #[test]
    fn regular_file_replaced_by_fifo_is_rejected_after_nonblocking_open() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("input.tri");
        fs::write(&path, b"data").unwrap();
        let before = fs::metadata(&path).unwrap();
        fs::remove_file(&path).unwrap();
        assert!(std::process::Command::new("mkfifo")
            .arg(&path)
            .status()
            .unwrap()
            .success());
        assert_eq!(
            open_admitted(&path, &before, 8).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }
}
