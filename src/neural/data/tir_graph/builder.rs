//! Graph construction: flatten TIR ops, extract data deps and mem order edges.

use crate::ir::tir::TIROp;

use super::node::TirNode;
use super::types::{EdgeKind, FieldType, OpKind};

/// Determine the field type of a TIROp's output.
fn output_field_type(op: &TIROp) -> FieldType {
    match op {
        TIROp::ExtMul | TIROp::ExtInvert => FieldType::XFE,
        TIROp::FoldExt => FieldType::XFE,
        TIROp::SpongeSqueeze => FieldType::BFE,
        TIROp::Hash { .. } => FieldType::BFE,
        TIROp::Add
        | TIROp::Sub
        | TIROp::Mul
        | TIROp::Neg
        | TIROp::Invert
        | TIROp::Eq
        | TIROp::Lt
        | TIROp::And
        | TIROp::Or
        | TIROp::Xor
        | TIROp::DivMod
        | TIROp::Split
        | TIROp::Shl
        | TIROp::Shr
        | TIROp::Log2
        | TIROp::Pow
        | TIROp::PopCount
        | TIROp::Push(_) => FieldType::BFE,
        _ => FieldType::Unknown,
    }
}

/// Flatten TIR ops into graph nodes, handling structural ops recursively.
/// Adds ControlFlow edges between sequential ops and into/out-of bodies.
pub(super) fn flatten_ops(
    ops: &[TIROp],
    nodes: &mut Vec<TirNode>,
    edges: &mut Vec<(usize, usize, EdgeKind)>,
) {
    let mut prev_idx: Option<usize> = None;

    for op in ops {
        let idx = nodes.len();

        // Determine immediate value (Q5: BFE only, XFE ops get None)
        let immediate = match op {
            TIROp::Push(v) => Some(*v),
            TIROp::Pop(n) | TIROp::Dup(n) | TIROp::Swap(n) => Some(*n as u64),
            TIROp::ReadIo(n)
            | TIROp::WriteIo(n)
            | TIROp::ReadMem(n)
            | TIROp::WriteMem(n)
            | TIROp::Assert(n)
            | TIROp::Hint(n) => Some(*n as u64),
            TIROp::Hash { width } | TIROp::RamRead { width } | TIROp::RamWrite { width } => {
                Some(*width as u64)
            }
            TIROp::Reveal { field_count, .. } | TIROp::Seal { field_count, .. } => {
                Some(*field_count as u64)
            }
            TIROp::Asm { effect, .. } => Some(*effect as u64),
            // XFE ops: has_immediate=0 per Q5 resolution
            TIROp::ExtMul | TIROp::ExtInvert => None,
            _ => None,
        };

        let node = TirNode {
            op: OpKind::from_tir_op(op),
            field_type: output_field_type(op),
            immediate,
        };
        nodes.push(node);

        // Sequential ControlFlow edge
        if let Some(p) = prev_idx {
            edges.push((p, idx, EdgeKind::ControlFlow));
        }

        // Recurse into structural bodies
        match op {
            TIROp::IfElse {
                then_body,
                else_body,
            } => {
                if !then_body.is_empty() {
                    let then_start = nodes.len();
                    flatten_ops(then_body, nodes, edges);
                    edges.push((idx, then_start, EdgeKind::ControlFlow));
                }
                if !else_body.is_empty() {
                    let else_start = nodes.len();
                    flatten_ops(else_body, nodes, edges);
                    edges.push((idx, else_start, EdgeKind::ControlFlow));
                }
            }
            TIROp::IfOnly { then_body } => {
                if !then_body.is_empty() {
                    let then_start = nodes.len();
                    flatten_ops(then_body, nodes, edges);
                    edges.push((idx, then_start, EdgeKind::ControlFlow));
                }
            }
            TIROp::Loop { body, .. } => {
                if !body.is_empty() {
                    let body_start = nodes.len();
                    flatten_ops(body, nodes, edges);
                    let body_end = nodes.len() - 1;
                    edges.push((idx, body_start, EdgeKind::ControlFlow));
                    // Back edge: loop body end -> loop header
                    edges.push((body_end, idx, EdgeKind::ControlFlow));
                }
            }
            TIROp::ProofBlock { body, .. } => {
                if !body.is_empty() {
                    let body_start = nodes.len();
                    flatten_ops(body, nodes, edges);
                    edges.push((idx, body_start, EdgeKind::ControlFlow));
                }
            }
            _ => {}
        }

        prev_idx = Some(idx);
    }
}

