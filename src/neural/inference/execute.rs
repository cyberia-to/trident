// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Parallel validation and ranking of beam search candidates.
//!
//! Takes K candidate token sequences from beam search, decodes them
//! to target assembly, validates equivalence with baseline using rayon parallel
//! iteration, and returns the cheapest valid candidate.

use rayon::prelude::*;

use crate::neural::Target;

/// Result of validating and ranking beam candidates.
pub struct RankedResult {
    /// Best valid target assembly sequence (if any).
    pub assembly: Vec<String>,
    /// Clock cycles (table cost) of the best candidate.
    pub cost: u64,
    /// How many candidates were valid out of total.
    pub valid_count: usize,
    /// Total candidates evaluated.
    pub total_count: usize,
}

/// Validate beam search candidates against a baseline and return the best.
///
/// Each candidate is decoded from token IDs to target assembly strings, then verified
/// for equivalence with the baseline target assembly using the stack verifier.
/// Valid candidates are profiled for cost, and the cheapest is returned.
///
/// Uses rayon for parallel validation across all K candidates.
///
/// Returns `None` if no valid candidate is found (fallback to compiler).
pub fn validate_and_rank(
    candidates: &[Vec<u32>],
    target: &dyn Target,
    baseline: &[String],
    seed: u64,
) -> Option<RankedResult> {
    if candidates.is_empty() || baseline.is_empty() {
        return None;
    }

    let results: Vec<Option<(Vec<String>, u64)>> = candidates
        .par_iter()
        .map(|token_ids| {
            // Decode tokens to target assembly lines
            let assembly = target.vocabulary().decode_sequence(token_ids);
            if assembly.is_empty() {
                return None;
            }

            // Verify equivalence with baseline
            if !target.verify_block(baseline, &assembly, seed) {
                return None;
            }

            // Profile for cost
            let cost = target.cost(&assembly);

            Some((assembly, cost))
        })
        .collect();

    let valid_count = results.iter().filter(|r| r.is_some()).count();
    let total_count = candidates.len();

    // Find cheapest valid candidate
    let best = results
        .into_iter()
        .flatten()
        .min_by_key(|(_, cost)| *cost)?;

    Some(RankedResult {
        assembly: best.0,
        cost: best.1,
        valid_count,
        total_count,
    })
}
