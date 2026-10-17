use super::{
    super::{job, job_result, reader, CompilerReport, RunLimits},
    Context,
};
use nox::{artifact, Order, Reduction};
use std::time::Instant;

pub(super) struct Admission<const N: usize> {
    pub ar: Box<Reduction<N>>,
    pub object: Order,
    pub formula: Order,
    pub job: Option<job::Job>,
    pub context: Context,
    pub allocations: u32,
    pub transport: artifact::Limits,
}

impl<const N: usize> Admission<N> {
    pub(super) fn load(
        program: &[u8],
        input: &[u8],
        limits: RunLimits,
        expires: Instant,
    ) -> Result<Self, String> {
        limits.validate()?;
        if program.len() > limits.artifact_bytes || input.len() > limits.artifact_bytes {
            return Err("certificate input file size limit".into());
        }
        let mut ar = Reduction::<N>::try_new_boxed().map_err(|e| format!("arena: {e}"))?;
        if !ar.limit_allocations(limits.resident_nodes()) {
            return Err("resident allowance".into());
        }
        let program = artifact::decode(&mut ar, program, limits.transport())
            .map_err(|e| format!("program: {e:?}"))?;
        deadline(expires)?;
        let mut r = reader::Reader {
            ar: &ar,
            remaining: limits.compiler.validation_visits,
            sequence_limit: limits.compiler.sequence_length,
            deadline: expires,
        };
        let (formula, profile) = job::program(&mut r, program)?;
        let program_visits = limits.compiler.validation_visits - r.remaining;
        let object = artifact::decode(&mut ar, input, limits.transport())
            .map_err(|e| format!("input: {e:?}"))?;
        deadline(expires)?;
        let job = if profile == 1 {
            Some(job::admit(
                &ar,
                object,
                program,
                limits,
                expires,
                program_visits,
            )?)
        } else {
            None
        };
        let (budget, frames, allocations, transport) = job.as_ref().map_or(
            (
                limits.budget,
                limits.frames,
                limits.arena_nodes,
                limits.transport(),
            ),
            |j| {
                (
                    j.limits.reductions,
                    j.limits.evaluator_frames,
                    j.limits.arena_nodes,
                    j.limits.transport(),
                )
            },
        );
        if !ar.limit_allocations(allocations.min(limits.resident_nodes())) {
            return Err("admitted arena allowance".into());
        }
        let context = Context {
            program: particle(&ar, program)?,
            object: particle(&ar, object)?,
            formula: particle(&ar, formula)?,
            profile: profile as u8,
            budget,
            frames,
        };
        Ok(Self {
            ar,
            object,
            formula,
            job,
            context,
            allocations,
            transport,
        })
    }
    pub(super) fn result(
        &mut self,
        result: Order,
        expires: Instant,
    ) -> Result<(Vec<u8>, Option<Vec<u8>>, Option<CompilerReport>), String> {
        deadline(expires)?;
        let (report, compiled) = match self.job.take() {
            Some(job) => {
                let (report, compiled) = job_result::validate(&self.ar, result, job, expires)?;
                (Some(report), compiled)
            }
            None => (None, None),
        };
        let output = artifact::encode(&self.ar, result, self.transport)
            .map_err(|e| format!("result: {e:?}"))?;
        deadline(expires)?;
        Ok((output, compiled, report))
    }
}

pub(super) fn particle<const N: usize>(ar: &Reduction<N>, id: Order) -> Result<[u64; 4], String> {
    ar.digest(id)
        .map(|d| d.map(|v| v.canonicalize().as_u64()))
        .ok_or_else(|| "missing particle".into())
}
pub(super) fn deadline(expires: Instant) -> Result<(), String> {
    if Instant::now() >= expires {
        Err("certificate deadline exceeded".into())
    } else {
        Ok(())
    }
}
