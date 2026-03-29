// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! CFG → structured control flow conversion.
//!
//! Converts MIR's flat basic-block CFG into TIR's nested structural
//! control flow (IfElse, IfOnly, Loop). Rs edition guarantees reducible
//! CFGs — no gotos, no unwind, all loops bounded.

use std::collections::{BTreeMap, BTreeSet};

use mir_format::{MirBlock, MirTerminator};

/// A structured control flow region ready for TIR emission.
#[derive(Debug, Clone)]
pub enum Region {
    /// Linear sequence of block indices to emit in order.
    Linear(Vec<u32>),
    /// If/else branch. `cond_block` ends with SwitchInt.
    IfElse {
        cond_block: u32,
        then_region: Box<Region>,
        else_region: Box<Region>,
        merge: u32,
    },
    /// If without else.
    IfOnly {
        cond_block: u32,
        then_region: Box<Region>,
        merge: u32,
    },
    /// Bounded loop.
    Loop {
        header: u32,
        body: Box<Region>,
        exit: u32,
    },
    /// Sequence of regions.
    Seq(Vec<Region>),
}

/// Convert a flat CFG (Vec<MirBlock>) into a structured Region.
pub fn structurize(blocks: &[MirBlock]) -> Region {
    if blocks.is_empty() {
        return Region::Linear(vec![]);
    }

    let block_map: BTreeMap<u32, &MirBlock> = blocks.iter().map(|b| (b.index, b)).collect();
    let successors = build_successors(blocks);
    let predecessors = build_predecessors(blocks, &successors);
    let dominators = compute_dominators(blocks);
    let back_edges = find_back_edges(blocks, &successors, &dominators);

    structurize_region(
        0,
        blocks.len() as u32,
        &block_map,
        &successors,
        &predecessors,
        &dominators,
        &back_edges,
    )
}

// ── Successors / predecessors ──────────────────────────────

fn build_successors(blocks: &[MirBlock]) -> BTreeMap<u32, Vec<u32>> {
    let mut succs = BTreeMap::new();
    for block in blocks {
        let targets = match &block.terminator {
            MirTerminator::Goto { target } => vec![*target],
            MirTerminator::SwitchInt {
                targets, otherwise, ..
            } => {
                let mut t: Vec<u32> = targets.iter().map(|(_, bb)| *bb).collect();
                t.push(*otherwise);
                t
            }
            MirTerminator::Call { target, .. } => target.iter().copied().collect(),
            MirTerminator::Assert { target, .. } => vec![*target],
            MirTerminator::Return | MirTerminator::Unreachable => vec![],
        };
        succs.insert(block.index, targets);
    }
    succs
}

fn build_predecessors(
    blocks: &[MirBlock],
    succs: &BTreeMap<u32, Vec<u32>>,
) -> BTreeMap<u32, Vec<u32>> {
    let mut preds: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for block in blocks {
        preds.entry(block.index).or_default();
    }
    for (&src, dsts) in succs {
        for &dst in dsts {
            preds.entry(dst).or_default().push(src);
        }
    }
    preds
}

// ── Dominator computation (simple iterative) ───────────────

fn compute_dominators(blocks: &[MirBlock]) -> BTreeMap<u32, BTreeSet<u32>> {
    let all_indices: BTreeSet<u32> = blocks.iter().map(|b| b.index).collect();
    let mut doms: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();

    // Entry block dominated only by itself.
    let entry = blocks[0].index;
    doms.insert(entry, BTreeSet::from([entry]));

    // All other blocks start with dom = all blocks.
    for block in blocks.iter().skip(1) {
        doms.insert(block.index, all_indices.clone());
    }

    let succs = build_successors(blocks);
    let preds = build_predecessors(blocks, &succs);

    // Iterate until fixed point.
    let mut changed = true;
    while changed {
        changed = false;
        for block in blocks.iter().skip(1) {
            let idx = block.index;
            let pred_list = preds.get(&idx).cloned().unwrap_or_default();

            let mut new_dom = if pred_list.is_empty() {
                BTreeSet::new()
            } else {
                let mut iter = pred_list.iter();
                let first = *iter.next().unwrap();
                let mut intersection = doms.get(&first).cloned().unwrap_or_default();
                for &p in iter {
                    let p_dom = doms.get(&p).cloned().unwrap_or_default();
                    intersection = intersection.intersection(&p_dom).copied().collect();
                }
                intersection
            };
            new_dom.insert(idx);

            if new_dom != *doms.get(&idx).unwrap() {
                doms.insert(idx, new_dom);
                changed = true;
            }
        }
    }

    doms
}

