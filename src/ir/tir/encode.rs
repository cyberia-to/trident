// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! TIR block encoding for neural optimizer input.
//!
//! Encodes TIR basic blocks as fixed-size tensors. Each node = 4 u64 words.
//! Max 32 nodes per block. Plus 16-element stack context vector.
//! Total: 144 u64 values per block.

use super::TIROp;

/// Maximum nodes per encoded block.
pub const MAX_NODES: usize = 32;
/// Words per node encoding.
pub const WORDS_PER_NODE: usize = 4;
/// Stack context elements.
pub const CONTEXT_SIZE: usize = 16;
/// Total input dimension: MAX_NODES * WORDS_PER_NODE + CONTEXT_SIZE.
pub const INPUT_DIM: usize = MAX_NODES * WORDS_PER_NODE + CONTEXT_SIZE;

/// Encoded TIR basic block for neural optimizer input.
#[derive(Clone, Debug)]
pub struct TIRBlock {
    /// 32 nodes * 4 words = 128 u64 values, zero-padded.
    pub nodes: [u64; MAX_NODES * WORDS_PER_NODE],
    /// Stack state context at block entry (16 elements).
    pub context: [u64; CONTEXT_SIZE],
    /// Number of actual nodes (before padding).
    pub node_count: usize,
    /// Source location: function name.
    pub fn_name: String,
    /// Source location: start index in the original TIR op sequence.
    pub start_idx: usize,
    /// Source location: end index (exclusive).
    pub end_idx: usize,
}

impl TIRBlock {
    /// Flattened input tensor (144 elements) for the neural model.
    pub fn as_input(&self) -> Vec<u64> {
        let mut v = Vec::with_capacity(INPUT_DIM);
        v.extend_from_slice(&self.nodes);
        v.extend_from_slice(&self.context);
        v
    }

    /// Block identifier for display (e.g., "main:0..14").
    pub fn block_id(&self) -> String {
        format!("{}:{}..{}", self.fn_name, self.start_idx, self.end_idx)
    }
}

/// Opcode mapping: TIROp variant -> 0..53 (6 bits).
fn opcode(op: &TIROp) -> u8 {
    match op {
        // Tier 0 — Structure (0..10)
        TIROp::Call(_) => 0,
        TIROp::Return => 1,
        TIROp::Halt => 2,
        TIROp::IfElse { .. } => 3,
        TIROp::IfOnly { .. } => 4,
        TIROp::Loop { .. } => 5,
        TIROp::FnStart(_) => 6,
        TIROp::FnEnd => 7,
        TIROp::Entry(_) => 8,
        TIROp::Comment(_) => 9,
        TIROp::Asm { .. } => 10,
        // Tier 1 — Universal (11..41)
        TIROp::Push(_) => 11,
        TIROp::Pop(_) => 12,
        TIROp::Dup(_) => 13,
        TIROp::Swap(_) => 14,
        TIROp::Add => 15,
        TIROp::Sub => 16,
        TIROp::Mul => 17,
        TIROp::Neg => 18,
        TIROp::Invert => 19,
        TIROp::Eq => 20,
        TIROp::Lt => 21,
        TIROp::And => 22,
        TIROp::Or => 23,
        TIROp::Xor => 24,
        TIROp::PopCount => 25,
        TIROp::Split => 26,
        TIROp::DivMod => 27,
        TIROp::Shl => 28,
        TIROp::Shr => 29,
        TIROp::Log2 => 30,
        TIROp::Pow => 31,
        TIROp::ReadIo(_) => 32,
        TIROp::WriteIo(_) => 33,
        TIROp::ReadMem(_) => 34,
        TIROp::WriteMem(_) => 35,
        TIROp::Assert(_) => 36,
        TIROp::Hash { .. } => 37,
        TIROp::Reveal { .. } => 38,
        TIROp::Seal { .. } => 39,
        TIROp::RamRead { .. } => 40,
        TIROp::RamWrite { .. } => 41,
        // Tier 2 — Provable (42..48)
        TIROp::Hint(_) => 42,
        TIROp::SpongeInit => 43,
        TIROp::SpongeAbsorb => 44,
        TIROp::SpongeSqueeze => 45,
        TIROp::SpongeLoad => 46,
        TIROp::MerkleStep => 47,
        TIROp::MerkleLoad => 48,
        // Tier 3 — Recursion (49..53)
        TIROp::ExtMul => 49,
        TIROp::ExtInvert => 50,
        TIROp::FoldExt => 51,
        TIROp::FoldBase => 52,
        TIROp::ProofBlock { .. } => 53,
    }
}

