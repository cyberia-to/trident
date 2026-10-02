use super::{CompilerCaps, DEFAULT_ARENA_NODES, MAX_ARENA_NODES, MAX_BUDGET, MAX_TIME_MS};
use nox::artifact;

/// Opt-in resident storage and collection-work policy for pure execution.
#[derive(Debug, Clone, Copy)]
pub struct CompactionPolicy {
    pub resident_nodes: u32,
    pub collection_work: u64,
}

const COMPACT_ALLOCATIONS: u64 = 1_000_000_000;
const COMPACT_REDUCTIONS: u64 = 20_000_000_000;
const COMPACT_WORK: u64 = 10_000_000_000;
const COMPACT_TIME_MS: u64 = 7_200_000;

#[derive(Debug, Clone, Copy)]
pub struct RunLimits {
    pub budget: u64,
    pub arena_nodes: u32,
    pub frames: u32,
    pub artifact_bytes: usize,
    pub artifact_nodes: u32,
    pub artifact_depth: u32,
    pub time_ms: u64,
    pub compiler: CompilerCaps,
    pub compaction: Option<CompactionPolicy>,
}

impl Default for RunLimits {
    fn default() -> Self {
        Self {
            budget: 1_000_000,
            arena_nodes: DEFAULT_ARENA_NODES,
            frames: 16_384,
            artifact_bytes: 16 << 20,
            artifact_nodes: 196_608,
            artifact_depth: 4096,
            time_ms: 30_000,
            compiler: CompilerCaps::default(),
            compaction: None,
        }
    }
}

impl RunLimits {
    pub fn validate(self) -> Result<(), String> {
        let (max_budget, max_nodes, max_time) = if let Some(policy) = self.compaction {
            positive(
                "resident_nodes",
                u64::from(policy.resident_nodes),
                u64::from(MAX_ARENA_NODES),
            )?;
            positive("collection_work", policy.collection_work, COMPACT_WORK)?;
            (COMPACT_REDUCTIONS, COMPACT_ALLOCATIONS, COMPACT_TIME_MS)
        } else {
            (MAX_BUDGET, u64::from(MAX_ARENA_NODES), MAX_TIME_MS)
        };
        for (name, value, max) in [
            ("budget", self.budget, max_budget),
            ("arena_nodes", self.arena_nodes as u64, max_nodes),
            ("frames", self.frames as u64, 65_536),
            ("artifact_bytes", self.artifact_bytes as u64, 16 << 20),
            ("artifact_nodes", self.artifact_nodes as u64, 196_608),
            ("artifact_depth", self.artifact_depth as u64, 4096),
            ("time_ms", self.time_ms, max_time),
        ] {
            positive(name, value, max)?;
        }
        self.compiler.validate()
    }

    pub(super) fn resident_nodes(self) -> u32 {
        self.compaction.map_or(self.arena_nodes, |policy| {
            policy.resident_nodes.min(self.arena_nodes)
        })
    }

    pub(super) fn transport(self) -> artifact::Limits {
        artifact::Limits {
            max_bytes: self.artifact_bytes,
            max_nodes: self.artifact_nodes,
            max_depth: self.artifact_depth,
        }
    }
}

fn positive(name: &str, value: u64, max: u64) -> Result<(), String> {
    if value == 0 || value > max {
        return Err(format!("limit {name} must be in 1..={max}"));
    }
    Ok(())
}
