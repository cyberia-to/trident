//! SH0.2 conformance model over real nox nodes, not a guest library implementation.
use nebu::Goldilocks;
use nox::{Order, Reduction};

pub const SEQ: u64 = 0x53455131;
pub const BYTES: u64 = 0x42595431;
const MODULUS: u64 = 18_446_744_069_414_584_321;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Allocation,
    Shape,
    Tag,
    Range,
    Padding,
    Index,
    Limit,
    Visits,
}
pub type Result<T> = std::result::Result<T, Error>;

pub fn atom<const N: usize>(ar: &mut Reduction<N>, value: u64) -> Result<Order> {
    if value >= MODULUS {
        return Err(Error::Range);
    }
    ar.atom(Goldilocks::new(value)).ok_or(Error::Allocation)
}

pub fn pair<const N: usize>(ar: &mut Reduction<N>, a: Order, b: Order) -> Result<Order> {
    ar.pair(a, b).ok_or(Error::Allocation)
}

pub fn value<const N: usize>(ar: &Reduction<N>, n: Order) -> Result<u64> {
    ar.atom_value(n).map(|v| v.as_u64()).ok_or(Error::Shape)
}

fn children<const N: usize>(ar: &Reduction<N>, n: Order) -> Result<(Order, Order)> {
    Ok((
        ar.head(n).ok_or(Error::Shape)?,
        ar.tail(n).ok_or(Error::Shape)?,
    ))
}

pub fn height(len: u32) -> u32 {
    if len <= 1 {
        0
    } else {
        32 - (len - 1).leading_zeros()
    }
}

fn empty_tree<const N: usize>(ar: &mut Reduction<N>, height: u32) -> Result<Order> {
    let mut root = atom(ar, 0)?;
    for _ in 0..height {
        root = pair(ar, root, root)?;
    }
    Ok(root)
}

fn build<const N: usize>(ar: &mut Reduction<N>, values: &[Order], h: u32) -> Result<Order> {
    if values.is_empty() {
        return empty_tree(ar, h);
    }
    if h == 0 {
        return Ok(values[0]);
    }
    let mid = values.len().min(1usize << (h - 1));
    let left = build(ar, &values[..mid], h - 1)?;
    let right = build(ar, &values[mid..], h - 1)?;
    pair(ar, left, right)
}

fn edit<const N: usize>(
    ar: &mut Reduction<N>,
    root: Order,
    h: u32,
    i: u32,
    v: Order,
) -> Result<Order> {
    if h == 0 {
        return Ok(v);
    }
    let (left, right) = children(ar, root)?;
    let half = 1u32 << (h - 1);
    if i < half {
        let next = edit(ar, left, h - 1, i, v)?;
        pair(ar, next, right)
    } else {
        let next = edit(ar, right, h - 1, i - half, v)?;
        pair(ar, left, next)
    }
}

fn validate_tree<const N: usize>(
    ar: &mut Reduction<N>,
    root: Order,
    len: u32,
    h: u32,
    visits: &mut u32,
) -> Result<()> {
    *visits = visits.checked_sub(1).ok_or(Error::Visits)?;
    if len == 0 {
        let empty = empty_tree(ar, h)?;
        return if ar.digest(root) == ar.digest(empty) {
            Ok(())
        } else {
            Err(Error::Padding)
        };
    }
    if h == 0 {
        return Ok(());
    }
    let (left, right) = children(ar, root)?;
    let half = 1u32 << (h - 1);
    validate_tree(ar, left, len.min(half), h - 1, visits)?;
    validate_tree(ar, right, len.saturating_sub(half), h - 1, visits)
}

fn wrap<const N: usize>(ar: &mut Reduction<N>, tag: u64, len: u32, root: Order) -> Result<Order> {
    let tag = atom(ar, tag)?;
    let len = atom(ar, u64::from(len))?;
    let body = pair(ar, len, root)?;
    pair(ar, tag, body)
}

fn unwrap<const N: usize>(
    ar: &Reduction<N>,
    n: Order,
    expected: u64,
    max: u32,
) -> Result<(u32, Order)> {
    let (tag, body) = children(ar, n)?;
    if value(ar, tag)? != expected {
        return Err(Error::Tag);
    }
    let (len, root) = children(ar, body)?;
    let len = u32::try_from(value(ar, len)?).map_err(|_| Error::Range)?;
    if len > max {
        return Err(Error::Limit);
    }
    Ok((len, root))
}

