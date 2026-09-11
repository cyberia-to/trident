//! Lower deep abstract stack accesses through compiler-allocated scratch RAM.
use super::TIRBuilder;
use crate::tir::TIROp;

impl TIRBuilder {
    pub(super) fn legalize_stack_ops(&mut self, ops: Vec<TIROp>) -> Vec<TIROp> {
        let window = self.target_config.stack_depth;
        if window == 0 {
            return ops;
        }
        let mut out = Vec::new();
        for op in ops {
            match op {
                TIROp::Dup(depth) | TIROp::Swap(depth) if depth >= window => {
                    let duplicate = matches!(op, TIROp::Dup(_));
                    let scratch = self.stack.alloc_scratch(depth + 1);
                    // Save top first. write_mem consumes address before value.
                    for i in 0..=depth {
                        out.extend([
                            TIROp::Push(scratch + u64::from(i)),
                            TIROp::WriteMem(1),
                            TIROp::Pop(1),
                        ]);
                    }
                    for i in (0..=depth).rev() {
                        let source = if duplicate {
                            i
                        } else if i == 0 {
                            depth
                        } else if i == depth {
                            0
                        } else {
                            i
                        };
                        out.extend([
                            TIROp::Push(scratch + u64::from(source)),
                            TIROp::ReadMem(1),
                            TIROp::Pop(1),
                        ]);
                    }
                    if duplicate {
                        out.extend([
                            TIROp::Push(scratch + u64::from(depth)),
                            TIROp::ReadMem(1),
                            TIROp::Pop(1),
                        ]);
                    }
                }
                TIROp::IfElse {
                    then_body,
                    else_body,
                } => out.push(TIROp::IfElse {
                    then_body: self.legalize_stack_ops(then_body),
                    else_body: self.legalize_stack_ops(else_body),
                }),
                TIROp::IfOnly { then_body } => out.push(TIROp::IfOnly {
                    then_body: self.legalize_stack_ops(then_body),
                }),
                TIROp::Loop { label, body } => out.push(TIROp::Loop {
                    label,
                    body: self.legalize_stack_ops(body),
                }),
                TIROp::ProofBlock { program_hash, body } => out.push(TIROp::ProofBlock {
                    program_hash,
                    body: self.legalize_stack_ops(body),
                }),
                op => out.push(op),
            }
        }
        out
    }
}
