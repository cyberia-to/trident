//! Generate/check SH0.2 golden data vectors. No source-language feature claim.
use clap::Parser;
use nox::Reduction;
use serde::Serialize;
use std::path::PathBuf;

#[path = "selfhost_data/model.rs"]
#[allow(dead_code)] // Mutation/validation methods are exercised by conformance tests.
mod model;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    check: bool,
}

#[derive(Serialize)]
struct Vector {
    name: &'static str,
    bytes: Vec<u8>,
    noun: String,
    particle_le_hex: String,
}

fn vectors() -> Result<String, String> {
    let mut ar = Reduction::<1024>::new();
    let mut vectors = Vec::new();
    for (name, bytes) in [
        ("empty", vec![]),
        ("zero", vec![0]),
        ("four_bytes", vec![1, 2, 3, 4]),
        ("five_bytes", vec![1, 2, 3, 4, 5]),
        ("three_words", vec![0, 1, 2, 3, 4, 5, 6, 7, 255]),
        ("all_ones", vec![255; 4]),
        ("raw_text_bytes", vec![0, 13, 10, 0xc3, 0xa9, 0xff]),
    ] {
        let data = model::Bytes::from_slice(&mut ar, &bytes, 64).map_err(|e| format!("{e:?}"))?;
        let noun = data.encode(&mut ar).map_err(|e| format!("{e:?}"))?;
        let digest = ar.digest(noun).ok_or("missing identity")?;
        let particle_le_hex = nox::data::hash::digest_bytes(digest)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        vectors.push(Vector {
            name,
            bytes,
            particle_le_hex,
            noun: model::display(&ar, noun, &mut 128).map_err(|e| format!("{e:?}"))?,
        });
    }
    serde_json::to_string_pretty(&vectors)
        .map(|s| s + "\n")
        .map_err(|e| e.to_string())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let json = vectors()?;
    if args.check {
        if std::fs::read_to_string(&args.output)? != json {
            return Err("native-data vectors are stale; regenerate and review".into());
        }
    } else {
        std::fs::write(&args.output, json)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "selfhost_data/tests.rs"]
mod tests;
#[cfg(test)]
#[path = "selfhost_data/vm.rs"]
mod vm;
