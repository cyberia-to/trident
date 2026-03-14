//! Equivalence verification: compare baseline and candidate TASM
//! on diverse test stacks.

use crate::field::goldilocks::MODULUS;

use super::executor::StackState;

/// Generate a deterministic test stack for a given seed.
pub fn generate_test_stack(seed: u64, size: usize) -> Vec<u64> {
    let mut stack = Vec::with_capacity(size);
    let mut state = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    for _ in 0..size {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        // Keep values in valid Goldilocks range
        let val = state % MODULUS;
        stack.push(val);
    }
    stack
}

/// Verify that candidate TASM produces the same stack as baseline TASM.
/// Tests with 40 stacks (32 random + 8 structured) — all must pass.
/// A single concrete test case is trivially gamed by the neural optimizer;
/// diverse stacks catch wrong operand order, off-by-one dup/swap depth,
/// missing operations, and false positives on complex blocks.
/// Conservative: rejects candidates when baseline can't be simulated.
pub fn verify_equivalent(baseline_tasm: &[String], candidate_tasm: &[String], seed: u64) -> bool {
    // Instructions the verifier can simulate with exact semantics.
    // Side-effect ops (write_io, assert, divine, halt, split) are now
    // supported via side-channel logging — the verifier compares I/O
    // output, assertion values, divine call patterns, and halt state
    // alongside the stack.
    const ALLOWED: &[&str] = &[
        "push",
        "pop",
        "dup",
        "swap",
        "pick",
        "place",
        "add",
        "mul",
        // invert is NOT allowed — verifier implements negation,
        // Triton VM does multiplicative inverse (1/x mod p).
        "eq",
        "lt",
        "and",
        "xor",
        "split",
        "div_mod",
        "pow",
        "log_2_floor",
        "pop_count",
        "nop",
        "halt",
        "write_io",
        "read_io",
        "divine",
        "assert",
        "assert_vector",
        // Memory ops — simulated with dummy values (correct stack effects)
        "read_mem",
        "write_mem",
        // Crypto ops — simulated with dummy values (correct stack effects)
        "hash",
        "sponge_init",
        "sponge_absorb",
        "sponge_squeeze",
        "sponge_absorb_mem",
        "merkle_step",
        "merkle_step_mem",
        // Extension field ops — simulated as nops (correct stack effects)
        "xb_mul",
        "x_invert",
        "xx_dot_step",
        "xb_dot_step",
        // Control flow — return/recurse are no-ops for stack in isolated functions
        "return",
        "recurse",
    ];
    for line in candidate_tasm {
        let op = line.trim().split_whitespace().next().unwrap_or("");
        if op.is_empty() || op.starts_with("//") || op.ends_with(':') {
            continue;
        }
        if !ALLOWED.contains(&op) {
            return false;
        }
    }

    // Test stacks must include structured values, not just random ones.
    // Random Goldilocks values make eq/lt comparisons near-deterministic:
    //   P(random a == random b) ≈ 2^-64 → "push 0" fakes "dup 1 | dup 1 | eq"
    // Structured stacks expose these exploits by including zeros, duplicates,
    // and ordered values where comparisons actually produce different results.
    let test_stacks: Vec<Vec<u64>> = {
        let mut stacks = Vec::with_capacity(40);
        // 32 random seeds — 4 was too few; complex blocks (keccak rounds,
        // field arithmetic) can produce false positives when random stacks
        // happen to collide. 32 brings P(false positive) to ~2^(-64*32).
        for i in 0..32u64 {
            let s = seed.wrapping_mul(6364136223846793005).wrapping_add(i);
            stacks.push(generate_test_stack(s, 16));
        }
        // All zeros (eq always returns 1, catches "push 0" faking eq)
        stacks.push(vec![0; 16]);
        // All ones
        stacks.push(vec![1; 16]);
        // Adjacent pairs equal: [5,5,3,3,7,7,...] (catches dup+eq exploits)
        stacks.push(vec![5, 5, 3, 3, 7, 7, 2, 2, 9, 9, 1, 1, 4, 4, 8, 8]);
        // Same value everywhere (catches any eq/dup combination)
        stacks.push(vec![42; 16]);
        // Ascending small values (catches lt/comparison order exploits)
        stacks.push((0..16).collect());
        // Descending (opposite lt behavior)
        stacks.push((0..16).rev().collect());
        // Mixed: zeros and large values (catches split/div_mod edge cases)
        stacks.push(vec![
            0,
            MODULUS - 1,
            0,
            1,
            0,
            MODULUS - 1,
            0,
            1,
            0,
            MODULUS - 1,
            0,
            1,
            0,
            MODULUS - 1,
            0,
            1,
        ]);
        // Powers of 2 (catches pop_count, log_2_floor, split edge cases)
        stacks.push(vec![
            1,
            2,
            4,
            8,
            16,
            32,
            64,
            128,
            256,
            512,
            1024,
            2048,
            1u64 << 32,
            1u64 << 33,
            1u64 << 48,
            1u64 << 63,
        ]);
        // Split-specific: values with non-trivial hi parts (all < MODULUS)
        stacks.push(vec![
            0x0000_0001_0000_0001, // hi=1, lo=1
            0x0000_0002_FFFF_FFFF, // hi=2, lo=max32
            0x0000_FFFF_0000_0000, // hi=65535, lo=0
            0x0000_0000_FFFF_FFFF, // hi=0, lo=max32
            0x0000_0001_0000_0000, // hi=1, lo=0
            0x0000_0000_0000_0001, // hi=0, lo=1
            0x0000_ABCD_1234_5678, // mixed
            0x0000_0000_8000_0000, // hi=0, lo=2^31
            0x0000_0001_8000_0000, // hi=1, lo=2^31
            0x0000_7FFF_7FFF_FFFF, // hi=32767, lo=max31+
            0x0000_0010_0000_0010, // hi=16, lo=16
            0x0000_0100_0001_0000, // hi=256, lo=65536
            0,
            1,
            MODULUS - 1,
            MODULUS - 2,
        ]);
        stacks
    };

    for test_stack in &test_stacks {
        let mut baseline_state = StackState::new(test_stack.clone());
        baseline_state.execute(baseline_tasm);

        // If baseline can't be simulated, we can't verify — reject candidate.
        if baseline_state.error {
            return false;
        }

        let mut candidate_state = StackState::new(test_stack.clone());
        candidate_state.execute(candidate_tasm);

        if candidate_state.error {
            return false;
        }

        if baseline_state.stack != candidate_state.stack {
            return false;
        }
        if baseline_state.halted != candidate_state.halted {
            return false;
        }
        if baseline_state.io_output != candidate_state.io_output {
            return false;
        }
        if baseline_state.divine_log != candidate_state.divine_log {
            return false;
        }
        if baseline_state.assert_log != candidate_state.assert_log {
            return false;
        }
        if baseline_state.assert_vector_log != candidate_state.assert_vector_log {
            return false;
        }
    }
    true
}

