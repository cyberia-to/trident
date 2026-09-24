//! Stable balanced frame slots and code-table paths.
use super::super::*;

pub(super) fn height(count: usize) -> Result<u32, String> {
    if count > 65536 {
        return Err("native frame/table exceeds 65536 slots".into());
    }
    Ok(if count <= 1 {
        0
    } else {
        usize::BITS - (count - 1).leading_zeros()
    })
}

pub(super) fn axis(base: u64, height: u32, slot: usize) -> u64 {
    (base << height) | slot as u64
}

/// Leaves are in source order. Runtime cons therefore evaluates arguments once
/// left to right. Padding is zero; it cannot inspect or evaluate source values.
pub(super) fn tree(mut leaves: Vec<Noun>, count: usize, formulas: bool) -> LowerResult {
    let h = height(count)?;
    if leaves.len() > count {
        return Err("native frame leaf overflow".into());
    }
    leaves.resize_with(1usize << h, || {
        if formulas {
            nox_unit()
        } else {
            Noun::atom(0)
        }
    });
    while leaves.len() > 1 {
        let mut parents = Vec::with_capacity(leaves.len() / 2);
        let mut children = leaves.into_iter();
        while let Some(left) = children.next() {
            let right = children.next().ok_or("unbalanced native layout")?;
            parents.push(if formulas {
                nox_cons(left, right)
            } else {
                Noun::cell(left, right)
            });
        }
        leaves = parents;
    }
    leaves.pop().ok_or_else(|| "empty native tree".into())
}

pub(super) fn subject(table: Noun, frame: Noun) -> Noun {
    nox_cons(table, nox_cons(nox_unit(), frame))
}

pub(super) fn set(slot: usize, h: u32, value: Noun) -> LowerResult {
    AxisPath::from_axis(axis(7, h, slot))?.edit(value)
}

pub(super) fn keep(subject: Noun) -> Noun {
    nox_cons(nox_unit(), subject)
}
pub(super) fn returned(value: Noun) -> Noun {
    nox_cons(nox_quote(Noun::atom(1)), value)
}

/// First executes exactly once. Continue runs next against its new subject;
/// Return propagates without evaluating next or duplicating its formula.
pub(super) fn then(first: Noun, next: Noun) -> Noun {
    seq(
        first,
        nox_branch(nox_axis(2), seq(nox_axis(3), next), nox_axis(1)),
    )
}

pub(super) fn result(flow: Noun) -> Noun {
    seq(flow, nox_branch(nox_axis(2), nox_unit(), nox_axis(3)))
}
