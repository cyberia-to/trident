use super::{records::Record, CertificateLimits, Context};
use zheng::execution::disclosed::{
    memory::{Memory, Value, VerifiedNode},
    stream::{
        CacheHandle, Claim, InputKey, Limits, SemanticStream, VerifiedSummary, VerifiedTerminal,
    },
};

pub(super) struct Session {
    memory: Memory,
    stream: SemanticStream,
    context: Context,
    limits: CertificateLimits,
    epoch: Option<u64>,
    pending: u32,
    records: u64,
    poisoned: bool,
}

impl Session {
    pub(super) fn new(context: Context, limits: CertificateLimits) -> Result<Self, String> {
        limits.validate()?;
        if context.frames == 0
            || context.frames > 65_536
            || context.budget == 0
            || context.budget > 20_000_000_000
            || context.profile > 1
        {
            return Err("certificate context allowance".into());
        }
        let stream = SemanticStream::new(
            InputKey {
                object: context.object,
                formula: context.formula,
            },
            Limits {
                max_frames: context.frames,
                max_cache_slots: limits.cache_slots,
                max_buffer_bytes: SemanticStream::storage_bytes(context.frames, limits.cache_slots)
                    .ok_or("semantic storage overflow")?,
                max_cost: context.budget,
                max_steps: limits.steps,
                max_events: limits.records,
            },
        )
        .map_err(|e| format!("semantic stream: {e:?}"))?;
        Ok(Self {
            memory: memory(limits.nouns)?,
            stream,
            context,
            limits,
            epoch: None,
            pending: 0,
            records: 0,
            poisoned: false,
        })
    }
    pub(super) fn noun_limit(&self) -> u32 {
        self.limits.nouns
    }
    pub(super) fn cache_slots(&self) -> u32 {
        self.limits.cache_slots
    }
    pub(super) fn steps_limit(&self) -> u64 {
        self.limits.steps
    }
    pub(super) fn node_count(&self) -> u32 {
        self.memory.len()
    }
    pub(super) fn record_count(&self) -> u64 {
        self.records
    }
    pub(super) fn node(&self, id: u32) -> Result<VerifiedNode, String> {
        self.memory
            .view(self.memory.len())
            .and_then(|v| v.get(id).copied())
            .map_err(|e| format!("noun: {e:?}"))
    }
    pub(super) fn cache(&self, slot: u32) -> Option<(CacheHandle, VerifiedSummary)> {
        if self.poisoned {
            None
        } else {
            self.stream.cache(slot)
        }
    }
    pub(super) fn apply(&mut self, record: &Record) -> Result<Option<CacheHandle>, String> {
        if self.poisoned {
            return Err("certificate session previously failed".into());
        }
        self.poisoned = true;
        self.charge()?;
        let handle = match *record {
            Record::Reset { epoch, nodes } => {
                let expected = self
                    .epoch
                    .map_or(Some(0), |e| e.checked_add(1))
                    .ok_or("epoch overflow")?;
                if self.pending != 0 || epoch != expected || nodes == 0 || nodes > self.limits.nouns
                {
                    return Err("noun epoch/count mismatch".into());
                }
                self.memory = memory(self.limits.nouns)?;
                self.epoch = Some(epoch);
                self.pending = nodes;
                None
            }
            Record::Atom(v) => {
                self.append(Value::Atom(v))?;
                None
            }
            Record::Pair { left, right } => {
                self.append(Value::Pair { left, right })?;
                None
            }
            _ => {
                if self.epoch.is_none() || self.pending != 0 {
                    return Err("incomplete noun snapshot".into());
                }
                let view = self
                    .memory
                    .view(self.memory.len())
                    .map_err(|e| format!("noun view: {e:?}"))?;
                match *record {
                    Record::Enter { object, formula } => {
                        self.stream.enter(&view, object, formula).map(|_| None)
                    }
                    Record::Finish { result, cache_slot } => {
                        self.stream.finish(&view, result, cache_slot)
                    }
                    Record::Reuse(handle) => self.stream.reuse(handle).map(|_| None),
                    _ => return Err("certificate record dispatch".into()),
                }
                .map_err(|e| format!("semantic record: {e:?}"))?
            }
        };
        self.poisoned = false;
        Ok(handle)
    }
    fn append(&mut self, value: Value) -> Result<(), String> {
        if self.epoch.is_none() {
            return Err("noun before initial reset".into());
        }
        self.memory
            .append_value(value)
            .map_err(|e| format!("noun definition: {e:?}"))?;
        self.pending = self.pending.saturating_sub(1);
        Ok(())
    }
    fn charge(&mut self) -> Result<(), String> {
        self.records = self
            .records
            .checked_add(1)
            .filter(|n| *n <= self.limits.records)
            .ok_or("certificate record allowance")?;
        Ok(())
    }
    pub(super) fn terminal(
        mut self,
        result: [u64; 4],
        cost: u64,
    ) -> Result<VerifiedTerminal, String> {
        if self.poisoned || self.epoch.is_none() || self.pending != 0 {
            return Err("incomplete certificate session".into());
        }
        self.charge()?;
        self.stream
            .bind_terminal(Claim {
                object: self.context.object,
                formula: self.context.formula,
                result,
                cost,
                budget: self.context.budget,
                max_frames: self.context.frames,
            })
            .map_err(|e| format!("semantic terminal: {e:?}"))
    }
}

fn memory(nouns: u32) -> Result<Memory, String> {
    let bytes = (nouns as usize)
        .checked_mul(std::mem::size_of::<VerifiedNode>())
        .ok_or("noun storage overflow")?;
    Memory::new(nouns, bytes).map_err(|e| format!("noun memory: {e:?}"))
}
