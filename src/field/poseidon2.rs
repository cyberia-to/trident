// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Generic Poseidon2 hash function over any PrimeField.
//!
//! Implements the Poseidon2 permutation (Grassi et al., 2023) with
//! configurable state width, round counts, and S-box. The sponge
//! construction (absorb/squeeze) is field-generic.
//!
//! Warriors call `poseidon2_hash::<Goldilocks>(...)` or
//! `poseidon2_hash::<BabyBear>(...)` — same code, different field.

use super::PrimeField;

// ─── Poseidon2 Parameters ──────────────────────────────────────────

/// Poseidon2 configuration for a specific field instantiation.
pub struct Poseidon2Config<F: PrimeField> {
    /// State width (typically 8 or 12).
    pub width: usize,
    /// Rate (number of input elements absorbed per permutation).
    pub rate: usize,
    /// Number of full rounds (split evenly: half before, half after partial).
    pub rounds_f: usize,
    /// Number of partial rounds.
    pub rounds_p: usize,
    /// Internal diagonal constants for the internal linear layer.
    pub diag: Vec<F>,
    /// Round constants (R_F * width + R_P elements).
    pub round_constants: Vec<F>,
}

/// Default Poseidon2 config for Goldilocks (t=8, rate=4, RF=8, RP=22).
pub fn goldilocks_config() -> Poseidon2Config<super::Goldilocks> {
    use super::Goldilocks;

    let width = 8;
    let rate = 4;
    let rounds_f = 8;
    let rounds_p = 22;

    let diag: Vec<Goldilocks> = [2u64, 3, 5, 9, 17, 33, 65, 129]
        .iter()
        .map(|&v| Goldilocks(v))
        .collect();

    let round_constants = generate_round_constants::<Goldilocks>(
        width,
        rounds_f,
        rounds_p,
        "Poseidon2-Goldilocks-t8-RF8-RP22",
    );

    Poseidon2Config {
        width,
        rate,
        rounds_f,
        rounds_p,
        diag,
        round_constants,
    }
}

/// Generate round constants deterministically from BLAKE3.
fn generate_round_constants<F: PrimeField>(
    width: usize,
    rounds_f: usize,
    rounds_p: usize,
    tag_prefix: &str,
) -> Vec<F> {
    let total_rounds = rounds_f + rounds_p;
    let mut constants = Vec::new();
    for r in 0..total_rounds {
        let is_full = r < rounds_f / 2 || r >= rounds_f / 2 + rounds_p;
        if is_full {
            for e in 0..width {
                let tag = format!("{}-{}-{}", tag_prefix, r, e);
                let digest = blake3::hash(tag.as_bytes());
                let bytes: [u8; 8] = digest.as_bytes()[..8].try_into().unwrap_or([0u8; 8]);
                constants.push(F::from_u64(u64::from_le_bytes(bytes)));
            }
        } else {
            let tag = format!("{}-{}-0", tag_prefix, r);
            let digest = blake3::hash(tag.as_bytes());
            let bytes: [u8; 8] = digest.as_bytes()[..8].try_into().unwrap_or([0u8; 8]);
            constants.push(F::from_u64(u64::from_le_bytes(bytes)));
        }
    }
    constants
}

// ─── Cached Goldilocks Config ──────────────────────────────────────

fn cached_goldilocks_config() -> &'static Poseidon2Config<super::Goldilocks> {
    static CONFIG: std::sync::OnceLock<Poseidon2Config<super::Goldilocks>> =
        std::sync::OnceLock::new();
    CONFIG.get_or_init(goldilocks_config)
}

// ─── Permutation ───────────────────────────────────────────────────

/// Apply the Poseidon2 S-box (x^7) to a single field element.
#[inline]
fn sbox<F: PrimeField>(x: F) -> F {
    let x2 = x.mul(x);
    let x3 = x2.mul(x);
    let x6 = x3.mul(x3);
    x6.mul(x)
}

/// External linear layer: circ(2,1,...,1).
/// new[i] = state[i] + sum(state).
fn external_linear<F: PrimeField>(state: &mut [F]) {
    let sum = state.iter().fold(F::ZERO, |a, &b| a.add(b));
    for s in state.iter_mut() {
        *s = s.add(sum);
    }
}

/// Internal linear layer: diag(d_0,...,d_{w-1}) + ones_matrix.
/// new[i] = d_i * state[i] + sum(state).
fn internal_linear<F: PrimeField>(state: &mut [F], diag: &[F]) {
    let sum = state.iter().fold(F::ZERO, |a, &b| a.add(b));
    for (i, s) in state.iter_mut().enumerate() {
        *s = diag[i].mul(*s).add(sum);
    }
}