/// Handles are local to the one arena used by this reference harness.
/// Guest values and the external ABI never expose these host Orders.
#[derive(Clone, Copy, Debug)]
pub struct Seq {
    len: u32,
    root: Order,
}

impl Seq {
    pub fn from_values<const N: usize>(
        ar: &mut Reduction<N>,
        values: &[Order],
        max: u32,
    ) -> Result<Self> {
        let len = u32::try_from(values.len()).map_err(|_| Error::Limit)?;
        if len > max {
            return Err(Error::Limit);
        }
        Ok(Self {
            len,
            root: build(ar, values, height(len))?,
        })
    }

    pub fn decode<const N: usize>(
        ar: &mut Reduction<N>,
        n: Order,
        max: u32,
        mut visits: u32,
    ) -> Result<Self> {
        let (len, root) = unwrap(ar, n, SEQ, max)?;
        validate_tree(ar, root, len, height(len), &mut visits)?;
        Ok(Self { len, root })
    }

    pub fn encode<const N: usize>(self, ar: &mut Reduction<N>) -> Result<Order> {
        wrap(ar, SEQ, self.len, self.root)
    }

    pub fn len(self) -> u32 {
        self.len
    }

    pub fn get<const N: usize>(self, ar: &Reduction<N>, i: u32) -> Result<Order> {
        if i >= self.len {
            return Err(Error::Index);
        }
        let mut root = self.root;
        for bit in (0..height(self.len)).rev() {
            let (left, right) = children(ar, root)?;
            root = if i & (1 << bit) == 0 { left } else { right };
        }
        Ok(root)
    }

    pub fn set<const N: usize>(self, ar: &mut Reduction<N>, i: u32, v: Order) -> Result<Self> {
        if i >= self.len {
            return Err(Error::Index);
        }
        Ok(Self {
            len: self.len,
            root: edit(ar, self.root, height(self.len), i, v)?,
        })
    }

