//! Remove dead locals while preserving a multi-word return value.
use super::TIRBuilder;
use crate::tir::TIROp;

impl TIRBuilder {
    pub(crate) fn emit_multi_ret_cleanup(&mut self, ret_width: u32, dead: u32) {
        if dead == 0 {
            return;
        }
        if dead % ret_width == 0 {
            for _ in 0..dead {
                self.ops.extend([TIROp::Swap(ret_width), TIROp::Pop(1)]);
            }
            return;
        }
        let scratch = self.stack.alloc_scratch(ret_width);
        // write_mem takes the address on top and the value immediately below.
        for i in 0..ret_width {
            self.ops.extend([
                TIROp::Push(scratch + u64::from(i)),
                TIROp::WriteMem(1),
                TIROp::Pop(1),
            ]);
        }
        self.emit_pop(dead);
        for i in (0..ret_width).rev() {
            self.ops.extend([
                TIROp::Push(scratch + u64::from(i)),
                TIROp::ReadMem(1),
                TIROp::Pop(1),
            ]);
        }
    }
}
