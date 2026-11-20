//! Formula builders and result classifiers shared by the per-backend regression
//! anchors (`*_anchors.rs`). Test-only: declared once in `lib.rs`.

use crate::CompileError;
use nebu::Goldilocks;
use nox::{Order as NounId, Reduction as Order};

pub(crate) fn atom_of<const N: usize>(o: &mut Order<N>, v: u64) -> NounId {
    o.atom(Goldilocks::new(v)).unwrap()
}

pub(crate) fn axis_of<const N: usize>(o: &mut Order<N>, ax: u64) -> NounId {
    let t = atom_of(o, 0);
    let a = atom_of(o, ax);
    o.pair(t, a).unwrap()
}

pub(crate) fn quote_of<const N: usize>(o: &mut Order<N>, v: u64) -> NounId {
    let t = atom_of(o, 1);
    let a = atom_of(o, v);
    o.pair(t, a).unwrap()
}

pub(crate) fn binary_of<const N: usize>(o: &mut Order<N>, tag: u64, l: NounId, r: NounId) -> NounId {
    let body = o.pair(l, r).unwrap();
    let t = atom_of(o, tag);
    o.pair(t, body).unwrap()
}

pub(crate) fn cons_of<const N: usize>(o: &mut Order<N>, head: NounId, tail: NounId) -> NounId {
    let body = o.pair(head, tail).unwrap();
    let t = atom_of(o, 3);
    o.pair(t, body).unwrap()
}

/// Right-nested sum with `live` leaves alive at once: each `a` stays in its
/// register while `b` is emitted (live=1 is a leaf; live=n is n-1 sums).
pub(crate) fn live_sum<const N: usize>(o: &mut Order<N>, live: u32) -> NounId {
    let mut f = axis_of(o, 2);
    for _ in 1..live {
        let leaf = axis_of(o, 2);
        f = binary_of(o, 5, leaf, f);
    }
    f
}

pub(crate) fn is_scratch_overflow<T>(r: &Result<T, CompileError>) -> bool {
    matches!(r, Err(CompileError::ScratchOverflow))
}

/// Loop formula `[2 [[3 [[1 0] inits]] [1 body]]]` with `carried` init values and
/// a body that adds two live values: `add(axis 2, axis 2)`.
pub(crate) fn loop_with_sum_body<const N: usize>(o: &mut Order<N>, carried: u64) -> NounId {
    let mut chain = axis_of(o, 1);
    for i in 0..carried {
        let init = quote_of(o, i + 1);
        chain = cons_of(o, init, chain);
    }
    let slot = quote_of(o, 0);
    let setup = cons_of(o, slot, chain);
    let (l, r) = (axis_of(o, 2), axis_of(o, 2));
    let body = binary_of(o, 5, l, r);
    let t1 = atom_of(o, 1);
    let quoted_body = o.pair(t1, body).unwrap();
    binary_of(o, 2, setup, quoted_body)
}

/// Little-endian 32-bit instruction words of a fixed-width encoding.
pub(crate) fn words_of(code: &[u8]) -> Vec<u32> {
    code.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}
