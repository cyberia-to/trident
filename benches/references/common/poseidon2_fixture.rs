// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Frozen Poseidon2-Goldilocks reference fixture.
//!
//! This is the ground truth for the Trident-language implementations in
//! `std/crypto/poseidon2.tri`, `std/crypto/merkle.tri`, and
//! `std/trinity/*` — the instance those `.tri` programs implement
//! (t = 8, rate = 4, RF = 8, RP = 22, S-box x^7, BLAKE3-derived round
//! constants tagged `Poseidon2-Goldilocks-t8-RF8-RP22`).
//!
//! It was moved here verbatim from `src/field/{goldilocks,poseidon2}.rs`
//! when the compiler shed its parallel field/hash implementations (M5:
//! the compiler's algebra is strata-nebu, its hash is cyber-hemera). The
//! *language-level* Poseidon2 library and its Rust ground truth are a
//! matched pair and stay bit-identical — this fixture must never drift
//! from the `.tri` implementations it validates.

/// Goldilocks prime: p = 2^64 - 2^32 + 1.
pub const MODULUS: u64 = 0xFFFF_FFFF_0000_0001;

/// A Goldilocks field element (u64 in [0, p)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Goldilocks(pub u64);

impl Goldilocks {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(1);

    #[inline]
    pub fn from_u64(v: u64) -> Self {
        Self(v % MODULUS)
    }

    #[inline]
    pub fn to_u64(self) -> u64 {
        self.0
    }

    /// Reduce a u128 value modulo p using 2^64 ≡ 2^32 - 1 (mod p).
    #[inline]
    fn reduce128(x: u128) -> Self {
        let lo = x as u64;
        let hi = (x >> 64) as u64;
        let hi_shifted = (hi as u128) * ((1u128 << 32) - 1);
        let sum = lo as u128 + hi_shifted;
        let lo2 = sum as u64;
        let hi2 = (sum >> 64) as u64;
        if hi2 == 0 {
            Self(if lo2 >= MODULUS { lo2 - MODULUS } else { lo2 })
        } else {
            let r = lo2 as u128 + (hi2 as u128) * ((1u128 << 32) - 1);
            let lo3 = r as u64;
            let hi3 = (r >> 64) as u64;
            if hi3 == 0 {
                Self(if lo3 >= MODULUS { lo3 - MODULUS } else { lo3 })
            } else {
                let v = lo3.wrapping_add(hi3.wrapping_mul(u32::MAX as u64));
                Self(if v >= MODULUS { v - MODULUS } else { v })
            }
        }
    }

    #[inline]
    pub fn add(self, rhs: Self) -> Self {
        let (sum, carry) = self.0.overflowing_add(rhs.0);
        if carry {
            let r = sum + (u32::MAX as u64);
            Self(if r >= MODULUS { r - MODULUS } else { r })
        } else {
            Self(if sum >= MODULUS { sum - MODULUS } else { sum })
        }
    }

    #[inline]
    pub fn sub(self, rhs: Self) -> Self {
        if self.0 >= rhs.0 {
            Self(self.0 - rhs.0)
        } else {
            Self(MODULUS - rhs.0 + self.0)
        }
    }

    #[inline]
    pub fn mul(self, rhs: Self) -> Self {
        Self::reduce128((self.0 as u128) * (rhs.0 as u128))
    }

    #[inline]
    pub fn neg(self) -> Self {
        if self.0 == 0 {
            Self(0)
        } else {
            Self(MODULUS - self.0)
        }
    }

    /// Multiplicative inverse via Fermat: a^(p-2). None for zero.
    pub fn inv(self) -> Option<Self> {
        if self == Self::ZERO {
            return None;
        }
        let mut exp = (MODULUS as u128) - 2;
        let mut base = self;
        let mut acc = Self::ONE;
        while exp > 0 {
            if exp & 1 == 1 {
                acc = acc.mul(base);
            }
            base = base.mul(base);
            exp >>= 1;
        }
        Some(acc)
    }

    /// Exponentiation: a^exp mod p.
    pub fn pow(self, mut exp: u64) -> Self {
        let mut base = self;
        let mut acc = Self::ONE;
        while exp > 0 {
            if exp & 1 == 1 {
                acc = acc.mul(base);
            }
            base = base.mul(base);
            exp >>= 1;
        }
        acc
    }
}

// ─── Poseidon2 instance: t=8, rate=4, RF=8, RP=22 ──────────────────

