use super::{
    admission::{self, Admission},
    capture::Capture,
    records::{self, Record},
    session::Session,
    transport, CertificateLimits, Context, ProverObservations,
};
use crate::structured::{CompilerReport, RunLimits};
use nox::{
    artifact,
    sequential::{
        self,
        observe::{self, CaptureLimits},
    },
    Outcome,
};
use serde::Serialize;
use std::{
    io::{Read, Write},
    time::{Duration, Instant},
};
use zheng::execution::disclosed::stream::VerifiedTerminal;

pub const FORMAT: &str = "joy-nox-disclosed-compiler-v1";

#[derive(Debug, Serialize)]
pub struct Report {
    pub format: &'static str,
    pub program_particle: String,
    pub input_particle: String,
    pub output_particle: String,
    pub charged_reductions: u64,
    pub invocations: u64,
    pub logical_peak_frames: u32,
    pub expanded_steps: u64,
    pub semantic_events: u64,
    pub records: u64,
    pub transport: transport::Stats,
    pub elapsed_micros: u128,
    pub disclosure: &'static str,
    pub physical_resource_claim: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prover_observations: Option<ProverObservations>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compiler_job: Option<CompilerReport>,
}

#[derive(Debug)]
pub struct CertificateResult {
    pub output: Vec<u8>,
    pub compiled: Option<Vec<u8>>,
    pub report: Report,
}

