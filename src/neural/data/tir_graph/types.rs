//! Type definitions for the TIR graph: EdgeKind, FieldType, OpKind.

use crate::ir::tir::TIROp;

/// Edge types in the TIR graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    /// Data dependency: op A produces a value consumed by op B.
    DataDep,
    /// Control flow: sequential or branch edge.
    ControlFlow,
    /// Memory ordering: conservative ordering between memory ops.
    MemOrder,
}

/// Field type annotation for a TIR node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    /// Base field element (Goldilocks).
    BFE,
    /// Extension field element (cubic extension).
    XFE,
    /// Unknown or not applicable.
    Unknown,
}

/// Opcode kind — mirrors TIROp variants without payloads.
/// Used for one-hot encoding in the GNN feature vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum OpKind {
    // Tier 0 — Structure (11)
    Call = 0,
    Return = 1,
    Halt = 2,
    IfElse = 3,
    IfOnly = 4,
    Loop = 5,
    FnStart = 6,
    FnEnd = 7,
    Entry = 8,
    Comment = 9,
    Asm = 10,
    // Tier 1 — Universal (31)
    Push = 11,
    Pop = 12,
    Dup = 13,
    Swap = 14,
    Add = 15,
    Sub = 16,
    Mul = 17,
    Neg = 18,
    Invert = 19,
    Eq = 20,
    Lt = 21,
    And = 22,
    Or = 23,
    Xor = 24,
    PopCount = 25,
    Split = 26,
    DivMod = 27,
    Shl = 28,
    Shr = 29,
    Log2 = 30,
    Pow = 31,
    ReadIo = 32,
    WriteIo = 33,
    ReadMem = 34,
    WriteMem = 35,
    Assert = 36,
    Hash = 37,
    Reveal = 38,
    Seal = 39,
    RamRead = 40,
    RamWrite = 41,
    // Tier 2 — Provable (7)
    Hint = 42,
    SpongeInit = 43,
    SpongeAbsorb = 44,
    SpongeSqueeze = 45,
    SpongeLoad = 46,
    MerkleStep = 47,
    MerkleLoad = 48,
    // Tier 3 — Recursion (5)
    ExtMul = 49,
    ExtInvert = 50,
    FoldExt = 51,
    FoldBase = 52,
    ProofBlock = 53,
}

pub const NUM_OP_KINDS: usize = 54;

impl OpKind {
    pub fn from_tir_op(op: &TIROp) -> Self {
        match op {
            TIROp::Call(_) => OpKind::Call,
            TIROp::Return => OpKind::Return,
            TIROp::Halt => OpKind::Halt,
            TIROp::IfElse { .. } => OpKind::IfElse,
            TIROp::IfOnly { .. } => OpKind::IfOnly,
            TIROp::Loop { .. } => OpKind::Loop,
            TIROp::FnStart(_) => OpKind::FnStart,
            TIROp::FnEnd => OpKind::FnEnd,
            TIROp::Entry(_) => OpKind::Entry,
            TIROp::Comment(_) => OpKind::Comment,
            TIROp::Asm { .. } => OpKind::Asm,
            TIROp::Push(_) => OpKind::Push,
            TIROp::Pop(_) => OpKind::Pop,
            TIROp::Dup(_) => OpKind::Dup,
            TIROp::Swap(_) => OpKind::Swap,
            TIROp::Add => OpKind::Add,
            TIROp::Sub => OpKind::Sub,
            TIROp::Mul => OpKind::Mul,
            TIROp::Neg => OpKind::Neg,
            TIROp::Invert => OpKind::Invert,
            TIROp::Eq => OpKind::Eq,
            TIROp::Lt => OpKind::Lt,
            TIROp::And => OpKind::And,
            TIROp::Or => OpKind::Or,
            TIROp::Xor => OpKind::Xor,
            TIROp::PopCount => OpKind::PopCount,
            TIROp::Split => OpKind::Split,
            TIROp::DivMod => OpKind::DivMod,
            TIROp::Shl => OpKind::Shl,
            TIROp::Shr => OpKind::Shr,
            TIROp::Log2 => OpKind::Log2,
            TIROp::Pow => OpKind::Pow,
            TIROp::ReadIo(_) => OpKind::ReadIo,
            TIROp::WriteIo(_) => OpKind::WriteIo,
            TIROp::ReadMem(_) => OpKind::ReadMem,
            TIROp::WriteMem(_) => OpKind::WriteMem,
            TIROp::Assert(_) => OpKind::Assert,
            TIROp::Hash { .. } => OpKind::Hash,
            TIROp::Reveal { .. } => OpKind::Reveal,
            TIROp::Seal { .. } => OpKind::Seal,
            TIROp::RamRead { .. } => OpKind::RamRead,
            TIROp::RamWrite { .. } => OpKind::RamWrite,
            TIROp::Hint(_) => OpKind::Hint,
            TIROp::SpongeInit => OpKind::SpongeInit,
            TIROp::SpongeAbsorb => OpKind::SpongeAbsorb,
            TIROp::SpongeSqueeze => OpKind::SpongeSqueeze,
            TIROp::SpongeLoad => OpKind::SpongeLoad,
            TIROp::MerkleStep => OpKind::MerkleStep,
            TIROp::MerkleLoad => OpKind::MerkleLoad,
            TIROp::ExtMul => OpKind::ExtMul,
            TIROp::ExtInvert => OpKind::ExtInvert,
            TIROp::FoldExt => OpKind::FoldExt,
            TIROp::FoldBase => OpKind::FoldBase,
            TIROp::ProofBlock { .. } => OpKind::ProofBlock,
        }
    }
}
