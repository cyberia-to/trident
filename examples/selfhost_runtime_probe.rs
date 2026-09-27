//! Bounded component diagnostic. This does not apply Joy admission or certify a milestone.
use clap::Parser;
use nox::{artifact, sequential, NoTrace, Outcome, Reduction};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use trident::NATIVE_ARTIFACT_LIMITS as LIMITS;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    artifact: PathBuf,
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 100_000_000)]
    budget: u64,
    #[arg(long, default_value_t = 3_145_728)]
    nodes: u32,
    #[arg(long, default_value_t = 300)]
    seconds: u64,
    #[arg(long)]
    cached: bool,
    /// Wrap input as [JOB1 stop-stage] for native_selfhost_prefix.tri only.
    #[arg(long)]
    stop_stage: Option<u64>,
}

fn run<const N: usize>(args: &Args) -> Result<(), String> {
    if args.output.exists() || args.output.with_extension("dag").exists() {
        return Err("choose a new output path; existing evidence is preserved".into());
    }
    let started = Instant::now();
    let mut arena =
        Reduction::<N>::try_new_boxed().map_err(|e| format!("arena allocation: {e:?}"))?;
    if !arena.limit_allocations(args.nodes) {
        return Err("logical arena limit exceeds physical capacity".into());
    }
    let read = |path: &PathBuf| -> Result<Vec<u8>, String> {
        let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.len() > LIMITS.max_bytes as u64 {
            return Err("input must be a bounded regular artifact file".into());
        }
        let mut bytes = Vec::new();
        file.take(LIMITS.max_bytes as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > LIMITS.max_bytes {
            return Err("input artifact grew past byte bound".into());
        }
        Ok(bytes)
    };
    let program = artifact::decode(&mut arena, &read(&args.artifact)?, LIMITS)
        .map_err(|e| format!("program: {e:?}"))?;
    let mut input = artifact::decode(&mut arena, &read(&args.input)?, LIMITS)
        .map_err(|e| format!("input: {e:?}"))?;
    if let Some(stage) = args.stop_stage {
        let stage = arena
            .atom(nebu::Goldilocks::new(stage))
            .ok_or("stage allocation")?;
        input = arena.pair(input, stage).ok_or("input allocation")?;
    }
    let identity = |value| -> Result<String, String> {
        let digest = arena.digest(value).ok_or("missing input identity")?;
        Ok(nox::data::digest_bytes(digest)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect())
    };
    let program_particle = identity(program)?;
    let subject_particle = identity(input)?;
    let mut fields = arena.tail(program).ok_or("ART1 fields")?;
    for _ in 0..3 {
        fields = arena.tail(fields).ok_or("ART1 metadata")?;
    }
    let formula = arena.head(fields).ok_or("ART1 formula")?;
    let loaded = arena.count();
    let setup_ms = started.elapsed().as_millis();
    let start = Instant::now();
    let deadline = Duration::from_secs(args.seconds);
    let mut checkpoints = 0u64;
    let mut next_report = Duration::from_secs(60);
    let mut cancelled = || {
        checkpoints += 1;
        let elapsed = start.elapsed();
        if elapsed >= next_report {
            eprintln!(
                "runtime probe: {}s, {checkpoints} evaluator checkpoints",
                elapsed.as_secs()
            );
            next_report += Duration::from_secs(60);
        }
        elapsed >= deadline
    };
    let limits = sequential::Limits { max_frames: 65536 };
    let execution = if args.cached {
        sequential::reduce_cached_controlled(
            &mut arena,
            input,
            formula,
            args.budget,
            limits,
            &mut cancelled,
        )
    } else {
        sequential::reduce_controlled(
            &mut arena,
            input,
            formula,
            args.budget,
            limits,
            &mut NoTrace,
            &mut cancelled,
        )
    };
    let elapsed_ms = start.elapsed().as_millis();
    let mut halt_budget = None;
    let (status, remaining, result, peak, outcome) = match execution {
        Ok(execution) => {
            let (status, remaining, result) = match execution.outcome {
                Outcome::Ok(result, left) => ("returned", Some(left), Some(result)),
                Outcome::Halt(left) => {
                    // A child's propagated halt budget is not the root remainder.
                    halt_budget = Some(left);
                    ("budget_exhausted", None, None)
                }
                Outcome::Error(_) => ("execution_error", None, None),
            };
            (
                status,
                remaining,
                result,
                Some(execution.peak_frames),
                format!("{:?}", execution.outcome),
            )
        }
        Err(error) => ("host_limit", None, None, None, format!("{error:?}")),
    };
    let mut words = Vec::new();
    let mut encode_error = None;
    if let Some(result) = result {
        // The prefix fixture returns five scalar words. Other outputs are left opaque.
        if args.stop_stage.is_some() {
            let mut rest = result;
            for _ in 0..4 {
                let head = arena.head(rest).ok_or("prefix result head")?;
                words.push(
                    arena
                        .atom_value(head)
                        .ok_or("prefix result scalar")?
                        .as_u64(),
                );
                rest = arena.tail(rest).ok_or("prefix result tail")?;
            }
            words.push(arena.atom_value(rest).ok_or("prefix result end")?.as_u64());
        }
        match artifact::encode(&arena, result, LIMITS) {
            Ok(bytes) => write_new(&args.output.with_extension("dag"), &bytes)?,
            Err(error) => encode_error = Some(format!("{error:?}")),
        }
    }
    let report = serde_json::json!({
        "scope": "Direct nox component diagnostic; not Joy admission, a usable C2, or a self-hosting milestone",
        "artifact": args.artifact, "input": args.input,
        "stop_stage": args.stop_stage,
        "program_particle": program_particle, "executed_subject_particle": subject_particle,
        "command": std::env::args_os().map(|s| s.to_string_lossy().into_owned()).collect::<Vec<_>>(),
        "budget": args.budget, "logical_nodes": args.nodes, "physical_nodes": N,
        "max_seconds": args.seconds, "max_frames": limits.max_frames,
        "cached": args.cached,
        "cache_bytes": if args.cached { sequential::finalizer_cache_storage_bytes() } else { 0 },
        "loaded_nodes": loaded, "allocated_nodes": arena.count(),
        "remaining_budget": remaining, "charged_reductions": remaining.map(|r| args.budget - r),
        "propagated_halt_budget": halt_budget,
        "peak_frames": peak, "evaluator_checkpoints": checkpoints,
        "status": status, "outcome": outcome, "setup_ms": setup_ms, "elapsed_ms": elapsed_ms,
        "prefix_words": words, "encode_error": encode_error,
    });
    let receipt = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())? + "\n";
    write_new(&args.output, receipt.as_bytes())?;
    println!("{report}");
    Ok(())
}

fn main() -> Result<(), String> {
    let args = Args::parse();
    if args.output == args.output.with_extension("dag") {
        return Err("receipt and result must use distinct paths; choose a .json receipt".into());
    }
    if args.budget == 0 || args.budget > 10_000_000_000 || args.seconds == 0 || args.seconds > 7200
    {
        return Err("diagnostic bounds: budget 1..10B, seconds 1..7200".into());
    }
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(move || match args.nodes {
            1..=3_145_728 => run::<{ 1 << 22 }>(&args),
            3_145_729..=12_582_912 => run::<{ 1 << 24 }>(&args),
            12_582_913..=25_165_824 => run::<{ 1 << 25 }>(&args),
            _ => Err("diagnostic arena: 1..25165824 logical nodes".into()),
        })
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|_| "probe thread panicked")?
}

fn write_new(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .and_then(|mut file| file.write_all(bytes))
        .map_err(|e| format!("{}: {e}", path.display()))
}
