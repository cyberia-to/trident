// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! MIR → TIR translator.
//!
//! Reads a `MirCrate` (from rsc's `--emit=mir-rs`) and produces `Vec<TIROp>`.
//! Uses the structurizer for control flow and the type module for widths.

use std::collections::BTreeMap;

use mir_format::*;

use crate::ir::tir::TIROp;

use super::structurize::{self, Region};
use super::types;

/// Translate a serialized MIR crate into TIR ops.
pub fn mir_to_tir(krate: &MirCrate) -> Vec<TIROp> {
    let struct_map = types::struct_map(&krate.structs);
    let mut ops = Vec::new();

    for func in &krate.functions {
        ops.extend(translate_function(func, &struct_map));
    }

    // Entry point: call main if it exists.
    if krate.functions.iter().any(|f| f.name == "main") {
        ops.push(TIROp::Entry("main".into()));
    }

    ops
}

/// Load a .mir.json file and translate to TIR.
pub fn mir_file_to_tir(path: &str) -> Result<Vec<TIROp>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {}", path, e))?;
    let krate =
        MirCrate::from_json(&bytes).map_err(|e| format!("parse {}: {}", path, e))?;
    Ok(mir_to_tir(&krate))
}

// ── Function translation ───────────────────────────────────

fn translate_function(
    func: &MirFunction,
    structs: &BTreeMap<String, &MirStruct>,
) -> Vec<TIROp> {
    let mut ctx = FnContext::new(func, structs);
    let mut ops = Vec::new();

    ops.push(TIROp::FnStart(func.name.clone()));

    // Structurize the CFG into nested regions.
    let region = structurize::structurize(&func.blocks);

    // Emit the structured body.
    emit_region(&region, func, &mut ctx, &mut ops);

    ops.push(TIROp::Return);
    ops.push(TIROp::FnEnd);

    ops
}

// ── Emission context ───────────────────────────────────────

/// Tracks local variable positions on the virtual stack.
struct FnContext<'a> {
    /// local_index → (stack_slot, width)
    locals: BTreeMap<u32, (u32, u32)>,
    structs: &'a BTreeMap<String, &'a MirStruct>,
    /// Current stack depth (number of field elements).
    stack_depth: u32,
}

impl<'a> FnContext<'a> {
    fn new(func: &MirFunction, structs: &'a BTreeMap<String, &'a MirStruct>) -> Self {
        let mut locals = BTreeMap::new();
        let mut offset = 0u32;

        // Allocate stack slots for all locals.
        // Local 0 = return place, 1..=arg_count = params, rest = temps.
        for local in &func.locals {
            let width = types::field_width(&local.ty, structs);
            locals.insert(local.index, (offset, width));
            offset += width;
        }

        FnContext {
            locals,
            structs,
            stack_depth: offset,
        }
    }

    fn local_width(&self, index: u32) -> u32 {
        self.locals.get(&index).map(|&(_, w)| w).unwrap_or(1)
    }

    fn type_width(&self, ty: &MirType) -> u32 {
        types::field_width(ty, self.structs)
    }
}

// ── Region emission ────────────────────────────────────────

fn emit_region(
    region: &Region,
    func: &MirFunction,
    ctx: &mut FnContext,
    ops: &mut Vec<TIROp>,
) {
    match region {
        Region::Linear(block_indices) => {
            for &idx in block_indices {
                if let Some(block) = func.blocks.iter().find(|b| b.index == idx) {
                    emit_block_statements(block, ctx, ops);
                }
            }
        }

        Region::Seq(regions) => {
            for r in regions {
                emit_region(r, func, ctx, ops);
            }
        }

        Region::IfElse {
            cond_block,
            then_region,
            else_region,
            ..
        } => {
            // The condition block's statements have already been emitted
            // (Linear region before the IfElse). The SwitchInt condition
            // value is on the stack.
            if let Some(block) = func.blocks.iter().find(|b| b.index == *cond_block) {
                if let MirTerminator::SwitchInt { discriminant, .. } = &block.terminator {
                    emit_operand(discriminant, ctx, ops);
                }
            }

            let mut then_ops = Vec::new();
            emit_region(then_region, func, ctx, &mut then_ops);

            let mut else_ops = Vec::new();
            emit_region(else_region, func, ctx, &mut else_ops);

            ops.push(TIROp::IfElse {
                then_body: then_ops,
                else_body: else_ops,
            });
        }

        Region::IfOnly {
            cond_block,
            then_region,
            ..
        } => {
            if let Some(block) = func.blocks.iter().find(|b| b.index == *cond_block) {
                if let MirTerminator::SwitchInt { discriminant, .. } = &block.terminator {
                    emit_operand(discriminant, ctx, ops);
                }
            }

            let mut then_ops = Vec::new();
            emit_region(then_region, func, ctx, &mut then_ops);

            ops.push(TIROp::IfOnly {
                then_body: then_ops,
            });
        }

        Region::Loop {
            header, body, ..
        } => {
            let label = format!("loop_{}", header);
            let mut body_ops = Vec::new();
            emit_region(body, func, ctx, &mut body_ops);

            ops.push(TIROp::Loop {
                label,
                body: body_ops,
            });
        }
    }
}

