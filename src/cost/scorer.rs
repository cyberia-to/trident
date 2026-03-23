// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Dynamic TASM table profiler.
//!
//! Counts actual table row increments per TASM instruction to compute
//! the cliff-aware proving cost. Lightweight — no memory model, no
//! hash computation. Just table height counters.

/// Table heights for Triton VM's 6 tracked Algebraic Execution Tables.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TableProfile {
    /// [processor, hash, u32, op_stack, ram, jump_stack]
    pub heights: [u64; 6],
}

const PROC: usize = 0;
const HASH: usize = 1;
const U32: usize = 2;
const OPST: usize = 3;
const RAM: usize = 4;
const JUMP: usize = 5;

impl TableProfile {
    /// Maximum height across all tables.
    pub fn max_height(&self) -> u64 {
        self.heights.iter().copied().max().unwrap_or(0)
    }

    /// Padded height: next power of 2 above max table height.
    pub fn padded_height(&self) -> u64 {
        let max = self.max_height();
        if max == 0 {
            return 1;
        }
        max.next_power_of_two()
    }

    /// Cliff-aware proving cost = padded_height.
    pub fn cost(&self) -> u64 {
        self.padded_height()
    }

    /// Index of the tallest table.
    pub fn dominant_table(&self) -> usize {
        self.heights
            .iter()
            .enumerate()
            .max_by_key(|(_, h)| *h)
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Table name for display.
    pub fn table_name(idx: usize) -> &'static str {
        match idx {
            PROC => "processor",
            HASH => "hash",
            U32 => "u32",
            OPST => "op_stack",
            RAM => "ram",
            JUMP => "jump_stack",
            _ => "unknown",
        }
    }

    /// Whether the neural candidate crossed a power-of-2 boundary
    /// that the baseline did not.
    pub fn is_cliff_jump(&self, baseline: &TableProfile) -> bool {
        self.padded_height() < baseline.padded_height()
    }

    /// Whether the dominant (tallest) table changed between baseline and this profile.
    pub fn is_table_rebalance(&self, baseline: &TableProfile) -> bool {
        self.dominant_table() != baseline.dominant_table()
            && self.max_height() < baseline.max_height()
    }
}

/// Profile a sequence of TASM instruction lines, counting table row increments.
///
/// Instructions are whitespace-trimmed. Labels, comments, and blank lines
/// are ignored. Returns cumulative table heights.
pub fn profile_tasm(lines: &[&str]) -> TableProfile {
    let mut p = TableProfile::default();
    for line in lines {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") || t.ends_with(':') {
            continue;
        }
        profile_instruction(t, &mut p);
    }
    p
}

/// Profile from a newline-separated TASM string.
pub fn profile_tasm_str(tasm: &str) -> TableProfile {
    let lines: Vec<&str> = tasm.lines().collect();
    profile_tasm(&lines)
}

