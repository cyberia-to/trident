// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Mersenne31 prime field: p = 2^31 - 1 = 0x7FFF_FFFF.
//!
//! Used by: Plonky3, Circle STARKs.
//!
//! A Mersenne prime with extremely efficient reduction: multiplication
//! reduces via shift-and-add since 2^31 ≡ 1 (mod p).

use super::PrimeField;

/// Mersenne31 prime: p = 2^31 - 1 = 2147483647.
pub const MODULUS: u64 = 0x7FFF_FFFF;

/// A Mersenne31 field element (u64 in [0, p)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Mersenne31(pub u64);

impl Mersenne31 {
    /// Reduce a u64 value modulo p using the Mersenne identity:
    /// 2^31 ≡ 1 (mod p), so (hi << 31 | lo) ≡ hi + lo (mod p).
    #[inline]
    fn reduce(v: u64) -> Self {
        let lo = v & MODULUS;
        let hi = v >> 31;
        let sum = lo + hi;
        Self(if sum >= MODULUS { sum - MODULUS } else { sum })
    }
}

impl PrimeField for Mersenne31 {
    const MODULUS: u128 = MODULUS as u128;
    const BITS: u32 = 31;
    const ZERO: Self = Self(0);
    const ONE: Self = Self(1);

    #[inline]
    fn from_u64(v: u64) -> Self {
        Self::reduce(v % MODULUS)
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
        let product = self.0 as u128 * rhs.0 as u128;
        // Reduce using 2^31 ≡ 1: split into 31-bit chunks
        let lo = (product & MODULUS as u128) as u64;
        let hi = (product >> 31) as u64;
        Self::reduce(lo + hi)
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