/// Extract the immediate argument from a TIROp (if any).
fn immediate(op: &TIROp) -> u64 {
    match op {
        TIROp::Push(v) => *v,
        TIROp::Pop(n) | TIROp::Dup(n) | TIROp::Swap(n) => *n as u64,
        TIROp::ReadIo(n) | TIROp::WriteIo(n) => *n as u64,
        TIROp::ReadMem(n) | TIROp::WriteMem(n) => *n as u64,
        TIROp::Assert(n) => *n as u64,
        TIROp::Hint(n) => *n as u64,
        TIROp::Hash { width } => *width as u64,
        TIROp::RamRead { width } | TIROp::RamWrite { width } => *width as u64,
        TIROp::Asm { effect, .. } => *effect as u64,
        _ => 0,
    }
}

/// Whether a TIROp is a control flow boundary (block terminator).
fn is_block_boundary(op: &TIROp) -> bool {
    matches!(
        op,
        TIROp::Call(_)
            | TIROp::Return
            | TIROp::Halt
            | TIROp::IfElse { .. }
            | TIROp::IfOnly { .. }
            | TIROp::Loop { .. }
            | TIROp::FnStart(_)
            | TIROp::FnEnd
            | TIROp::Entry(_)
    )
}

/// Encode a single node as 4 u64 words.
///
/// Word 0: opcode (6 bits) | immediate (58 bits packed)
/// Word 1: node index (position in block)
/// Word 2: immediate value (full 64 bits for Push)
/// Word 3: reserved (0)
fn encode_node(op: &TIROp, index: usize) -> [u64; WORDS_PER_NODE] {
    let opc = opcode(op) as u64;
    let imm = immediate(op);
    [
        opc,          // word 0: opcode
        index as u64, // word 1: position
        imm,          // word 2: immediate
        0,            // word 3: reserved
    ]
}

/// Split a TIR op sequence into basic blocks at control flow boundaries.
///
/// Each block is a maximal straight-line segment of <= MAX_NODES ops.
/// Structural ops (FnStart, FnEnd, Entry) start new blocks but are
/// not included in the block content.
pub fn encode_blocks(ops: &[TIROp]) -> Vec<TIRBlock> {
    let mut blocks = Vec::new();
    let mut current_fn = String::new();
    let mut block_ops: Vec<(usize, &TIROp)> = Vec::new();
    let mut block_start = 0;

    for (i, op) in ops.iter().enumerate() {
        // Track current function name
        if let TIROp::FnStart(name) = op {
            // Flush pending block
            if !block_ops.is_empty() {
                blocks.push(build_block(&block_ops, &current_fn, block_start));
                block_ops.clear();
            }
            current_fn = name.clone();
            block_start = i + 1;
            continue;
        }

        // Skip structural markers
        if matches!(op, TIROp::FnEnd | TIROp::Entry(_) | TIROp::Comment(_)) {
            continue;
        }

        // Control flow boundaries flush the current block
        if is_block_boundary(op) {
            if !block_ops.is_empty() {
                blocks.push(build_block(&block_ops, &current_fn, block_start));
                block_ops.clear();
            }
            block_start = i + 1;
            continue;
        }

        block_ops.push((i, op));

        // Split at MAX_NODES
        if block_ops.len() >= MAX_NODES {
            blocks.push(build_block(&block_ops, &current_fn, block_start));
            block_start = i + 1;
            block_ops.clear();
        }
    }

    // Flush remaining
    if !block_ops.is_empty() {
        blocks.push(build_block(&block_ops, &current_fn, block_start));
    }

    blocks
}

