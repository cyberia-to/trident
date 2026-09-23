//! SH0.3 protocol fixtures and bounded structural validation, without compilation.
use clap::Parser;
use nox::{Order, Reduction};
use serde::Serialize;
use std::path::PathBuf;

#[path = "selfhost_data/model.rs"]
#[allow(dead_code)]
mod data;
#[path = "selfhost_jobs/schema.rs"]
mod schema;
#[path = "selfhost_jobs/validate.rs"]
#[allow(dead_code)]
mod validate;

const TRANSPORT: nox::artifact::Limits = nox::artifact::Limits {
    max_bytes: 1 << 20,
    max_nodes: 3000,
    max_depth: 128,
};

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
    root_particle_le: String,
    bytes: usize,
    artifact_hex: String,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn fixture(ar: &mut Reduction<4096>) -> schema::Result<(Order, Order, Order, Order)> {
    use schema::*;
    // A profile1 placeholder program is a transport fixture, not a compiler.
    let zero = atom(ar, 0)?;
    let quote = atom(ar, 1)?;
    let body = pair(ar, quote, zero)?;
    let compiler = artifact(ar, body, 1, 1)?;
    let modules = vec![Module {
        path: "demo".into(),
        origin: "fixture".into(),
        version: "1".into(),
        source: b"program demo fn main() -> Field { 2 + 3 * 4 }\n".to_vec(),
    }];
    let options = Options {
        input: 0,
        output: 0,
        optimization: 0,
        cfg: vec!["release".into()],
    };
    let job = job(
        ar,
        compiler,
        &modules,
        "demo",
        "main",
        &options,
        &FIXTURE_CAPS,
    )?;
    // The fixture deliberately supplies a hand-authored output formula. No
    // source compiler ran; later SH2 must produce the formula inside the VM.
    let fourteen = atom(ar, 14)?;
    let formula = pair(ar, quote, fourteen)?;
    let artifact = artifact(ar, formula, 0, 0)?;
    let success = success(ar, job, artifact)?;
    let failure = failure(
        ar,
        job,
        &[Diagnostic {
            module: 0,
            start: 0,
            end: 7,
            code: 6,
            message: "unsupported fixture".into(),
        }],
    )?;
    let decoded = validate::job(ar, job, compiler, FIXTURE_CAPS)?;
    validate::result(ar, success, job, &decoded)?;
    validate::result(ar, failure, job, &decoded)?;
    Ok((compiler, job, success, failure))
}

fn vectors() -> schema::Result<String> {
    let mut ar = Reduction::<4096>::new();
    let (compiler, job, success, failure) = fixture(&mut ar)?;
    let mut vectors = Vec::new();
    for (name, root) in [
        ("compiler_profile_fixture", compiler),
        ("job", job),
        ("success_14", success),
        ("diagnostic", failure),
    ] {
        let encoded =
            nox::artifact::encode(&ar, root, TRANSPORT).map_err(|e| format!("encode: {e:?}"))?;
        vectors.push(Vector {
            name,
            root_particle_le: hex(&encoded[8..40]),
            bytes: encoded.len(),
            artifact_hex: hex(&encoded),
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
            return Err("job vectors are stale".into());
        }
    } else {
        std::fs::write(args.output, json)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "selfhost_jobs/tests.rs"]
mod tests;
