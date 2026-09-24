#![allow(dead_code)]
use nebu::Goldilocks;
use nox::{artifact, sequential, NoTrace, Outcome, Reduction};
use trident::{CompileOptions, RawArtifact, RAW_ARTIFACT_LIMITS as LIMITS};

pub const ARENA: usize = 1 << 18;
pub fn worker<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap()
}
pub fn compile(source: &str) -> RawArtifact {
    trident::compile_raw_artifact(source, "control.tri", &CompileOptions::default(), LIMITS)
        .unwrap()
}
pub fn program(body: &str) -> String {
    format!("program control\nfn main(input: Noun) -> Noun {{ {body} }}")
}
pub fn run(
    bytes: &[u8],
    input: u64,
    budget: u64,
    frames: u32,
    nodes: u32,
) -> Result<(u64, u64, u32, u32), String> {
    run_traced(bytes, input, budget, frames, nodes, &mut NoTrace)
}
pub fn run_traced(
    bytes: &[u8],
    input: u64,
    budget: u64,
    frames: u32,
    nodes: u32,
    tracer: &mut impl nox::trace::Tracer,
) -> Result<(u64, u64, u32, u32), String> {
    let mut arena = Reduction::<ARENA>::new();
    assert!(arena.limit_allocations(nodes));
    let root = artifact::decode(&mut arena, bytes, LIMITS).map_err(|e| format!("decode {e:?}"))?;
    let mut cursor = arena.tail(root).unwrap();
    for _ in 0..3 {
        cursor = arena.tail(cursor).unwrap();
    }
    let formula = arena.head(cursor).unwrap();
    let input = arena
        .atom(Goldilocks::new(input))
        .ok_or("input allocation")?;
    let execution = sequential::reduce(
        &mut arena,
        input,
        formula,
        budget,
        sequential::Limits { max_frames: frames },
        tracer,
    )
    .map_err(|e| format!("{e:?}"))?;
    match execution.outcome {
        Outcome::Ok(result, left) => Ok((
            arena.atom_value(result).ok_or("non-atom result")?.as_u64(),
            budget - left,
            execution.peak_frames,
            arena.count(),
        )),
        result => Err(format!("{result:?}")),
    }
}
pub fn evaluate(source: &str, input: u64) -> u64 {
    let source = source.to_string();
    worker(move || {
        run(
            &compile(&source).bytes,
            input,
            10_000_000,
            65536,
            LIMITS.max_nodes,
        )
        .unwrap()
        .0
    })
}
