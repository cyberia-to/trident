// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Universal STARK proof estimation and claim structure.
//!
//! These formulas apply to all FRI-based STARK provers regardless of
//! target VM or field. Warriors call these functions for cost reporting
//! and proof parameter computation.

// ─── Claim ─────────────────────────────────────────────────────────

/// Universal proof claim: what any STARK/SNARK proof asserts.
///
/// This is the public data shared between prover and verifier.
/// The proof itself is warrior-specific (opaque bytes); the claim
/// is universal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Claim {
    /// Hash of the compiled program (VM-native digest).
    pub program_hash: Vec<u64>,
    /// Public input field elements.
    pub public_input: Vec<u64>,
    /// Public output field elements.
    pub public_output: Vec<u64>,
}

// ─── Trace Geometry ────────────────────────────────────────────────

/// Padded trace height: next power of two above the tallest table.
///
/// Universal to all STARKs — the execution trace is padded to a power
/// of two for NTT (Number Theoretic Transform) efficiency.
pub fn padded_height(max_table_rows: u64) -> u64 {
    if max_table_rows == 0 {
        return 1;
    }
    max_table_rows.next_power_of_two()
}

/// Merkle tree depth for a trace of the given padded height.
pub fn merkle_depth(padded_height: u64) -> u32 {
    if padded_height <= 1 {
        return 0;
    }
    64 - (padded_height - 1).leading_zeros()
}

/// NTT (Number Theoretic Transform) domain size.
///
/// The evaluation domain is `padded_height * blowup_factor`. Larger
/// blowup factors give stronger soundness per FRI query but increase
/// prover work linearly.
pub fn ntt_domain_size(padded_height: u64, blowup_factor: u64) -> u64 {
    padded_height.saturating_mul(blowup_factor)
}

// ─── FRI Parameters ────────────────────────────────────────────────

/// Estimate the number of FRI queries needed for a target security level.
///
/// Each FRI query provides approximately `log2(blowup_factor)` bits of
/// security against a dishonest prover. The total number of queries is
/// `ceil(security_bits / log2(blowup_factor))`.
///
/// Typical parameters: security_bits=128, blowup_factor=4 → 64 queries.
pub fn fri_query_count(security_bits: u32, blowup_factor: u64) -> u32 {
    if blowup_factor <= 1 {
        return security_bits;
    }
    // Integer log2 of blowup_factor
    let log2_blowup = 63 - blowup_factor.leading_zeros();
    if log2_blowup == 0 {
        return security_bits;
    }
    (security_bits + log2_blowup - 1) / log2_blowup
}

// ─── Proof Size Estimation ─────────────────────────────────────────

/// Estimate proof size in bytes.
///
/// A STARK proof contains:
/// - FRI merkle authentication paths (one per query per column)
/// - Polynomial evaluations at query points
/// - FRI folding commitments
///
/// This gives a rough lower bound. Actual proofs include additional
/// metadata, boundary constraints, and zero-knowledge blinding.
pub fn estimate_proof_size(
    padded_height: u64,
    column_count: u64,
    field_bytes: u64,
    blowup_factor: u64,
    security_bits: u32,
) -> u64 {
    let queries = fri_query_count(security_bits, blowup_factor) as u64;
    let depth = merkle_depth(padded_height) as u64;
    // Each query: column_count evaluations + Merkle path (depth * hash_size)
    let hash_size: u64 = 32; // typical: 256-bit hash nodes
    let per_query = column_count * field_bytes + depth * hash_size;
    queries * per_query
}

/// Estimate proving time in nanoseconds.
///
/// Rough universal estimate based on NTT-dominated proving:
/// `padded_height * column_count * log2(padded_height) * ns_per_field_op`.
///
/// The constant 3 ns/op is a conservative estimate for modern CPUs
/// performing 64-bit field multiplication.
pub fn estimate_proving_ns(padded_height: u64, column_count: u64) -> u64 {
    if padded_height == 0 || column_count == 0 {
        return 0;
    }
    let log_h = 64 - padded_height.leading_zeros() as u64;
    padded_height
        .saturating_mul(column_count)
        .saturating_mul(log_h)
        .saturating_mul(3)
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padded_height_powers_of_two() {
        assert_eq!(padded_height(0), 1);
        assert_eq!(padded_height(1), 1);
        assert_eq!(padded_height(2), 2);
        assert_eq!(padded_height(3), 4);
        assert_eq!(padded_height(5), 8);
        assert_eq!(padded_height(1024), 1024);
        assert_eq!(padded_height(1025), 2048);
    }

    #[test]
    fn merkle_depth_correct() {
        assert_eq!(merkle_depth(1), 0);
        assert_eq!(merkle_depth(2), 1);
        assert_eq!(merkle_depth(4), 2);
        assert_eq!(merkle_depth(1024), 10);
        assert_eq!(merkle_depth(1 << 20), 20);
    }

    #[test]
    fn fri_queries_typical() {
        // blowup=4 → log2=2 → 128/2 = 64 queries
        assert_eq!(fri_query_count(128, 4), 64);
        // blowup=8 → log2=3 → ceil(128/3) = 43 queries
        assert_eq!(fri_query_count(128, 8), 43);
        // blowup=2 → log2=1 → 128 queries
        assert_eq!(fri_query_count(128, 2), 128);
        // blowup=1 → degenerate
        assert_eq!(fri_query_count(128, 1), 128);
    }

    #[test]
    fn proof_size_reasonable() {
        // Typical Triton: 2^20 padded, 200 columns, 8-byte field, blowup=4, 128-bit security
        let size = estimate_proof_size(1 << 20, 200, 8, 4, 128);
        // Should be in the range of hundreds of KB to low MB
        assert!(size > 100_000, "proof too small: {}", size);
        assert!(size < 100_000_000, "proof too large: {}", size);
    }

    #[test]
    fn proving_time_nonzero() {
        let ns = estimate_proving_ns(1 << 20, 200);
        assert!(ns > 0);
        // Should be in the range of seconds (billions of ns)
        assert!(ns > 1_000_000_000, "estimate too fast: {} ns", ns);
    }

    #[test]
    fn proving_time_zero_inputs() {
        assert_eq!(estimate_proving_ns(0, 100), 0);
        assert_eq!(estimate_proving_ns(100, 0), 0);
    }
}
