// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! TirGraph — graph representation of TIR for GNN encoding.
//!
//! Converts a flat `Vec<TIROp>` into a graph with typed edges:
//! - DataDep: producer->consumer via abstract stack simulation
//! - ControlFlow: sequential and branch edges
//! - MemOrder: conservative ordering between memory operations

mod builder;
pub mod node;
mod tests;
pub mod types;

// Re-export all public types at module level for backward compatibility.
pub use node::{TirNode, NODE_FEATURE_DIM};
pub use types::{EdgeKind, FieldType, OpKind, NUM_OP_KINDS};

use crate::ir::tir::TIROp;

/// Graph representation of TIR operations.
#[derive(Debug, Clone)]
pub struct TirGraph {
    pub nodes: Vec<TirNode>,
    pub edges: Vec<(usize, usize, EdgeKind)>,
}

impl TirGraph {
    /// Build a TirGraph from a flat sequence of TIR operations.
    ///
    /// Flattens structural ops (IfElse bodies, Loop bodies) into
    /// a single node list, adding appropriate control flow edges.
    pub fn from_tir_ops(ops: &[TIROp]) -> Self {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // Flatten ops into nodes, recursing into structural bodies
        builder::flatten_ops(ops, &mut nodes, &mut edges);

        // Extract DataDep edges via abstract stack simulation
        builder::extract_data_deps(&nodes, &mut edges);

        // Extract MemOrder edges (conservative pairwise ordering)
        builder::extract_mem_order(&nodes, &mut edges);

        TirGraph { nodes, edges }
    }

    /// Number of nodes.
    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Number of edges.
    pub fn num_edges(&self) -> usize {
        self.edges.len()
    }

    /// Count edges of a specific kind.
    pub fn count_edges(&self, kind: EdgeKind) -> usize {
        self.edges.iter().filter(|(_, _, k)| *k == kind).count()
    }
}
