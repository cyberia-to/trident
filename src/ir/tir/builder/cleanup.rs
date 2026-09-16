//! Remove dead locals while preserving a multi-word return value.
use super::TIRBuilder;
use crate::tir::TIROp;

impl TIRBuilder {
    pub(crate) fn emit_multi_ret_cleanup(&mut self, ret_width: u32, dead: u32) {
        if dead == 0 {
            return;
        }
        if ret_width == 0 {
            self.emit_pop(dead);
            return;
        }
        // Each removal rotates the surviving result right by one word.
        for _ in 0..dead {
            self.ops.extend([TIROp::Swap(ret_width), TIROp::Pop(1)]);
        }
        let rotation = dead % ret_width;
        if rotation != 0 {
            // Undo that rotation with three reversals in O(ret_width) swaps.
            // Indices are bottom-first within the preserved result suffix.
            self.reverse_result_range(ret_width, 0, rotation);
            self.reverse_result_range(ret_width, rotation, ret_width);
            self.reverse_result_range(ret_width, 0, ret_width);
        }
    }

    fn reverse_result_range(&mut self, width: u32, start: u32, end: u32) {
        for offset in 0..(end - start) / 2 {
            let left = width - 1 - (start + offset);
            let right = width - end + offset;
            if right == 0 {
                self.ops.push(TIROp::Swap(left));
            } else {
                self.ops
                    .extend([TIROp::Swap(left), TIROp::Swap(right), TIROp::Swap(left)]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_preserves_prefix_and_result_without_memory_effects() {
        for width in 0..=64u32 {
            for dead in 0..=128u32 {
                let mut builder = TIRBuilder::new(crate::target::TerrainConfig::nox());
                builder.emit_multi_ret_cleanup(width, dead);
                let mut stack: Vec<_> = (0..3 + dead + width).collect();
                let expected: Vec<_> = stack[..3]
                    .iter()
                    .chain(&stack[(3 + dead) as usize..])
                    .copied()
                    .collect();
                for op in builder.ops {
                    match op {
                        TIROp::Swap(depth) => {
                            let top = stack.len() - 1;
                            stack.swap(top, top - depth as usize);
                        }
                        TIROp::Pop(count) => stack.truncate(stack.len() - count as usize),
                        other => panic!("cleanup must use only stack operations: {other:?}"),
                    }
                }
                assert_eq!(stack, expected, "width={width}, dead={dead}");
            }
        }
    }
}