// ── Block statement emission ───────────────────────────────

fn emit_block_statements(block: &MirBlock, ctx: &mut FnContext, ops: &mut Vec<TIROp>) {
    for stmt in &block.statements {
        if let MirStatement::Assign { place, rvalue } = stmt {
            emit_rvalue(rvalue, ctx, ops);
            emit_store(place, ctx, ops);
        }
    }

    // Emit terminator side effects (calls, asserts).
    match &block.terminator {
        MirTerminator::Call {
            func,
            args,
            destination,
            ..
        } => {
            for arg in args {
                emit_operand(arg, ctx, ops);
            }
            ops.push(TIROp::Call(func.clone()));
            emit_store(destination, ctx, ops);
        }
        MirTerminator::Assert {
            cond, expected, ..
        } => {
            emit_operand(cond, ctx, ops);
            if !expected {
                // Negate: assert false means "assert this is 0"
                ops.push(TIROp::Push(1));
                ops.push(TIROp::Xor);
            }
            ops.push(TIROp::Assert(1));
        }
        _ => {} // Goto, Return, SwitchInt handled by structurizer
    }
}

// ── Rvalue emission ────────────────────────────────────────

fn emit_rvalue(rvalue: &MirRvalue, ctx: &mut FnContext, ops: &mut Vec<TIROp>) {
    match rvalue {
        MirRvalue::Use(operand) => {
            emit_operand(operand, ctx, ops);
        }

        MirRvalue::BinaryOp(op, lhs, rhs) => {
            emit_operand(lhs, ctx, ops);
            emit_operand(rhs, ctx, ops);
            ops.push(binop_to_tir(op));
        }

        MirRvalue::CheckedBinaryOp(op, lhs, rhs) => {
            // Rs edition: checked ops become unchecked (overflow is lint-enforced).
            emit_operand(lhs, ctx, ops);
            emit_operand(rhs, ctx, ops);
            ops.push(binop_to_tir(op));
        }

        MirRvalue::UnaryOp(op, operand) => {
            emit_operand(operand, ctx, ops);
            match op {
                MirUnaryOp::Neg => ops.push(TIROp::Neg),
                MirUnaryOp::Not => {
                    ops.push(TIROp::Push(1));
                    ops.push(TIROp::Xor);
                }
            }
        }

        MirRvalue::Aggregate(_, operands) => {
            // Push all fields in order.
            for op in operands {
                emit_operand(op, ctx, ops);
            }
        }

        MirRvalue::Cast(_, operand, _target_ty) => {
            // For field elements, most integer casts are identity.
            emit_operand(operand, ctx, ops);
        }

        MirRvalue::Ref(place) => {
            // Rs edition refs are stack values. Load the referent.
            emit_load(place, ctx, ops);
        }

        MirRvalue::Repeat(operand, count) => {
            for _ in 0..*count {
                emit_operand(operand, ctx, ops);
            }
        }

        MirRvalue::Len(_place) => {
            // Array lengths are compile-time constants in Rs edition.
            ops.push(TIROp::Push(0));
            ops.push(TIROp::Comment("len: should be resolved at compile time".into()));
        }
    }
}

// ── Operand emission ───────────────────────────────────────

fn emit_operand(operand: &MirOperand, ctx: &mut FnContext, ops: &mut Vec<TIROp>) {
    match operand {
        MirOperand::Copy(place) | MirOperand::Move(place) => {
            emit_load(place, ctx, ops);
        }
        MirOperand::Constant(val) => match val {
            MirConstValue::Scalar(n) => {
                if *n <= u64::MAX as u128 {
                    ops.push(TIROp::Push(*n as u64));
                } else {
                    // u128: push as two limbs (lo, hi).
                    let lo = *n as u64;
                    let hi = (*n >> 64) as u64;
                    ops.push(TIROp::Push(lo));
                    ops.push(TIROp::Push(hi));
                }
            }
            MirConstValue::Bool(b) => {
                ops.push(TIROp::Push(if *b { 1 } else { 0 }));
            }
            MirConstValue::Unit => {
                // No value to push.
            }
        },
    }
}

// ── Place load/store ───────────────────────────────────────

fn emit_load(place: &MirPlace, ctx: &FnContext, ops: &mut Vec<TIROp>) {
    match place {
        MirPlace::Local(idx) => {
            let width = ctx.local_width(*idx);
            for i in 0..width {
                // Each field element of the local is at a separate stack position.
                // Use RAM-based addressing for simplicity.
                ops.push(TIROp::Push((*idx as u64) * 8 + i as u64));
                ops.push(TIROp::RamRead { width: 1 });
                ops.push(TIROp::Pop(1)); // pop address left by RamRead
            }
        }
        MirPlace::Projection { base, elem } => {
            emit_load(base, ctx, ops);
            match elem {
                MirProjection::Field(field_idx) => {
                    // For now, field access into an already-loaded struct.
                    // The struct is on the stack as concatenated fields.
                    // We need to extract the right field.
                    ops.push(TIROp::Comment(format!("field {}", field_idx)));
                }
                MirProjection::Index(local_idx) => {
                    ops.push(TIROp::Comment(format!("index from local {}", local_idx)));
                }
                MirProjection::ConstantIndex { offset, .. } => {
                    ops.push(TIROp::Comment(format!("const_index {}", offset)));
                }
                MirProjection::Downcast(variant) => {
                    ops.push(TIROp::Comment(format!("downcast variant {}", variant)));
                }
            }
        }
    }
}