// ── Back-edge detection ────────────────────────────────────

/// A back edge (src → dst) where dst dominates src = natural loop.
fn find_back_edges(
    blocks: &[MirBlock],
    succs: &BTreeMap<u32, Vec<u32>>,
    doms: &BTreeMap<u32, BTreeSet<u32>>,
) -> Vec<(u32, u32)> {
    let mut edges = Vec::new();
    for block in blocks {
        let src = block.index;
        let src_doms = doms.get(&src).cloned().unwrap_or_default();
        for &dst in succs.get(&src).unwrap_or(&vec![]) {
            if src_doms.contains(&dst) {
                edges.push((src, dst));
            }
        }
    }
    edges
}

// ── Region construction ────────────────────────────────────

fn structurize_region(
    start: u32,
    end: u32,
    block_map: &BTreeMap<u32, &MirBlock>,
    succs: &BTreeMap<u32, Vec<u32>>,
    preds: &BTreeMap<u32, Vec<u32>>,
    doms: &BTreeMap<u32, BTreeSet<u32>>,
    back_edges: &[(u32, u32)],
) -> Region {
    let mut regions = Vec::new();
    let mut current = start;

    while current < end {
        let block = match block_map.get(&current) {
            Some(b) => b,
            None => break,
        };

        // Check if this block is a loop header.
        let is_loop_header = back_edges.iter().any(|&(_, dst)| dst == current);

        if is_loop_header {
            // Find all blocks in the loop body (dominated by header, can reach header).
            let loop_blocks = find_loop_body(current, succs, doms, back_edges);
            let max_loop_block = loop_blocks.iter().max().copied().unwrap_or(current);

            // Find exit block (first successor of loop blocks not in loop).
            let exit = find_loop_exit(&loop_blocks, succs);

            let body_region = structurize_region(
                current,
                max_loop_block + 1,
                block_map,
                succs,
                preds,
                doms,
                &[], // no nested back-edges for now
            );

            regions.push(Region::Loop {
                header: current,
                body: Box::new(body_region),
                exit,
            });

            current = exit;
            continue;
        }

        // Check if this block is a branch (SwitchInt).
        if let MirTerminator::SwitchInt {
            targets, otherwise, ..
        } = &block.terminator
        {
            if targets.len() == 1 {
                // Boolean branch: if/else.
                let then_target = targets[0].1;
                let else_target = *otherwise;

                // Find merge point (post-dominator).
                let merge = find_merge(then_target, else_target, succs, end);

                let then_region = if then_target < merge {
                    structurize_region(
                        then_target,
                        merge,
                        block_map,
                        succs,
                        preds,
                        doms,
                        back_edges,
                    )
                } else {
                    Region::Linear(vec![])
                };

                let else_region = if else_target < merge && else_target != then_target {
                    structurize_region(
                        else_target,
                        merge,
                        block_map,
                        succs,
                        preds,
                        doms,
                        back_edges,
                    )
                } else {
                    Region::Linear(vec![])
                };

                // Emit the condition block as linear (statements only, terminator handled).
                regions.push(Region::Linear(vec![current]));
                regions.push(Region::IfElse {
                    cond_block: current,
                    then_region: Box::new(then_region),
                    else_region: Box::new(else_region),
                    merge,
                });

                current = merge;
                continue;
            }
        }

        // Linear block.
        regions.push(Region::Linear(vec![current]));

        // Advance to the next block.
        match &block.terminator {
            MirTerminator::Goto { target } => current = *target,
            MirTerminator::Call { target, .. } => {
                current = target.unwrap_or(current + 1);
            }
            MirTerminator::Assert { target, .. } => current = *target,
            MirTerminator::Return | MirTerminator::Unreachable => break,
            _ => {
                current += 1;
            }
        }
    }

    if regions.len() == 1 {
        regions.into_iter().next().unwrap()
    } else {
        Region::Seq(regions)
    }
}

// ── Helpers ────────────────────────────────────────────────

