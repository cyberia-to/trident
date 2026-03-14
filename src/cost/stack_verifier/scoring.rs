//! Fitness scoring for neural optimizer candidates.

use super::equivalence::{generate_test_stack, verify_equivalent};
use super::executor::StackState;

/// Score how close a candidate is to matching the baseline.
/// Runs both on one test stack and returns a shaped fitness score:
///   0   = candidate crashes
///   100 = doesn't crash
///   200 = stack depth matches
///   400 = 50%+ of stack values match
///   600 = 90%+ of stack values match
///   800 = all stack values match
///   900 = stack + all side-channels match on this stack
pub fn score_candidate(baseline_tasm: &[String], candidate_tasm: &[String], seed: u64) -> i64 {
    let test_stack = generate_test_stack(seed.wrapping_mul(6364136223846793005), 16);

    let mut bl = StackState::new(test_stack.clone());
    bl.execute(baseline_tasm);
    if bl.error {
        return 0;
    }

    let mut cd = StackState::new(test_stack);
    cd.execute(candidate_tasm);

    if cd.error {
        return 0;
    }
    let mut score: i64 = 100;

    if bl.stack.len() == cd.stack.len() {
        score = 200;
        let matches = bl
            .stack
            .iter()
            .zip(&cd.stack)
            .filter(|(a, b)| a == b)
            .count();
        let total = bl.stack.len().max(1);
        let ratio = matches as f64 / total as f64;
        if ratio >= 0.5 {
            score = 400;
        }
        if ratio >= 0.9 {
            score = 600;
        }
        if matches == total {
            score = 800;
        }
    }

    if score >= 800
        && bl.halted == cd.halted
        && bl.io_output == cd.io_output
        && bl.divine_log == cd.divine_log
        && bl.assert_log == cd.assert_log
        && bl.assert_vector_log == cd.assert_vector_log
    {
        score = 900;
    }

    score
}

/// Score a neural model's raw output against a baseline block.
/// Decodes the output, verifies equivalence, and returns the lower cost
/// (or baseline cost if candidate is invalid/worse).
pub fn score_neural_output(
    raw_codes: &[u32],
    block_baseline: u64,
    baseline_tasm: &[String],
    block_seed: u64,
) -> u64 {
    use crate::ir::tir::lower::decode_output;

    let codes: Vec<u64> = raw_codes
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c as u64)
        .collect();
    if codes.is_empty() {
        return block_baseline;
    }
    let candidate_lines = decode_output(&codes);
    if candidate_lines.is_empty() {
        return block_baseline;
    }
    // No baseline = nothing to verify against = reject.
    if baseline_tasm.is_empty() || !verify_equivalent(baseline_tasm, &candidate_lines, block_seed) {
        return block_baseline;
    }
    let profile = crate::cost::scorer::profile_tasm(
        &candidate_lines
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>(),
    );
    profile.cost().min(block_baseline)
}

/// Score improvement of a neural candidate over baseline.
/// Returns 0 for failures or equal/worse cost, positive value for genuine wins.
/// Used by training to reward only actual improvement (not negated cost).
pub fn score_neural_improvement(
    raw_codes: &[u32],
    block_baseline: u64,
    baseline_tasm: &[String],
    block_seed: u64,
) -> u64 {
    let cost = score_neural_output(raw_codes, block_baseline, baseline_tasm, block_seed);
    block_baseline.saturating_sub(cost)
}