fn emit_store(place: &MirPlace, ctx: &FnContext, ops: &mut Vec<TIROp>) {
    match place {
        MirPlace::Local(idx) => {
            let width = ctx.local_width(*idx);
            // Store top-of-stack to RAM slot for this local.
            for i in (0..width).rev() {
                ops.push(TIROp::Push((*idx as u64) * 8 + i as u64));
                ops.push(TIROp::RamWrite { width: 1 });
                ops.push(TIROp::Pop(1)); // pop address left by RamWrite
            }
        }
        MirPlace::Projection { base, elem } => {
            // Projected store — compute target address and write.
            ops.push(TIROp::Comment(format!("store to projection {:?}", elem)));
            emit_store(base, ctx, ops);
        }
    }
}

// ── Binary op mapping ──────────────────────────────────────

fn binop_to_tir(op: &MirBinOp) -> TIROp {
    match op {
        MirBinOp::Add => TIROp::Add,
        MirBinOp::Sub => TIROp::Sub,
        MirBinOp::Mul => TIROp::Mul,
        MirBinOp::Div => TIROp::DivMod, // caller should Pop remainder
        MirBinOp::Rem => TIROp::DivMod, // caller should Swap+Pop quotient
        MirBinOp::BitAnd => TIROp::And,
        MirBinOp::BitOr => TIROp::Or,
        MirBinOp::BitXor => TIROp::Xor,
        MirBinOp::Shl => TIROp::Shl,
        MirBinOp::Shr => TIROp::Shr,
        MirBinOp::Eq => TIROp::Eq,
        MirBinOp::Ne => TIROp::Eq, // caller should Xor with 1
        MirBinOp::Lt => TIROp::Lt,
        MirBinOp::Le => TIROp::Lt, // a <= b is !(b < a)
        MirBinOp::Gt => TIROp::Lt, // a > b is b < a (swap operands)
        MirBinOp::Ge => TIROp::Lt, // a >= b is !(a < b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_crate() -> MirCrate {
        MirCrate {
            name: "test".into(),
            functions: vec![MirFunction {
                name: "add".into(),
                params: vec![
                    MirLocal { index: 1, name: Some("a".into()), ty: MirType::U32 },
                    MirLocal { index: 2, name: Some("b".into()), ty: MirType::U32 },
                ],
                return_ty: MirType::U32,
                locals: vec![
                    MirLocal { index: 0, name: None, ty: MirType::U32 },
                    MirLocal { index: 1, name: Some("a".into()), ty: MirType::U32 },
                    MirLocal { index: 2, name: Some("b".into()), ty: MirType::U32 },
                ],
                blocks: vec![MirBlock {
                    index: 0,
                    statements: vec![MirStatement::Assign {
                        place: MirPlace::Local(0),
                        rvalue: MirRvalue::BinaryOp(
                            MirBinOp::Add,
                            MirOperand::Copy(MirPlace::Local(1)),
                            MirOperand::Copy(MirPlace::Local(2)),
                        ),
                    }],
                    terminator: MirTerminator::Return,
                }],
            }],
            structs: vec![],
            constants: vec![],
        }
    }

    #[test]
    fn translate_add_function() {
        let tir = mir_to_tir(&test_crate());
        assert!(!tir.is_empty());

        // Should contain FnStart("add"), some ops, Return, FnEnd.
        let fn_starts: Vec<_> = tir
            .iter()
            .filter(|op| matches!(op, TIROp::FnStart(name) if name == "add"))
            .collect();
        assert_eq!(fn_starts.len(), 1);

        // Should contain an Add op.
        assert!(tir.iter().any(|op| matches!(op, TIROp::Add)));
    }

    #[test]
    fn translate_constants() {
        let krate = MirCrate {
            name: "const_test".into(),
            functions: vec![MirFunction {
                name: "answer".into(),
                params: vec![],
                return_ty: MirType::U32,
                locals: vec![
                    MirLocal { index: 0, name: None, ty: MirType::U32 },
                ],
                blocks: vec![MirBlock {
                    index: 0,
                    statements: vec![MirStatement::Assign {
                        place: MirPlace::Local(0),
                        rvalue: MirRvalue::Use(MirOperand::Constant(MirConstValue::Scalar(42))),
                    }],
                    terminator: MirTerminator::Return,
                }],
            }],
            structs: vec![],
            constants: vec![],
        };

        let tir = mir_to_tir(&krate);
        assert!(tir.iter().any(|op| matches!(op, TIROp::Push(42))));
    }
}
