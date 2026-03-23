// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! BabyBear prime field: p = 2^31 - 2^27 + 1 = 0x7800_0001.
//!
//! Used by: SP1, RISC Zero, Jolt.
//!
//! A 31-bit prime with efficient NTT (multiplicative group order 2^27).

use super::PrimeField;

/// BabyBear prime: p = 2^31 - 2^27 + 1 = 2013265921.
pub const MODULUS: u64 = 0x7800_0001;

/// A BabyBear field element (u64 in [0, p), stored in lower 31 bits).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BabyBear(pub u64);

impl PrimeField for BabyBear {
    const MODULUS: u128 = MODULUS as u128;
    const BITS: u32 = 31;
    const ZERO: Self = Self(0);
    const ONE: Self = Self(1);

    #[inline]
    fn from_u64(v: u64) -> Self {
        Self(v % MODULUS)
    }

    #[inline]
    fn to_u64(self) -> u64 {
        self.0
    }

    #[inline]
    fn add(self, rhs: Self) -> Self {
        let sum = self.0 + rhs.0;
        Self(if sum >= MODULUS { sum - MODULUS } else { sum })
    }

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        if self.0 >= rhs.0 {
            Self(self.0 - rhs.0)
        } else {
            Self(MODULUS - rhs.0 + self.0)
        }
    }

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self(((self.0 as u128 * rhs.0 as u128) % MODULUS as u128) as u64)
    }

    #[inline]
    fn neg(self) -> Self {
        if self.0 == 0 {
            Self(0)
        } else {
            Self(MODULUS - self.0)
        }
    }
}