    pub fn push<const N: usize>(self, ar: &mut Reduction<N>, v: Order, max: u32) -> Result<Self> {
        if self.len >= max {
            return Err(Error::Limit);
        }
        let next_len = self.len.checked_add(1).ok_or(Error::Limit)?;
        let mut root = self.root;
        let h = height(next_len);
        if h > height(self.len) {
            let right = empty_tree(ar, h - 1)?;
            root = pair(ar, root, right)?;
        }
        Ok(Self {
            len: next_len,
            root: edit(ar, root, h, self.len, v)?,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Bytes {
    len: u32,
    words: Seq,
}

pub fn word_count(len: u32) -> u32 {
    len / 4 + u32::from(len % 4 != 0)
}

impl Bytes {
    pub fn from_slice<const N: usize>(
        ar: &mut Reduction<N>,
        bytes: &[u8],
        max: u32,
    ) -> Result<Self> {
        let len = u32::try_from(bytes.len()).map_err(|_| Error::Limit)?;
        if len > max {
            return Err(Error::Limit);
        }
        let mut words = Vec::new();
        for chunk in bytes.chunks(4) {
            let mut word = [0; 4];
            word[..chunk.len()].copy_from_slice(chunk);
            words.push(atom(ar, u64::from(u32::from_le_bytes(word)))?);
        }
        Ok(Self {
            len,
            words: Seq::from_values(ar, &words, word_count(max))?,
        })
    }

    pub fn encode<const N: usize>(self, ar: &mut Reduction<N>) -> Result<Order> {
        wrap(ar, BYTES, self.len, self.words.root)
    }

    pub fn decode<const N: usize>(
        ar: &mut Reduction<N>,
        n: Order,
        max: u32,
        mut visits: u32,
    ) -> Result<Self> {
        let (len, root) = unwrap(ar, n, BYTES, max)?;
        let words = Seq {
            len: word_count(len),
            root,
        };
        validate_tree(ar, root, words.len, height(words.len), &mut visits)?;
        for i in 0..words.len {
            // The payload scan is also charged; shape validation alone cannot
            // hide an unbounded second traversal behind a small visit budget.
            visits = visits
                .checked_sub(height(words.len) + 1)
                .ok_or(Error::Visits)?;
            let word = value(ar, words.get(ar, i)?)?;
            if word > u64::from(u32::MAX) {
                return Err(Error::Range);
            }
            if i + 1 == words.len && len % 4 != 0 && word >> (8 * (len % 4)) != 0 {
                return Err(Error::Padding);
            }
        }
        Ok(Self { len, words })
    }

    pub fn len(self) -> u32 {
        self.len
    }

    pub fn get<const N: usize>(self, ar: &Reduction<N>, i: u32) -> Result<u8> {
        if i >= self.len {
            return Err(Error::Index);
        }
        let word = value(ar, self.words.get(ar, i / 4)?)?;
        Ok(((word >> (8 * (i % 4))) & 255) as u8)
    }

    pub fn set<const N: usize>(self, ar: &mut Reduction<N>, i: u32, byte: u32) -> Result<Self> {
        if i >= self.len {
            return Err(Error::Index);
        }
        if byte > 255 {
            return Err(Error::Range);
        }
        let old = value(ar, self.words.get(ar, i / 4)?)?;
        let shift = 8 * (i % 4);
        let word = (old & !(255 << shift)) | (u64::from(byte) << shift);
        let word = atom(ar, word)?;
        Ok(Self {
            len: self.len,
            words: self.words.set(ar, i / 4, word)?,
        })
    }

    pub fn push<const N: usize>(self, ar: &mut Reduction<N>, byte: u32, max: u32) -> Result<Self> {
        if self.len >= max {
            return Err(Error::Limit);
        }
        if byte > 255 {
            return Err(Error::Range);
        }
        let len = self.len.checked_add(1).ok_or(Error::Limit)?;
        if self.len % 4 == 0 {
            let word = atom(ar, u64::from(byte))?;
            Ok(Self {
                len,
                words: self.words.push(ar, word, word_count(max))?,
            })
        } else {
            Self {
                len,
                words: self.words,
            }
            .set(ar, self.len, byte)
        }
    }
}

/// Only print bounded fixtures; never recursively expand an untrusted DAG.
pub fn display<const N: usize>(ar: &Reduction<N>, n: Order, visits: &mut u32) -> Result<String> {
    *visits = visits.checked_sub(1).ok_or(Error::Visits)?;
    if let Some(v) = ar.atom_value(n) {
        return Ok(v.as_u64().to_string());
    }
    let (a, b) = children(ar, n)?;
    Ok(format!(
        "[{} {}]",
        display(ar, a, visits)?,
        display(ar, b, visits)?
    ))
}

#[cfg(test)]
mod boundary_tests {
    use super::*;

    #[test]
    fn sparse_height32_updates_and_growth_do_not_overflow_u32() {
        let mut ar = Reduction::<1024>::new();
        let zero = atom(&mut ar, 0).unwrap();
        let replacement = atom(&mut ar, 77).unwrap();
        // Construct a known-valid shared zero tree directly. This is not a
        // claim to have visited/validated billions of leaves on the VM.
        let seq = Seq {
            len: u32::MAX,
            root: empty_tree(&mut ar, 32).unwrap(),
        };
        let changed = seq.set(&mut ar, u32::MAX - 1, replacement).unwrap();
        assert_eq!(changed.get(&ar, u32::MAX - 1), Ok(replacement));
        assert_eq!(changed.get(&ar, u32::MAX - 2), Ok(zero));
        assert_eq!(seq.get(&ar, u32::MAX - 1), Ok(zero));
        assert_eq!(seq.get(&ar, u32::MAX), Err(Error::Index));
        assert_eq!(
            seq.push(&mut ar, replacement, u32::MAX).unwrap_err(),
            Error::Limit
        );
        let full = Seq {
            len: 1 << 31,
            root: empty_tree(&mut ar, 31).unwrap(),
        };
        let grown = full.push(&mut ar, replacement, u32::MAX).unwrap();
        assert_eq!(grown.len(), (1 << 31) + 1);
        assert_eq!(grown.get(&ar, 1 << 31), Ok(replacement));
        assert_eq!(grown.get(&ar, (1 << 31) - 1), Ok(zero));
        let root = seq.encode(&mut ar).unwrap();
        assert_eq!(
            Seq::decode(&mut ar, root, u32::MAX, 100).unwrap_err(),
            Error::Visits
        );
    }
}