fn report(
    context: Context,
    checked: &VerifiedTerminal,
    records: u64,
    transport: transport::Stats,
    started: Instant,
    compiler_job: Option<CompilerReport>,
) -> Report {
    let summary = checked.summary();
    Report {
        format: FORMAT,
        program_particle: text(context.program),
        input_particle: text(context.object),
        output_particle: text(summary.result().particle()),
        charged_reductions: summary.cost(),
        invocations: summary.occurrences(),
        logical_peak_frames: summary.peak_frames(),
        expanded_steps: summary.steps(),
        semantic_events: checked.events(),
        records,
        transport,
        elapsed_micros: started.elapsed().as_micros(),
        disclosure: "complete public witness",
        physical_resource_claim: "unattested",
        prover_observations: None,
        compiler_job,
    }
}
fn text(particle: [u64; 4]) -> String {
    particle
        .into_iter()
        .flat_map(u64::to_le_bytes)
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub(super) fn prove<const N: usize, W: Write>(
    program: &[u8],
    input: &[u8],
    output: W,
    host: RunLimits,
    limits: CertificateLimits,
) -> Result<(W, CertificateResult), String> {
    let started = Instant::now();
    let expires = started + Duration::from_millis(host.time_ms);
    let mut admitted = Admission::<N>::load(program, input, host, expires)?;
    let context = admitted.context;
    let session = Session::new(context, limits)?;
    let writer = transport::Writer::new(output, context.digest(), limits.transport())
        .map_err(|e| format!("certificate transport: {e}"))?;
    let mut capture = Capture::new(writer, session, context)?;
    let collection_work = host.compaction.map_or(0, |policy| policy.collection_work);
    let events = limits
        .steps
        .checked_add(limits.records)
        .and_then(|n| n.checked_add(2))
        .ok_or("capture allowance overflow")?;
    let observed = observe::reduce_compacting_observed_v2_controlled(
        &mut admitted.ar,
        admitted.object,
        admitted.formula,
        context.budget,
        sequential::CompactionLimits {
            max_frames: context.frames,
            max_total_allocations: u64::from(admitted.allocations),
            max_collection_work: collection_work,
        },
        CaptureLimits {
            max_events: events,
            max_bytes: events.checked_mul(512).ok_or("capture byte allowance")?,
            max_work: events.checked_mul(32).ok_or("capture work allowance")?,
        },
        &mut capture,
        &mut || Instant::now() >= expires,
    )
    .map_err(|e| format!("certificate execution/capture: {:?}", e.kind))?;
    let (root, remaining) = match observed.execution.outcome {
        Outcome::Ok(root, remaining) => (root, remaining),
        other => {
            return Err(format!(
                "certificate requires successful pure execution: {other:?}"
            ))
        }
    };
    admission::deadline(expires)?;
    let observations = capture.stats();
    let (mut writer, session) = capture.complete()?;
    let result = admission::particle(&admitted.ar, root)?;
    let cost = context
        .budget
        .checked_sub(remaining)
        .ok_or("runtime remaining budget")?;
    let record_count = session
        .record_count()
        .checked_add(1)
        .ok_or("record count overflow")?;
    let checked = session.terminal(result, cost)?;
    if checked.summary().steps()
        != observed
            .execution
            .stats
            .evaluator_checkpoints
            .saturating_sub(1)
    {
        return Err("semantic/native step count mismatch".into());
    }
    let (output, compiled, compiler_job) = admitted.result(root, expires)?;
    writer.write_all(&[6]).map_err(|e| e.to_string())?;
    for limb in result {
        writer
            .write_all(&limb.to_le_bytes())
            .map_err(|e| e.to_string())?;
    }
    writer
        .write_all(&cost.to_le_bytes())
        .map_err(|e| e.to_string())?;
    let length = u32::try_from(output.len()).map_err(|_| "result length overflow")?;
    writer
        .write_all(&length.to_le_bytes())
        .and_then(|_| writer.write_all(&output))
        .map_err(|e| e.to_string())?;
    admission::deadline(expires)?;
    let (writer, stats) = writer
        .finish()
        .map_err(|e| format!("certificate completion: {e}"))?;
    admission::deadline(expires)?;
    let mut report = report(
        context,
        &checked,
        record_count,
        stats,
        started,
        compiler_job,
    );
    report.prover_observations = Some(observations);
    Ok((
        writer,
        CertificateResult {
            output,
            compiled,
            report,
        },
    ))
}

pub(super) fn verify<const N: usize, R: Read>(
    program: &[u8],
    input: &[u8],
    proof: R,
    host: RunLimits,
    limits: CertificateLimits,
) -> Result<CertificateResult, String> {
    let started = Instant::now();
    let expires = started + Duration::from_millis(host.time_ms);
    let mut admitted = Admission::<N>::load(program, input, host, expires)?;
    let context = admitted.context;
    let mut session = Session::new(context, limits)?;
    let mut reader = transport::Reader::new(proof, context.digest(), limits.transport())
        .map_err(|e| format!("certificate transport: {e}"))?;
    loop {
        admission::deadline(expires)?;
        let tag = records::read::<1>(&mut reader)?[0];
        if tag == 6 {
            break;
        }
        let record = Record::read(tag, &mut reader)?;
        session.apply(&record)?;
    }
    let mut result = [0; 4];
    for limb in &mut result {
        *limb = records::word(&mut reader)?;
    }
    let cost = records::word(&mut reader)?;
    let record_count = session
        .record_count()
        .checked_add(1)
        .ok_or("record count overflow")?;
    let checked = session.terminal(result, cost)?;
    let length = records::index(&mut reader)? as usize;
    if length == 0 || length > admitted.transport.max_bytes {
        return Err("certificate result byte limit".into());
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|_| "certificate result allocation")?;
    bytes.resize(length, 0);
    reader
        .read_exact(&mut bytes)
        .map_err(|e| format!("certificate result: {e}"))?;
    let stats = reader
        .finish()
        .map_err(|e| format!("certificate completion: {e}"))?;
    admission::deadline(expires)?;
    let root = artifact::decode(&mut admitted.ar, &bytes, admitted.transport)
        .map_err(|e| format!("certificate result noun: {e:?}"))?;
    if admission::particle(&admitted.ar, root)? != result {
        return Err("certificate result identity mismatch".into());
    }
    let (output, compiled, compiler_job) = admitted.result(root, expires)?;
    Ok(CertificateResult {
        output,
        compiled,
        report: report(
            context,
            &checked,
            record_count,
            stats,
            started,
            compiler_job,
        ),
    })
}
