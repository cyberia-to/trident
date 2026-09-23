//! SH0.4 observed runtime baseline: hand-built continuations, no source compiler.
use clap::Parser;
use nebu::Goldilocks;
use nox::{NoTrace, NullCalls, Order, Outcome, Reduction, Tracer, VecTrace};
use serde::Serialize;
use std::path::PathBuf;

const ARENA: usize = 1 << 16;
const BUDGET: u64 = 1_000_000;
type Result<T> = std::result::Result<T, String>;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    check: bool,
}

fn atom(ar: &mut Reduction<ARENA>, v: u64) -> Result<Order> {
    ar.atom(Goldilocks::new(v)).ok_or("arena full".into())
}
fn pair(ar: &mut Reduction<ARENA>, a: Order, b: Order) -> Result<Order> {
    ar.pair(a, b).ok_or("arena full".into())
}
fn op(ar: &mut Reduction<ARENA>, tag: u64, body: Order) -> Result<Order> {
    let t = atom(ar, tag)?;
    pair(ar, t, body)
}
fn binary(ar: &mut Reduction<ARENA>, tag: u64, a: Order, b: Order) -> Result<Order> {
    let body = pair(ar, a, b)?;
    op(ar, tag, body)
}

// subject = [code [remaining accumulator]]; code selects itself from subject.
// There is one immutable loop body, independent of the runtime iteration count.
fn loop_formula(ar: &mut Reduction<ARENA>) -> Result<Order> {
    let zero = atom(ar, 0)?;
    let one = atom(ar, 1)?;
    let two = atom(ar, 2)?;
    let six = atom(ar, 6)?;
    let seven = atom(ar, 7)?;
    let q0 = op(ar, 1, zero)?;
    let q1 = op(ar, 1, one)?;
    let code = op(ar, 0, two)?;
    let remaining = op(ar, 0, six)?;
    let acc = op(ar, 0, seven)?;
    let done = binary(ar, 9, remaining, q0)?;
    let next = binary(ar, 6, remaining, q1)?;
    let acc_next = binary(ar, 5, acc, q1)?;
    let state = binary(ar, 3, next, acc_next)?;
    let subject = binary(ar, 3, code, state)?;
    let again = binary(ar, 2, subject, code)?;
    let arms = pair(ar, acc, again)?;
    binary(ar, 4, done, arms)
}

#[derive(Serialize)]
struct Probe {
    iterations: u64,
    formula_particle: String,
    status: String,
    output: Option<u64>,
    reductions: Option<u64>,
    traced_rows: usize,
    allocated_nodes: u32,
    run_only_agrees: bool,
}

fn run<T: Tracer>(iterations: u64, tracer: &mut T) -> Result<(String, Outcome, Reduction<ARENA>)> {
    let mut ar = Reduction::<ARENA>::new();
    let formula = loop_formula(&mut ar)?;
    let identity = nox::data::digest_bytes(ar.digest(formula).ok_or("missing digest")?)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let zero = atom(&mut ar, 0)?;
    let count = atom(&mut ar, iterations)?;
    let state = pair(&mut ar, count, zero)?;
    let subject = pair(&mut ar, formula, state)?;
    let outcome = nox::reduce(&mut ar, subject, formula, BUDGET, &NullCalls, tracer);
    Ok((identity, outcome, ar))
}

fn observation(ar: &Reduction<ARENA>, outcome: Outcome) -> (String, Option<u64>, Option<u64>) {
    match outcome {
        Outcome::Ok(r, remaining) => (
            "ok".into(),
            ar.atom_value(r).map(|v| v.as_u64()),
            Some(BUDGET - remaining),
        ),
        Outcome::Halt(_) => ("halt".into(), None, None),
        Outcome::Error(e) => (format!("error:{e:?}"), None, None),
    }
}

fn probes() -> Result<String> {
    let mut probes = Vec::new();
    for iterations in [0, 1, 16, 4097] {
        let mut traced = VecTrace::default();
        let (identity, outcome, ar) = run(iterations, &mut traced)?;
        let observed = observation(&ar, outcome);
        let allocated_nodes = ar.count();
        drop(ar);
        let (_, outcome, ar) = run(iterations, &mut NoTrace)?;
        let agrees = observed == observation(&ar, outcome) && allocated_nodes == ar.count();
        if !agrees {
            return Err("trace mode changed execution".into());
        }
        if iterations < 4097
            && (observed.1 != Some(iterations) || observed.2 != Some(15 * iterations + 5))
        {
            return Err(format!("unexpected success/cost: {observed:?}"));
        }
        if iterations == 4097 && observed.0 != "error:Malformed" {
            return Err(format!("legacy depth baseline changed: {observed:?}"));
        }
        probes.push(Probe {
            iterations,
            formula_particle: identity,
            status: observed.0,
            output: observed.1,
            reductions: observed.2,
            traced_rows: traced.0.len(),
            allocated_nodes,
            run_only_agrees: agrees,
        });
    }
    serde_json::to_string_pretty(&serde_json::json!({
        "profile": "recursive-sequential-pure-l1-baseline",
        "budget": BUDGET, "arena_slots": ARENA, "usable_nodes": ARENA / 4 * 3,
        "arena_reserved_bytes": std::mem::size_of::<Reduction<ARENA>>(),
        "trace_row_bytes": std::mem::size_of::<nox::TraceRow>(),
        "worker_stack_bytes": 128 * 1024 * 1024, "probes": probes
    }))
    .map(|s| s + "\n")
    .map_err(|e| e.to_string())
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let json = std::thread::Builder::new()
        .stack_size(128 * 1024 * 1024)
        .spawn(probes)?
        .join()
        .map_err(|_| "probe worker panicked")??;
    if args.check {
        if std::fs::read_to_string(&args.output)? != json {
            return Err("runtime baseline is stale".into());
        }
    } else {
        std::fs::write(args.output, json)?;
    }
    Ok(())
}
