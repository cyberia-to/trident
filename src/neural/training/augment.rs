//! Shared augmentation orchestration and TIR graph transformations.
use crate::neural::data::pairs::TrainingPair;
use crate::neural::data::tir_graph::TirGraph;
use crate::neural::Target;

pub struct AugmentConfig {
    /// Number of TIR reordering variants per seed pair.
    pub tir_reorder_variants: usize,
    /// Number of target assembly random-walk variants per seed pair.
    pub assembly_walk_variants: usize,
    /// Max swap attempts per random walk.
    pub max_swap_attempts: usize,
    /// Random seed for reproducibility.
    pub seed: u64,
}

impl Default for AugmentConfig {
    fn default() -> Self {
        Self {
            tir_reorder_variants: 10,
            assembly_walk_variants: 50,
            max_swap_attempts: 20,
            seed: 0xDEAD_BEEF_A097,
        }
    }
}

pub fn augment_pairs(
    pairs: &[TrainingPair],
    target: &dyn Target,
    config: &AugmentConfig,
) -> Vec<TrainingPair> {
    let mut result = Vec::new();
    let mut rng = Xorshift64::new(config.seed);
    for pair in pairs {
        result.push(TrainingPair {
            graph: pair.graph.clone(),
            target_tokens: pair.target_tokens.clone(),
            source_id: pair.source_id.clone(),
            baseline_cost: pair.baseline_cost,
        });
        let assembly = target.vocabulary().decode_sequence(&pair.target_tokens);
        for (index, candidate) in target
            .augment(
                &assembly,
                rng.next(),
                config.assembly_walk_variants,
                config.max_swap_attempts,
            )
            .into_iter()
            .enumerate()
        {
            if !target.verify_block(&assembly, &candidate, config.seed) {
                continue;
            }
            let tokens = target.vocabulary().encode_sequence(&candidate);
            if tokens.len() <= 1 {
                continue;
            }
            result.push(TrainingPair {
                graph: pair.graph.clone(),
                target_tokens: tokens,
                source_id: format!("{}:target{}", pair.source_id, index),
                baseline_cost: target.cost(&candidate),
            });
        }
        for index in 0..config.tir_reorder_variants {
            result.push(TrainingPair {
                graph: insert_dead_code(&pair.graph, &mut rng),
                target_tokens: pair.target_tokens.clone(),
                source_id: format!("{}:tir{}", pair.source_id, index),
                baseline_cost: pair.baseline_cost,
            });
        }
    }
    result
}

// ─── Dead Code Insertion (TIR-side) ──────────────────────────────

/// Insert dead code nodes into a TirGraph.
///
/// Adds operations that don't affect the output: push+pop pairs,
/// dup+pop pairs, nop sequences. The model must learn to ignore these.
fn insert_dead_code(graph: &TirGraph, rng: &mut Xorshift64) -> TirGraph {
    use crate::neural::data::tir_graph::{EdgeKind, FieldType, OpKind, TirNode};

    let mut nodes = graph.nodes.clone();
    let mut edges = graph.edges.clone();

    // Number of dead code insertions: 1-3
    let num_insertions = 1 + (rng.next() % 3) as usize;

    for _ in 0..num_insertions {
        if nodes.is_empty() {
            break;
        }

        // Pick random insertion point
        let insert_at = (rng.next() % nodes.len() as u64) as usize;
        let dead_kind = rng.next() % 3;

        let dead_nodes: Vec<TirNode> = match dead_kind {
            0 => {
                // push + pop pair
                vec![
                    TirNode {
                        op: OpKind::Push,
                        field_type: FieldType::BFE,
                        immediate: Some(0),
                    },
                    TirNode {
                        op: OpKind::Pop,
                        field_type: FieldType::Unknown,
                        immediate: Some(1),
                    },
                ]
            }
            1 => {
                // dup 0 + pop 1 (if stack nonempty — conservative: always add push first)
                vec![
                    TirNode {
                        op: OpKind::Push,
                        field_type: FieldType::BFE,
                        immediate: Some(0),
                    },
                    TirNode {
                        op: OpKind::Dup,
                        field_type: FieldType::BFE,
                        immediate: Some(0),
                    },
                    TirNode {
                        op: OpKind::Pop,
                        field_type: FieldType::Unknown,
                        immediate: Some(2),
                    },
                ]
            }
            _ => {
                // Single nop-like: push 0; push 0; add; pop 1
                vec![
                    TirNode {
                        op: OpKind::Push,
                        field_type: FieldType::BFE,
                        immediate: Some(0),
                    },
                    TirNode {
                        op: OpKind::Push,
                        field_type: FieldType::BFE,
                        immediate: Some(0),
                    },
                    TirNode {
                        op: OpKind::Add,
                        field_type: FieldType::BFE,
                        immediate: None,
                    },
                    TirNode {
                        op: OpKind::Pop,
                        field_type: FieldType::Unknown,
                        immediate: Some(1),
                    },
                ]
            }
        };

        let num_dead = dead_nodes.len();

        // Shift all edge indices >= insert_at by num_dead
        for edge in edges.iter_mut() {
            if edge.0 >= insert_at {
                edge.0 += num_dead;
            }
            if edge.1 >= insert_at {
                edge.1 += num_dead;
            }
        }

        // Insert dead nodes
        let mut new_nodes = nodes[..insert_at].to_vec();
        new_nodes.extend(dead_nodes);
        new_nodes.extend_from_slice(&nodes[insert_at..]);
        nodes = new_nodes;

        // Add control flow edges within dead code
        for j in 0..num_dead.saturating_sub(1) {
            edges.push((insert_at + j, insert_at + j + 1, EdgeKind::ControlFlow));
        }

        // Add data dep edges within dead code (push→pop, push→dup, etc.)
        if num_dead >= 2 {
            edges.push((insert_at, insert_at + num_dead - 1, EdgeKind::DataDep));
        }

        // Connect to surrounding control flow
        if insert_at > 0 {
            edges.push((insert_at - 1, insert_at, EdgeKind::ControlFlow));
        }
        if insert_at + num_dead < nodes.len() {
            edges.push((
                insert_at + num_dead - 1,
                insert_at + num_dead,
                EdgeKind::ControlFlow,
            ));
        }
    }

    TirGraph { nodes, edges }
}

// ─── PRNG ─────────────────────────────────────────────────────────

/// Simple xorshift64 PRNG for reproducible augmentation.
struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: seed | 1, // ensure non-zero
        }
    }

    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
}

// ─── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tir::TIROp;
    #[test]
    fn dead_code_increases_graph_size() {
        let ops = vec![TIROp::Push(1), TIROp::Push(2), TIROp::Add];
        let graph = TirGraph::from_tir_ops(&ops);
        let original_size = graph.num_nodes();

        let mut rng = Xorshift64::new(42);
        let augmented = insert_dead_code(&graph, &mut rng);
        assert!(
            augmented.num_nodes() > original_size,
            "dead code should increase graph size",
        );
    }
}