fn find_loop_body(
    header: u32,
    succs: &BTreeMap<u32, Vec<u32>>,
    _doms: &BTreeMap<u32, BTreeSet<u32>>,
    back_edges: &[(u32, u32)],
) -> BTreeSet<u32> {
    let mut body = BTreeSet::from([header]);

    // Collect all back-edge sources for this header.
    let back_sources: Vec<u32> = back_edges
        .iter()
        .filter(|&&(_, dst)| dst == header)
        .map(|&(src, _)| src)
        .collect();

    // Walk backwards from back-edge sources to header.
    let mut worklist = back_sources;
    while let Some(block) = worklist.pop() {
        if body.insert(block) {
            // Add predecessors (simple: any block that has this block as successor).
            for (&src, dsts) in succs {
                if dsts.contains(&block) && !body.contains(&src) {
                    worklist.push(src);
                }
            }
        }
    }

    body
}

fn find_loop_exit(loop_blocks: &BTreeSet<u32>, succs: &BTreeMap<u32, Vec<u32>>) -> u32 {
    for &block in loop_blocks {
        for &succ in succs.get(&block).unwrap_or(&vec![]) {
            if !loop_blocks.contains(&succ) {
                return succ;
            }
        }
    }
    // Fallback: next block after max.
    loop_blocks.iter().max().copied().unwrap_or(0) + 1
}

fn find_merge(then_target: u32, else_target: u32, succs: &BTreeMap<u32, Vec<u32>>, end: u32) -> u32 {
    // Walk forward from both branches to find convergence.
    let mut then_reachable = BTreeSet::new();
    let mut worklist = vec![then_target];
    while let Some(b) = worklist.pop() {
        if b < end && then_reachable.insert(b) {
            for &s in succs.get(&b).unwrap_or(&vec![]) {
                worklist.push(s);
            }
        }
    }

    let mut else_reachable = BTreeSet::new();
    worklist = vec![else_target];
    while let Some(b) = worklist.pop() {
        if b < end && else_reachable.insert(b) {
            for &s in succs.get(&b).unwrap_or(&vec![]) {
                worklist.push(s);
            }
        }
    }

    // Merge = lowest block reachable from both.
    for idx in 0..end {
        if then_reachable.contains(&idx) && else_reachable.contains(&idx) {
            return idx;
        }
    }

    end
}

#[cfg(test)]
mod tests {
    use super::*;
    use mir_format::{MirBlock, MirOperand, MirConstValue, MirPlace, MirTerminator};

    fn linear_blocks() -> Vec<MirBlock> {
        vec![
            MirBlock {
                index: 0,
                statements: vec![],
                terminator: MirTerminator::Goto { target: 1 },
            },
            MirBlock {
                index: 1,
                statements: vec![],
                terminator: MirTerminator::Return,
            },
        ]
    }

    fn if_else_blocks() -> Vec<MirBlock> {
        vec![
            MirBlock {
                index: 0,
                statements: vec![],
                terminator: MirTerminator::SwitchInt {
                    discriminant: MirOperand::Constant(MirConstValue::Bool(true)),
                    targets: vec![(0, 2)],
                    otherwise: 1,
                },
            },
            MirBlock {
                index: 1,
                statements: vec![],
                terminator: MirTerminator::Goto { target: 3 },
            },
            MirBlock {
                index: 2,
                statements: vec![],
                terminator: MirTerminator::Goto { target: 3 },
            },
            MirBlock {
                index: 3,
                statements: vec![],
                terminator: MirTerminator::Return,
            },
        ]
    }

    #[test]
    fn test_structurize_linear() {
        let region = structurize(&linear_blocks());
        match region {
            Region::Seq(regions) => {
                assert_eq!(regions.len(), 2);
            }
            Region::Linear(blocks) => {
                assert!(!blocks.is_empty());
            }
            _ => panic!("expected Seq or Linear"),
        }
    }

    #[test]
    fn test_structurize_if_else() {
        let region = structurize(&if_else_blocks());
        // Should produce a sequence containing an IfElse region.
        fn has_if_else(r: &Region) -> bool {
            match r {
                Region::IfElse { .. } => true,
                Region::Seq(rs) => rs.iter().any(has_if_else),
                _ => false,
            }
        }
        assert!(has_if_else(&region), "expected IfElse in structured output");
    }
}