fn build_block(ops: &[(usize, &TIROp)], fn_name: &str, start_idx: usize) -> TIRBlock {
    let mut nodes = [0u64; MAX_NODES * WORDS_PER_NODE];
    let node_count = ops.len().min(MAX_NODES);
    let end_idx = ops.last().map(|(i, _)| i + 1).unwrap_or(start_idx);

    for (local_idx, (_global_idx, op)) in ops.iter().enumerate().take(MAX_NODES) {
        let encoded = encode_node(op, local_idx);
        let base = local_idx * WORDS_PER_NODE;
        nodes[base..base + WORDS_PER_NODE].copy_from_slice(&encoded);
    }

    TIRBlock {
        nodes,
        context: [0; CONTEXT_SIZE],
        node_count,
        fn_name: fn_name.to_string(),
        start_idx,
        end_idx,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcode_coverage() {
        // All 54 variants should map to distinct opcodes 0..53
        let ops = vec![
            TIROp::Call("f".into()),
            TIROp::Return,
            TIROp::Halt,
            TIROp::IfElse {
                then_body: vec![],
                else_body: vec![],
            },
            TIROp::IfOnly { then_body: vec![] },
            TIROp::Loop {
                label: "l".into(),
                body: vec![],
            },
            TIROp::FnStart("f".into()),
            TIROp::FnEnd,
            TIROp::Entry("m".into()),
            TIROp::Comment("c".into()),
            TIROp::Asm {
                lines: vec![],
                effect: 0,
            },
            TIROp::Push(0),
            TIROp::Pop(1),
            TIROp::Dup(0),
            TIROp::Swap(1),
            TIROp::Add,
            TIROp::Sub,
            TIROp::Mul,
            TIROp::Neg,
            TIROp::Invert,
            TIROp::Eq,
            TIROp::Lt,
            TIROp::And,
            TIROp::Or,
            TIROp::Xor,
            TIROp::PopCount,
            TIROp::Split,
            TIROp::DivMod,
            TIROp::Shl,
            TIROp::Shr,
            TIROp::Log2,
            TIROp::Pow,
            TIROp::ReadIo(1),
            TIROp::WriteIo(1),
            TIROp::ReadMem(1),
            TIROp::WriteMem(1),
            TIROp::Assert(1),
            TIROp::Hash { width: 0 },
            TIROp::Reveal {
                name: "e".into(),
                tag: 0,
                field_count: 1,
            },
            TIROp::Seal {
                name: "e".into(),
                tag: 0,
                field_count: 1,
            },
            TIROp::RamRead { width: 1 },
            TIROp::RamWrite { width: 1 },
            TIROp::Hint(1),
            TIROp::SpongeInit,
            TIROp::SpongeAbsorb,
            TIROp::SpongeSqueeze,
            TIROp::SpongeLoad,
            TIROp::MerkleStep,
            TIROp::MerkleLoad,
            TIROp::ExtMul,
            TIROp::ExtInvert,
            TIROp::FoldExt,
            TIROp::FoldBase,
            TIROp::ProofBlock {
                program_hash: "h".into(),
                body: vec![],
            },
        ];
        let mut seen = std::collections::HashSet::new();
        for op in &ops {
            let code = opcode(op);
            assert!(code <= 53, "opcode {} out of range for {:?}", code, op);
            seen.insert(code);
        }
        assert_eq!(
            seen.len(),
            54,
            "expected 54 distinct opcodes, got {}",
            seen.len()
        );
    }

    #[test]
    fn encode_simple_block() {
        let ops = vec![
            TIROp::FnStart("main".into()),
            TIROp::Push(42),
            TIROp::Push(10),
            TIROp::Add,
            TIROp::WriteIo(1),
            TIROp::Return,
        ];
        let blocks = encode_blocks(&ops);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].node_count, 4); // Push, Push, Add, WriteIo
        assert_eq!(blocks[0].fn_name, "main");
        // First node is Push(42)
        assert_eq!(blocks[0].nodes[0], 11); // opcode for Push
        assert_eq!(blocks[0].nodes[2], 42); // immediate
    }

    #[test]
    fn block_split_at_control_flow() {
        let ops = vec![
            TIROp::FnStart("main".into()),
            TIROp::Push(1),
            TIROp::Push(2),
            TIROp::Call("helper".into()), // boundary
            TIROp::Push(3),
            TIROp::Add,
            TIROp::Return, // boundary
        ];
        let blocks = encode_blocks(&ops);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].node_count, 2); // Push(1), Push(2)
        assert_eq!(blocks[1].node_count, 2); // Push(3), Add
    }

    #[test]
    fn block_split_at_max_nodes() {
        let mut ops = vec![TIROp::FnStart("big".into())];
        for i in 0..40 {
            ops.push(TIROp::Push(i));
        }
        let blocks = encode_blocks(&ops);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].node_count, 32);
        assert_eq!(blocks[1].node_count, 8);
    }

    #[test]
    fn empty_ops() {
        let blocks = encode_blocks(&[]);
        assert!(blocks.is_empty());
    }

    #[test]
    fn input_dimension() {
        let ops = vec![
            TIROp::FnStart("f".into()),
            TIROp::Push(1),
            TIROp::Push(2),
            TIROp::Add,
        ];
        let blocks = encode_blocks(&ops);
        let input = blocks[0].as_input();
        assert_eq!(input.len(), INPUT_DIM);
    }

    #[test]
    fn block_id_format() {
        let ops = vec![TIROp::FnStart("main".into()), TIROp::Push(1), TIROp::Add];
        let blocks = encode_blocks(&ops);
        assert!(blocks[0].block_id().starts_with("main:"));
    }
}
