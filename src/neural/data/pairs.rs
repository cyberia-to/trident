// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Training pair extraction: (TirGraph, target assembly token sequence).
//!
//! Builds training pairs from compiled Trident source files.
//! Splits per-file TIR into per-function blocks, lowers each independently,
//! and creates (graph, tokens) pairs for each function.

use crate::ir::tir::TIROp;
use crate::neural::data::tir_graph::TirGraph;
use crate::neural::Vocabulary;

/// A single training pair: graph input + token sequence target.
pub struct TrainingPair {
    /// TIR graph representation of the input block.
    pub graph: TirGraph,
    /// Target target assembly as vocab token IDs (with EOS appended).
    pub target_tokens: Vec<u32>,
    /// Source identifier (e.g., "poseidon2:authenticate").
    pub source_id: String,
    /// Compiler baseline cost for this block.
    pub baseline_cost: u64,
}

/// Extract training pairs from pre-compiled data.
///
/// Each block is a `(tir_ops, assembly, source_id, baseline_cost)` tuple
/// representing a single function or code block.
pub fn extract_pairs(
    blocks: &[(Vec<TIROp>, Vec<String>, String, u64)],
    vocab: &dyn Vocabulary,
) -> Vec<TrainingPair> {
    let mut pairs = Vec::new();

    for (tir_ops, assembly, source_id, baseline_cost) in blocks {
        if tir_ops.is_empty() || assembly.is_empty() {
            continue;
        }

        // Build graph from TIR ops
        let graph = TirGraph::from_tir_ops(tir_ops);
        if graph.nodes.is_empty() {
            continue;
        }

        // Encode target assembly to token IDs
        let target_tokens = vocab.encode_sequence(assembly);
        if target_tokens.len() <= 1 {
            // Only EOS — no actual content
            continue;
        }

        pairs.push(TrainingPair {
            graph,
            target_tokens,
            source_id: source_id.clone(),
            baseline_cost: *baseline_cost,
        });
    }

    pairs
}

/// Split a file's TIR ops into per-function chunks.
///
/// Each chunk is `(function_name, Vec<TIROp>)`. The TIR ops between
/// `FnStart(name)` and `FnEnd` form one function. Ops outside any
/// function (entry prologue, etc.) are grouped as "__entry".
pub fn split_tir_by_function(ops: &[TIROp]) -> Vec<(String, Vec<TIROp>)> {
    let mut functions = Vec::new();
    let mut current_name = String::new();
    let mut current_ops = Vec::new();
    let mut in_function = false;

    for op in ops {
        match op {
            TIROp::FnStart(name) => {
                // Save any accumulated non-function ops
                if !current_ops.is_empty() && !in_function {
                    functions.push(("__entry".to_string(), std::mem::take(&mut current_ops)));
                }
                current_name = name.clone();
                current_ops = vec![op.clone()];
                in_function = true;
            }
            TIROp::FnEnd => {
                current_ops.push(op.clone());
                functions.push((
                    std::mem::take(&mut current_name),
                    std::mem::take(&mut current_ops),
                ));
                in_function = false;
            }
            _ => {
                current_ops.push(op.clone());
            }
        }
    }

    // Any trailing ops
    if !current_ops.is_empty() {
        let name = if in_function && !current_name.is_empty() {
            current_name
        } else {
            "__trailing".to_string()
        };
        functions.push((name, current_ops));
    }

    functions
}

/// Split training pairs into train and holdout sets.
/// Returns (train, holdout).
pub fn train_holdout_split(
    pairs: Vec<TrainingPair>,
    holdout_count: usize,
) -> (Vec<TrainingPair>, Vec<TrainingPair>) {
    if pairs.len() <= holdout_count {
        return (pairs, Vec::new());
    }
    let split_point = pairs.len() - holdout_count;
    let mut all = pairs;
    let holdout = all.split_off(split_point);
    (all, holdout)
}
