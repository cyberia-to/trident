//! Bounded whole-certificate diagnostic. Never evaluates a formula.
use joy_rs::structured::certificate::transport::Limits;
use std::{fs::File, io, path::Path};

mod frames;
mod index;
mod mutation;
mod noun;

const WIRE: u64 = 24 * 1024 * 1024 * 1024;
const DECODED: u64 = 96 * 1024 * 1024 * 1024;
const RECORDS: u64 = 12_000_000_000;
const RESULT: usize = 16 * 1024 * 1024;

fn bad(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn limits() -> Limits {
    Limits {
        wire_bytes: WIRE,
        decoded_bytes: DECODED,
        frames: DECODED + 1,
    }
}

fn input(path: &Path) -> io::Result<File> {
    let meta = path.symlink_metadata()?;
    if !meta.file_type().is_file() || meta.len() > WIRE {
        return Err(bad("regular input file within 24GiB required"));
    }
    File::open(path)
}

fn create(path: &Path) -> io::Result<File> {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
}

fn exact<const N: usize>(reader: &mut impl io::Read) -> io::Result<[u8; N]> {
    let mut bytes = [0; N];
    reader.read_exact(&mut bytes)?;
    Ok(bytes)
}

fn hex<const N: usize>(value: &str) -> io::Result<[u8; N]> {
    if value.len() != N * 2 {
        return Err(bad("hex length"));
    }
    let mut bytes = [0; N];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[2 * i..2 * i + 2], 16).map_err(|_| bad("hex"))?;
    }
    Ok(bytes)
}

fn context(args: &[String]) -> io::Result<[u8; 32]> {
    if args.len() != 6 {
        return Err(bad("program formula input profile budget frames"));
    }
    let mut h = hemera::Hasher::new();
    h.update(b"joy/nox/disclosed-compiler/v1\0");
    for arg in &args[..3] {
        h.update(&hex::<32>(arg)?);
    }
    h.update(&[args[3].parse::<u8>().map_err(|_| bad("profile"))?]);
    h.update(
        &args[4]
            .parse::<u64>()
            .map_err(|_| bad("budget"))?
            .to_le_bytes(),
    );
    h.update(
        &args[5]
            .parse::<u32>()
            .map_err(|_| bad("frames"))?
            .to_le_bytes(),
    );
    Ok(*h.finalize().as_bytes())
}

fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("index") if args.len() == 6 => index::build(Path::new(&args[2]), Path::new(&args[3]), Path::new(&args[4]), &args[5]),
        Some("mutate") if args.len() >= 7 => mutation::run(Path::new(&args[2]), Path::new(&args[3]), Path::new(&args[4]), Path::new(&args[5]), &args[6], &args[7..]),
        _ => Err(bad("index INPUT INDEX_JSON OUTPUT_DAG SHA256 | mutate INPUT INDEX_JSON ORIGINAL_DAG OUTPUT MODE [context]")),
    }
}