/// Abstract stack entry: tracks which node produced this value.
#[derive(Clone, Copy)]
struct StackEntry {
    producer: usize,
}

/// Extract DataDep edges by simulating an abstract stack.
/// When op B pops a value produced by op A -> edge (A->B, DataDep).
pub(super) fn extract_data_deps(
    nodes: &[TirNode],
    edges: &mut Vec<(usize, usize, EdgeKind)>,
) {
    let mut stack: Vec<StackEntry> = Vec::new();

    for (idx, node) in nodes.iter().enumerate() {
        let (pops, pushes) = stack_effect_from_kind(node);

        // Pop: create DataDep edges from producers to this consumer
        let actual_pops = pops.min(stack.len());
        for _ in 0..actual_pops {
            if let Some(entry) = stack.pop() {
                edges.push((entry.producer, idx, EdgeKind::DataDep));
            }
        }

        // Handle Dup specially: reads from depth without consuming
        if node.op == OpKind::Dup {
            let depth = node.immediate.unwrap_or(0) as usize;
            if depth < stack.len() {
                let producer = stack[stack.len() - 1 - depth].producer;
                edges.push((producer, idx, EdgeKind::DataDep));
            }
        }

        // Handle Swap: creates read-dependencies on both swapped positions
        if node.op == OpKind::Swap {
            let depth = node.immediate.unwrap_or(1) as usize;
            if depth < stack.len() && !stack.is_empty() {
                let top = stack.len() - 1;
                let other = stack.len() - 1 - depth;
                stack.swap(top, other);
            }
        }

        // Push: record this node as producer
        for _ in 0..pushes {
            stack.push(StackEntry { producer: idx });
        }
    }
}

/// Get stack effect from a TirNode (using OpKind + immediate).
fn stack_effect_from_kind(node: &TirNode) -> (usize, usize) {
    let n = node.immediate.unwrap_or(0) as usize;
    match node.op {
        OpKind::Push => (0, 1),
        OpKind::Pop => (n, 0),
        OpKind::Dup => (0, 1),
        OpKind::Swap => (0, 0),
        OpKind::Add | OpKind::Sub | OpKind::Mul => (2, 1),
        OpKind::Neg | OpKind::Invert => (1, 1),
        OpKind::Eq | OpKind::Lt => (2, 1),
        OpKind::And | OpKind::Or | OpKind::Xor => (2, 1),
        OpKind::PopCount | OpKind::Log2 => (1, 1),
        OpKind::Split => (1, 2),
        OpKind::DivMod => (2, 2),
        OpKind::Shl | OpKind::Shr | OpKind::Pow => (2, 1),
        OpKind::ReadIo => (0, n),
        OpKind::WriteIo => (n, 0),
        OpKind::ReadMem => (1, n + 1),
        OpKind::WriteMem => (n + 1, 1),
        OpKind::Assert => (n, 0),
        OpKind::Hash => (10, 5),
        OpKind::Reveal | OpKind::Seal => (n, 0),
        OpKind::RamRead => (1, n),
        OpKind::RamWrite => (n + 1, 0),
        OpKind::Hint => (0, n),
        OpKind::SpongeInit => (0, 0),
        OpKind::SpongeAbsorb => (10, 0),
        OpKind::SpongeSqueeze => (0, 10),
        OpKind::SpongeLoad => (1, 1),
        OpKind::MerkleStep | OpKind::MerkleLoad => (0, 0),
        OpKind::ExtMul => (6, 3),
        OpKind::ExtInvert => (3, 3),
        OpKind::FoldExt | OpKind::FoldBase => (0, 0),
        OpKind::IfElse | OpKind::IfOnly | OpKind::Loop => (1, 0),
        _ => (0, 0),
    }
}

/// Extract MemOrder edges: pairwise between all memory operations.
/// Conservative — preserves all possible orderings.
pub(super) fn extract_mem_order(
    nodes: &[TirNode],
    edges: &mut Vec<(usize, usize, EdgeKind)>,
) {
    let mem_indices: Vec<usize> = nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| {
            matches!(
                n.op,
                OpKind::ReadMem
                    | OpKind::WriteMem
                    | OpKind::RamRead
                    | OpKind::RamWrite
                    | OpKind::SpongeLoad
                    | OpKind::MerkleLoad
            )
        })
        .map(|(i, _)| i)
        .collect();

    // Pairwise edges between consecutive memory ops (not O(n^2) — sequential ordering)
    for window in mem_indices.windows(2) {
        edges.push((window[0], window[1], EdgeKind::MemOrder));
    }
}
