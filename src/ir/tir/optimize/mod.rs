// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
/// TIR peephole optimizer.
///
/// Runs pattern-based rewrites on Vec<TIROp> to reduce instruction count.
/// Applied between TIR building and lowering to target assembly.
use super::TIROp;

pub(crate) mod spill;
#[cfg(test)]
mod tests;

/// Apply all peephole optimizations until no more changes occur.
pub(crate) fn optimize(ops: Vec<TIROp>) -> Vec<TIROp> {
    let mut ir = ops;
    loop {
        let before = ir.len();
        ir = merge_hints(ir);
        ir = merge_pops(ir);
        ir = eliminate_nops(ir);
        ir = spill::eliminate_dead_spills(ir);
        ir = eliminate_dup_pop_nops(ir);
        ir = eliminate_double_swaps(ir);
        ir = collapse_swap_pop_chains(ir);
        ir = collapse_epilogue_cleanup(ir);
        ir = optimize_nested(ir);
        if ir.len() == before {
            break;
        }
    }
    ir
}

/// Merge consecutive Hint(a), Hint(b) -> Hint(a+b).
fn merge_hints(ops: Vec<TIROp>) -> Vec<TIROp> {
    let mut out: Vec<TIROp> = Vec::with_capacity(ops.len());
    let mut i = 0;
    while i < ops.len() {
        if let TIROp::Hint(n) = &ops[i] {
            let mut total = *n;
            let mut j = i + 1;
            while j < ops.len() {
                if let TIROp::Hint(m) = &ops[j] {
                    total += m;
                    j += 1;
                } else {
                    break;
                }
            }
            if total > 0 {
                out.push(TIROp::Hint(total));
            }
            i = j;
        } else {
            out.push(ops[i].clone());
            i += 1;
        }
    }
    out
}

/// Merge consecutive Pop(a), Pop(b) -> Pop(a+b).
fn merge_pops(ops: Vec<TIROp>) -> Vec<TIROp> {
    let mut out: Vec<TIROp> = Vec::with_capacity(ops.len());
    let mut i = 0;
    while i < ops.len() {
        if let TIROp::Pop(n) = &ops[i] {
            let mut total = *n;
            let mut j = i + 1;
            while j < ops.len() {
                if let TIROp::Pop(m) = &ops[j] {
                    total += m;
                    j += 1;
                } else {
                    break;
                }
            }
            if total > 0 {
                out.push(TIROp::Pop(total));
            }
            i = j;
        } else {
            out.push(ops[i].clone());
            i += 1;
        }
    }
    out
}

/// Remove no-op instructions: Swap(0), Pop(0).
fn eliminate_nops(ops: Vec<TIROp>) -> Vec<TIROp> {
    ops.into_iter()
        .filter(|op| !matches!(op, TIROp::Swap(0) | TIROp::Pop(0)))
        .collect()
}

/// Eliminate `Dup(0); Pop(1)` and `Dup(0); Swap(1); Pop(1)` no-ops.
///
/// `dup 0; pop 1` duplicates the top element then immediately discards it.
/// `dup 0; swap 1; pop 1` copies top, swaps with element below, pops -- net
/// effect is identity (the original value below is replaced by an identical copy).
fn eliminate_dup_pop_nops(ops: Vec<TIROp>) -> Vec<TIROp> {
    let mut out: Vec<TIROp> = Vec::with_capacity(ops.len());
    let mut i = 0;
    while i < ops.len() {
        // Pattern: Dup(0), Swap(1), Pop(1) -> skip all three
        if i + 2 < ops.len() {
            if let (TIROp::Dup(0), TIROp::Swap(1), TIROp::Pop(1)) =
                (&ops[i], &ops[i + 1], &ops[i + 2])
            {
                i += 3;
                continue;
            }
        }
        // Pattern: Dup(0), Pop(1) -> skip both
        if i + 1 < ops.len() {
            if let (TIROp::Dup(0), TIROp::Pop(1)) = (&ops[i], &ops[i + 1]) {
                i += 2;
                continue;
            }
        }
        out.push(ops[i].clone());
        i += 1;
    }
    out
}