/// Full Poseidon2 permutation (in-place, generic over field and width).
pub fn permutation<F: PrimeField>(state: &mut [F], config: &Poseidon2Config<F>) {
    let mut ci = 0;
    let width = config.width;

    // First R_F/2 full rounds
    for _ in 0..config.rounds_f / 2 {
        for s in state[..width].iter_mut() {
            *s = s.add(config.round_constants[ci]);
            ci += 1;
        }
        for s in state[..width].iter_mut() {
            *s = sbox(*s);
        }
        external_linear(&mut state[..width]);
    }

    // R_P partial rounds
    for _ in 0..config.rounds_p {
        state[0] = state[0].add(config.round_constants[ci]);
        ci += 1;
        state[0] = sbox(state[0]);
        internal_linear(&mut state[..width], &config.diag);
    }

    // Last R_F/2 full rounds
    for _ in 0..config.rounds_f / 2 {
        for s in state[..width].iter_mut() {
            *s = s.add(config.round_constants[ci]);
            ci += 1;
        }
        for s in state[..width].iter_mut() {
            *s = sbox(*s);
        }
        external_linear(&mut state[..width]);
    }
}

// ─── Sponge Hasher ─────────────────────────────────────────────────

/// Absorb field elements, permute, squeeze — generic over PrimeField.
fn sponge_hash<F: PrimeField>(
    elements: &[F],
    config: &Poseidon2Config<F>,
    squeeze_count: usize,
) -> Vec<F> {
    let mut state = vec![F::ZERO; config.width];
    let mut absorbed = 0;

    for &elem in elements {
        if absorbed == config.rate {
            permutation(&mut state, config);
            absorbed = 0;
        }
        state[absorbed] = state[absorbed].add(elem);
        absorbed += 1;
    }

    // Squeeze
    permutation(&mut state, config);
    let mut out = Vec::with_capacity(squeeze_count);
    let mut squeezed = 0;
    loop {
        for &elem in state[..config.rate].iter() {
            out.push(elem);
            squeezed += 1;
            if squeezed == squeeze_count {
                return out;
            }
        }
        permutation(&mut state, config);
    }
}

// ─── Goldilocks Convenience Functions ──────────────────────────────

/// Hash arbitrary bytes using Poseidon2 over Goldilocks, returning 32 bytes.
///
/// This is the drop-in replacement for `crate::package::poseidon2::hash_bytes`.
pub fn hash_bytes_goldilocks(data: &[u8]) -> [u8; 32] {
    use super::Goldilocks;

    const BYTES_PER_ELEM: usize = 7;
    let mut elements = Vec::with_capacity(data.len() / BYTES_PER_ELEM + 2);
    for chunk in data.chunks(BYTES_PER_ELEM) {
        let mut buf = [0u8; 8];
        buf[..chunk.len()].copy_from_slice(chunk);
        elements.push(Goldilocks::from_u64(u64::from_le_bytes(buf)));
    }
    // Length separator
    elements.push(Goldilocks::from_u64(data.len() as u64));

    let config = cached_goldilocks_config();
    let result = sponge_hash(&elements, config, 4);

    let mut out = [0u8; 32];
    for (i, elem) in result.iter().enumerate() {
        out[i * 8..i * 8 + 8].copy_from_slice(&elem.to_u64().to_le_bytes());
    }
    out
}

/// Hash Goldilocks field elements, returning 4 elements.
pub fn hash_fields_goldilocks(elements: &[super::Goldilocks]) -> [super::Goldilocks; 4] {
    let config = cached_goldilocks_config();
    let result = sponge_hash(elements, config, 4);
    [result[0], result[1], result[2], result[3]]
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::Goldilocks;

    #[test]
    fn goldilocks_hash_deterministic() {
        assert_eq!(
            hash_bytes_goldilocks(b"hello world"),
            hash_bytes_goldilocks(b"hello world"),
        );
    }

    #[test]
    fn goldilocks_hash_different_inputs() {
        assert_ne!(
            hash_bytes_goldilocks(b"hello"),
            hash_bytes_goldilocks(b"world"),
        );
    }

    #[test]
    fn goldilocks_hash_fields_deterministic() {
        let elems: Vec<Goldilocks> = (1..=5).map(|v| Goldilocks::from_u64(v)).collect();
        assert_eq!(
            hash_fields_goldilocks(&elems),
            hash_fields_goldilocks(&elems)
        );
    }

    #[test]
    fn goldilocks_collision_resistance() {
        let hashes: Vec<[u8; 32]> = (0u64..20)
            .map(|i| hash_bytes_goldilocks(&i.to_le_bytes()))
            .collect();
        for i in 0..hashes.len() {
            for j in i + 1..hashes.len() {
                assert_ne!(hashes[i], hashes[j], "collision between {} and {}", i, j);
            }
        }
    }

    #[test]
    fn permutation_diffusion() {
        let config = cached_goldilocks_config();
        let base: Vec<Goldilocks> = (0..8).map(|i| Goldilocks::from_u64(i + 100)).collect();
        let mut s1 = base.clone();
        permutation(&mut s1, config);

        let mut tweaked = base;
        tweaked[0] = tweaked[0].add(Goldilocks::ONE);
        let mut s2 = tweaked;
        permutation(&mut s2, config);

        for i in 0..8 {
            assert_ne!(s1[i], s2[i], "element {} unchanged after input tweak", i);
        }
    }
}