/// Diagnose why verification failed for a candidate vs baseline.
/// Runs both on the first test stack and reports what differs.
/// Returns a short human-readable reason string.
pub fn diagnose_failure(baseline_tasm: &[String], candidate_tasm: &[String], seed: u64) -> String {
    let test_stack = generate_test_stack(seed.wrapping_mul(6364136223846793005), 16);

    let mut bl = StackState::new(test_stack.clone());
    bl.execute(baseline_tasm);
    if bl.error {
        return "baseline errors on test stack".into();
    }

    let mut cd = StackState::new(test_stack);
    cd.execute(candidate_tasm);
    if cd.error {
        return "candidate errors on test stack".into();
    }

    if bl.stack != cd.stack {
        let bl_len = bl.stack.len();
        let cd_len = cd.stack.len();
        if bl_len != cd_len {
            return format!("stack depth: baseline={} candidate={}", bl_len, cd_len);
        }
        for i in 0..bl_len {
            if bl.stack[i] != cd.stack[i] {
                return format!(
                    "stack[{}]: baseline={} candidate={} (depth {})",
                    i, bl.stack[i], cd.stack[i], bl_len
                );
            }
        }
    }
    if bl.halted != cd.halted {
        return format!("halted: baseline={} candidate={}", bl.halted, cd.halted);
    }
    if bl.io_output != cd.io_output {
        return format!(
            "io_output: baseline={:?} candidate={:?}",
            &bl.io_output[..bl.io_output.len().min(5)],
            &cd.io_output[..cd.io_output.len().min(5)]
        );
    }
    if bl.divine_log != cd.divine_log {
        return format!(
            "divine_log: baseline={:?} candidate={:?}",
            bl.divine_log, cd.divine_log
        );
    }
    if bl.assert_log != cd.assert_log {
        return format!(
            "assert_log: baseline={:?} candidate={:?}",
            &bl.assert_log[..bl.assert_log.len().min(5)],
            &cd.assert_log[..cd.assert_log.len().min(5)]
        );
    }
    if bl.assert_vector_log != cd.assert_vector_log {
        return "assert_vector_log differs".into();
    }
    // Passed on this test stack but fails on others
    "passes first stack, fails on structured stacks".into()
}