/// Eliminate consecutive `Swap(N); Swap(N)` pairs (double swap is identity).
fn eliminate_double_swaps(ops: Vec<TIROp>) -> Vec<TIROp> {
    let mut out: Vec<TIROp> = Vec::with_capacity(ops.len());
    let mut i = 0;
    while i < ops.len() {
        if i + 1 < ops.len() {
            if let (TIROp::Swap(a), TIROp::Swap(b)) = (&ops[i], &ops[i + 1]) {
                if a == b {
                    i += 2;
                    continue;
                }
            }
        }
        out.push(ops[i].clone());
        i += 1;
    }
    out
}

/// Collapse `swap D; pop 1` chains used for stack cleanup.
///
/// Pattern 1: `swap 1; pop 1; return` means the top element is the return value
/// and the element below it is garbage. This is already minimal (2 instructions).
///
/// Pattern 2: Multiple consecutive `swap D; pop 1` pairs with decreasing D
/// right before `return` -- these remove locals from below the return value.
/// When the return value width is 1 and all elements below it are being removed,
/// we can sometimes replace the entire chain with `swap N; pop N` followed by return.
///
/// Pattern 3: `dup D; dup D; ... (K times); swap K; pop K` -- this duplicates
/// K elements from depth D, then removes the originals. If the originals aren't
/// needed after, this is just copying. When the dups reference a contiguous block
/// that is immediately popped, the net effect is a no-op (elements stay in place).
fn collapse_swap_pop_chains(ops: Vec<TIROp>) -> Vec<TIROp> {
    let mut out: Vec<TIROp> = Vec::with_capacity(ops.len());
    let mut i = 0;
    while i < ops.len() {
        // Pattern: dup D, dup D, ..., dup D (N times), swap N, pop N1, pop N2, ...
        // where the total popped equals N and D == N-1.
        // This is "extract copy of block at depth D, discard original."
        // Net: the N elements stay on the stack without the dup+pop round trip.
        if let TIROp::Dup(d) = &ops[i] {
            let d_val = *d;
            // Count consecutive dup D instructions with the same D value.
            let mut dup_count = 0u32;
            let mut j = i;
            while j < ops.len() {
                if let TIROp::Dup(dd) = &ops[j] {
                    if *dd == d_val {
                        dup_count += 1;
                        j += 1;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            // After the dups, check for swap(dup_count) followed by pop totaling dup_count.
            if dup_count >= 2 && j < ops.len() {
                if let TIROp::Swap(s) = &ops[j] {
                    if *s == dup_count {
                        let after_swap = j + 1;
                        let mut total_popped = 0u32;
                        let mut k = after_swap;
                        while k < ops.len() {
                            if let TIROp::Pop(p) = &ops[k] {
                                total_popped += p;
                                k += 1;
                                if total_popped >= dup_count {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        if total_popped == dup_count && d_val + 1 == dup_count {
                            // The dup+swap+pop sequence is a no-op: elements are
                            // already in the right position. Skip everything.
                            i = k;
                            continue;
                        }
                    }
                }
            }
        }
        out.push(ops[i].clone());
        i += 1;
    }
    out
}

/// Collapse repeated removal of the word immediately below the top.
/// Wider or decreasing-depth chains permute surviving values and must remain.
fn collapse_epilogue_cleanup(ops: Vec<TIROp>) -> Vec<TIROp> {
    let mut out = Vec::with_capacity(ops.len());
    let mut i = 0;
    while i < ops.len() {
        let mut end = i;
        while end + 1 < ops.len()
            && matches!(ops[end], TIROp::Swap(1))
            && matches!(ops[end + 1], TIROp::Pop(1))
        {
            end += 2;
        }
        if end > i {
            let count = ((end - i) / 2) as u32;
            out.extend([TIROp::Swap(count), TIROp::Pop(count)]);
            i = end;
        } else {
            out.push(ops[i].clone());
            i += 1;
        }
    }
    out
}

/// Recursively optimize nested bodies (IfElse, IfOnly, Loop, ProofBlock).
fn optimize_nested(ops: Vec<TIROp>) -> Vec<TIROp> {
    ops.into_iter()
        .map(|op| match op {
            TIROp::IfElse {
                then_body,
                else_body,
            } => TIROp::IfElse {
                then_body: optimize(then_body),
                else_body: optimize(else_body),
            },
            TIROp::IfOnly { then_body } => TIROp::IfOnly {
                then_body: optimize(then_body),
            },
            TIROp::Loop { label, body } => TIROp::Loop {
                label,
                body: optimize(body),
            },
            TIROp::ProofBlock { program_hash, body } => TIROp::ProofBlock {
                program_hash,
                body: optimize(body),
            },
            other => other,
        })
        .collect()
}
