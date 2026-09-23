//! Bounded noun paths with canonical machine axes, including deep aggregates.
use super::*;

const MAX_PATH_STEPS: usize = 4096;
const CHUNK_STEPS: usize = 62;

#[derive(Clone)]
pub(super) struct AxisPath(Vec<bool>);

impl AxisPath {
    pub(super) fn from_axis(axis: u64) -> Result<Self, String> {
        if axis == 0 {
            return Err("nox: axis zero has no value".into());
        }
        let bits = 63 - axis.leading_zeros();
        Ok(Self(
            (0..bits).rev().map(|i| ((axis >> i) & 1) != 0).collect(),
        ))
    }

    pub(super) fn append_element(&mut self, index: u64) -> Result<(), String> {
        if index >= MAX_PATH_STEPS as u64 || self.0.len() + index as usize + 1 > MAX_PATH_STEPS {
            return Err("nox: aggregate access exceeds the noun path limit".into());
        }
        self.0.extend(std::iter::repeat_n(true, index as usize));
        self.0.push(false);
        Ok(())
    }

    fn check(&self) -> Result<(), String> {
        if self.0.len() > MAX_PATH_STEPS {
            Err("nox: aggregate access exceeds the noun path limit".into())
        } else {
            Ok(())
        }
    }

    pub(super) fn access(&self) -> Noun {
        let mut chunks = self.0.chunks(CHUNK_STEPS);
        let axis = |chunk: &[bool]| {
            chunk
                .iter()
                .fold(1u64, |a, right| (a << 1) | u64::from(*right))
        };
        let mut result = nox_axis(chunks.next().map(axis).unwrap_or(1));
        for chunk in chunks {
            result = seq(result, nox_axis(axis(chunk)));
        }
        result
    }

    /// All siblings and the RHS are evaluated against the original subject.
    /// The RHS occurs exactly once; reconstruction preserves every sibling.
    pub(super) fn edit(&self, value: Noun) -> LowerResult {
        self.check()?;
        let depth = self.0.len();
        if depth
            .saturating_mul(depth / CHUNK_STEPS + 1)
            .saturating_mul(12)
            > MAX_INLINE_NODES
        {
            return Err("nox: aggregate edit exceeds the emitted node budget".into());
        }
        let mut result = value;
        for i in (0..depth).rev() {
            let mut sibling = Self(self.0[..i].to_vec());
            sibling.0.push(!self.0[i]);
            let sibling = sibling.access();
            result = if self.0[i] {
                nox_cons(sibling, result)
            } else {
                nox_cons(result, sibling)
            };
        }
        Ok(result)
    }
}

pub(super) fn element_access(base: Noun, index: u64) -> LowerResult {
    let mut path = if let Noun::Cell(tag, addr) = &base {
        if let (Noun::Atom(0), Noun::Atom(a)) = (tag.as_ref(), addr.as_ref()) {
            let mut path = AxisPath::from_axis(*a)?;
            path.append_element(index)?;
            return Ok(path.access());
        }
        AxisPath::from_axis(1)?
    } else {
        AxisPath::from_axis(1)?
    };
    path.append_element(index)?;
    Ok(seq(base, path.access()))
}
