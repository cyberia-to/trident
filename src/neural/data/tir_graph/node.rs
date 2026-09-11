// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! TirNode struct and feature vector encoding.

use super::types::{FieldType, OpKind, NUM_OP_KINDS};

/// Node feature vector dimensions:
/// - op_onehot: 54 (NUM_OP_KINDS)
/// - field_type_onehot: 3 (BFE, XFE, Unknown)
/// - has_immediate: 1
/// - immediate_normalized: 1
/// Total: 59
pub const NODE_FEATURE_DIM: usize = NUM_OP_KINDS + 3 + 1 + 1;

/// A node in the TIR graph.
#[derive(Debug, Clone)]
pub struct TirNode {
    pub op: OpKind,
    pub field_type: FieldType,
    pub immediate: Option<u64>,
}

impl TirNode {
    /// Encode this node as a 59-dimensional feature vector.
    pub fn feature_vector(&self) -> [f32; NODE_FEATURE_DIM] {
        let mut v = [0.0f32; NODE_FEATURE_DIM];

        // One-hot op kind (54 dims)
        v[self.op as usize] = 1.0;

        // One-hot field type (3 dims, offset 54)
        let ft_offset = NUM_OP_KINDS;
        match self.field_type {
            FieldType::BFE => v[ft_offset] = 1.0,
            FieldType::XFE => v[ft_offset + 1] = 1.0,
            FieldType::Unknown => v[ft_offset + 2] = 1.0,
        }

        // Has immediate (1 dim, offset 57)
        if self.immediate.is_some() {
            v[ft_offset + 3] = 1.0;
        }

        // Normalized immediate (1 dim, offset 58)
        // Normalize to [0, 1] using log1p for large values
        if let Some(imm) = self.immediate {
            v[ft_offset + 4] = (imm as f64 + 1.0).ln() as f32 / 44.4; // ln(2^64) ~ 44.4
        }

        v
    }
}