/// Add table row increments for a single TASM instruction.
fn profile_instruction(instr: &str, p: &mut TableProfile) {
    let parts: Vec<&str> = instr.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }
    let op = parts[0];
    match op {
        // Stack operations: 1 proc + 1 opstack
        "push" | "pop" | "dup" | "swap" | "pick" | "place" => {
            p.heights[PROC] += 1;
            p.heights[OPST] += 1;
        }

        // Arithmetic: 1 proc + 1 opstack
        "add" | "mul" | "eq" | "split" | "invert" => {
            p.heights[PROC] += 1;
            p.heights[OPST] += 1;
        }

        // U32 operations: 1 proc + 33 u32 + 1 opstack
        "lt" | "and" | "xor" | "pow" | "div_mod" => {
            p.heights[PROC] += 1;
            p.heights[U32] += 33;
            p.heights[OPST] += 1;
        }

        // U32 no-stack: 1 proc + 33 u32
        "log_2_floor" | "pop_count" => {
            p.heights[PROC] += 1;
            p.heights[U32] += 33;
        }

        // Hash operations: 1 proc + 6 hash + 1 opstack
        "hash" => {
            p.heights[PROC] += 1;
            p.heights[HASH] += 6;
            p.heights[OPST] += 1;
        }
        "sponge_init" => {
            p.heights[PROC] += 1;
            p.heights[HASH] += 6;
        }
        "sponge_absorb" | "sponge_squeeze" => {
            p.heights[PROC] += 1;
            p.heights[HASH] += 6;
            p.heights[OPST] += 1;
        }
        "sponge_absorb_mem" => {
            p.heights[PROC] += 1;
            p.heights[HASH] += 6;
            p.heights[OPST] += 1;
            p.heights[RAM] += 10;
        }

        // Merkle operations
        "merkle_step" => {
            p.heights[PROC] += 1;
            p.heights[HASH] += 6;
            p.heights[U32] += 33;
        }
        "merkle_step_mem" => {
            p.heights[PROC] += 1;
            p.heights[HASH] += 6;
            p.heights[U32] += 33;
            p.heights[RAM] += 5;
        }

        // I/O: 1 proc + 1 opstack
        "read_io" | "write_io" => {
            p.heights[PROC] += 1;
            p.heights[OPST] += 1;
        }

        // Witness: 1 proc + 1 opstack
        "divine" => {
            p.heights[PROC] += 1;
            p.heights[OPST] += 1;
        }

        // Memory: 2 proc + 2 opstack + 1 ram (per word)
        "read_mem" | "write_mem" => {
            let width = parts
                .get(1)
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(1);
            p.heights[PROC] += 2;
            p.heights[OPST] += 2;
            p.heights[RAM] += width;
        }

        // Control flow: call/return affect jump stack
        "call" => {
            p.heights[PROC] += 1;
            p.heights[JUMP] += 1;
        }
        "return" => {
            p.heights[PROC] += 1;
            p.heights[JUMP] += 1;
        }
        "recurse" | "recurse_or_return" => {
            p.heights[PROC] += 1;
            p.heights[JUMP] += 1;
        }

        // Assertions: 1 proc + 1 opstack
        "assert" | "assert_vector" => {
            p.heights[PROC] += 1;
            p.heights[OPST] += 1;
        }

        // Skiz: 1 proc + 1 opstack
        "skiz" => {
            p.heights[PROC] += 1;
            p.heights[OPST] += 1;
        }

        // Halt: 1 proc
        "halt" => {
            p.heights[PROC] += 1;
        }

        // Nop: 1 proc
        "nop" => {
            p.heights[PROC] += 1;
        }

        // Extension field: 1 proc
        "xb_mul" | "x_invert" | "xx_dot_step" | "xb_dot_step" => {
            p.heights[PROC] += 1;
            p.heights[OPST] += 1;
        }

        // Unknown instruction: count as 1 proc row (conservative)
        _ => {
            p.heights[PROC] += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_program() {
        let p = profile_tasm(&[]);
        assert_eq!(p.max_height(), 0);
        assert_eq!(p.padded_height(), 1);
        assert_eq!(p.cost(), 1);
    }

    #[test]
    fn simple_add() {
        let p = profile_tasm(&["push 1", "push 2", "add"]);
        assert_eq!(p.heights[PROC], 3);
        assert_eq!(p.heights[OPST], 3);
        assert_eq!(p.heights[HASH], 0);
    }

    #[test]
    fn hash_dominance() {
        let p = profile_tasm(&["hash", "hash", "hash"]);
        assert_eq!(p.heights[PROC], 3);
        assert_eq!(p.heights[HASH], 18);
        assert_eq!(p.dominant_table(), HASH);
    }

    #[test]
    fn cliff_boundary() {
        // 1024 proc rows pads to 1024
        let mut p = TableProfile::default();
        p.heights[PROC] = 1024;
        assert_eq!(p.padded_height(), 1024);

        // 1025 proc rows pads to 2048
        p.heights[PROC] = 1025;
        assert_eq!(p.padded_height(), 2048);
    }

    #[test]
    fn cliff_jump_detection() {
        let mut baseline = TableProfile::default();
        baseline.heights[PROC] = 1025;

        let mut candidate = TableProfile::default();
        candidate.heights[PROC] = 1024;

        assert!(candidate.is_cliff_jump(&baseline));
        assert!(!baseline.is_cliff_jump(&candidate));
    }

    #[test]
    fn table_rebalance_detection() {
        let mut baseline = TableProfile::default();
        baseline.heights[PROC] = 1000;
        baseline.heights[HASH] = 500;

        let mut candidate = TableProfile::default();
        candidate.heights[PROC] = 700;
        candidate.heights[HASH] = 700;

        assert!(candidate.is_table_rebalance(&baseline));
    }

    #[test]
    fn labels_and_comments_ignored() {
        let p = profile_tasm(&["__main:", "  // comment", "  push 1", ""]);
        assert_eq!(p.heights[PROC], 1);
    }

    #[test]
    fn memory_width() {
        let p = profile_tasm(&["read_mem 5"]);
        assert_eq!(p.heights[RAM], 5);
        assert_eq!(p.heights[PROC], 2);
    }

    #[test]
    fn u32_operations() {
        let p = profile_tasm(&["lt", "and"]);
        assert_eq!(p.heights[U32], 66); // 33 + 33
        assert_eq!(p.heights[PROC], 2);
    }

    #[test]
    fn call_return_jump_stack() {
        let p = profile_tasm(&["call __foo", "return"]);
        assert_eq!(p.heights[JUMP], 2);
    }

    #[test]
    fn profile_from_string() {
        let tasm = "push 1\npush 2\nadd\nwrite_io 1\nhalt\n";
        let p = profile_tasm_str(tasm);
        assert_eq!(p.heights[PROC], 5);
    }
}