const WIDTH: usize = 8;
const RATE: usize = 4;
const ROUNDS_F: usize = 8;
const ROUNDS_P: usize = 22;
const TAG_PREFIX: &str = "Poseidon2-Goldilocks-t8-RF8-RP22";

/// Internal diagonal constants: d_i = 1 + 2^i.
const DIAG: [u64; WIDTH] = [2, 3, 5, 9, 17, 33, 65, 129];

/// Round constants derived deterministically from BLAKE3 (verbatim from the
/// deleted `src/field/poseidon2.rs` generator).
fn round_constants() -> &'static Vec<Goldilocks> {
    static CONSTANTS: std::sync::OnceLock<Vec<Goldilocks>> = std::sync::OnceLock::new();
    CONSTANTS.get_or_init(|| {
        let total_rounds = ROUNDS_F + ROUNDS_P;
        let mut constants = Vec::new();
        for r in 0..total_rounds {
            let is_full = r < ROUNDS_F / 2 || r >= ROUNDS_F / 2 + ROUNDS_P;
            if is_full {
                for e in 0..WIDTH {
                    let tag = format!("{}-{}-{}", TAG_PREFIX, r, e);
                    let digest = blake3::hash(tag.as_bytes());
                    let bytes: [u8; 8] = digest.as_bytes()[..8].try_into().unwrap_or([0u8; 8]);
                    constants.push(Goldilocks::from_u64(u64::from_le_bytes(bytes)));
                }
            } else {
                let tag = format!("{}-{}-0", TAG_PREFIX, r);
                let digest = blake3::hash(tag.as_bytes());
                let bytes: [u8; 8] = digest.as_bytes()[..8].try_into().unwrap_or([0u8; 8]);
                constants.push(Goldilocks::from_u64(u64::from_le_bytes(bytes)));
            }
        }
        constants
    })
}

#[inline]
fn sbox(x: Goldilocks) -> Goldilocks {
    let x2 = x.mul(x);
    let x3 = x2.mul(x);
    let x6 = x3.mul(x3);
    x6.mul(x)
}

/// External linear layer: circ(2,1,...,1). new[i] = state[i] + sum(state).
fn external_linear(state: &mut [Goldilocks]) {
    let sum = state.iter().fold(Goldilocks::ZERO, |a, &b| a.add(b));
    for s in state.iter_mut() {
        *s = s.add(sum);
    }
}

/// Internal linear layer: diag(d) + ones. new[i] = d_i * state[i] + sum(state).
fn internal_linear(state: &mut [Goldilocks]) {
    let sum = state.iter().fold(Goldilocks::ZERO, |a, &b| a.add(b));
    for (i, s) in state.iter_mut().enumerate() {
        *s = Goldilocks::from_u64(DIAG[i]).mul(*s).add(sum);
    }
}

/// Full Poseidon2 permutation (in place).
pub fn permutation(state: &mut [Goldilocks]) {
    let constants = round_constants();
    let mut ci = 0;

    for _ in 0..ROUNDS_F / 2 {
        for s in state[..WIDTH].iter_mut() {
            *s = s.add(constants[ci]);
            ci += 1;
        }
        for s in state[..WIDTH].iter_mut() {
            *s = sbox(*s);
        }
        external_linear(&mut state[..WIDTH]);
    }

    for _ in 0..ROUNDS_P {
        state[0] = state[0].add(constants[ci]);
        ci += 1;
        state[0] = sbox(state[0]);
        internal_linear(&mut state[..WIDTH]);
    }

    for _ in 0..ROUNDS_F / 2 {
        for s in state[..WIDTH].iter_mut() {
            *s = s.add(constants[ci]);
            ci += 1;
        }
        for s in state[..WIDTH].iter_mut() {
            *s = sbox(*s);
        }
        external_linear(&mut state[..WIDTH]);
    }
}

/// Sponge: absorb elements at the rate, permute, squeeze 4 elements.
pub fn hash_fields_goldilocks(elements: &[Goldilocks]) -> [Goldilocks; 4] {
    let mut state = vec![Goldilocks::ZERO; WIDTH];
    let mut absorbed = 0;

    for &elem in elements {
        if absorbed == RATE {
            permutation(&mut state);
            absorbed = 0;
        }
        state[absorbed] = state[absorbed].add(elem);
        absorbed += 1;
    }

    permutation(&mut state);
    [state[0], state[1], state[2], state[3]]
}
