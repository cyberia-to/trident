//! Assignments and branch stack cleanup.
use super::*;
use crate::span::Spanned;

impl TIRBuilder {
    /// Evaluate all tuple components before replacing any destination. The
    /// remaining RHS words stay in the model until each component is stored.
    pub(crate) fn build_tuple_assign(&mut self, names: &[Spanned<String>], value: &Expr) {
        let widths = names
            .iter()
            .map(|name| {
                let info = self.stack.find_var_depth_and_width(&name.node);
                self.flush_stack_effects();
                info.map(|(_, width)| width)
            })
            .collect::<Option<Vec<_>>>();
        self.build_expr(value);
        let rhs_width = self.stack.last().map_or(0, |v| v.width);
        let widths = widths.filter(|widths| widths.iter().sum::<u32>() == rhs_width);
        let Some(widths) = widths else {
            self.ops.push(TIROp::Comment(
                "ERROR: tuple assignment has unresolved or incompatible target widths".into(),
            ));
            self.emit_pop(rhs_width);
            self.stack.pop();
            return;
        };
        for (name, &width) in names.iter().zip(&widths).rev() {
            let depth = self.stack.access_var(&name.node);
            self.flush_stack_effects();
            self.store_top_words_into(depth, width);
            if let Some(rhs) = self.stack.last_mut() {
                rhs.width -= width;
            }
        }
        self.stack.pop();
    }

    /// Lower an assignment `place = value` on the stack machine.
    ///
    /// Whole values and projections retain their complete declared word width.
    /// Runtime indices are bounds checked before evaluating the right-hand side.
    pub(crate) fn build_assign(&mut self, place: &Place, value: &Expr) {
        match place {
            Place::Var(name) if !name.contains('.') => {
                // Reload before evaluating so a spill reload cannot cover
                // the new value. Each pop exposes the next aggregate word;
                // its corresponding destination remains at the same depth.
                self.stack.access_var(name);
                self.flush_stack_effects();
                self.build_expr(value);
                let depth = self.stack.access_var(name);
                let width = self.stack.last().map_or(0, |value| value.width);
                self.flush_stack_effects();
                self.store_top_words_into(depth, width);
                self.stack.pop();
            }
            _ => self.build_projected_store(place, value),
        }
    }

    fn store_top_words_into(&mut self, depth: u32, width: u32) {
        for _ in 0..width {
            self.ops.extend([TIROp::Swap(depth), TIROp::Pop(1)]);
        }
    }

    /// Append Pop ops to clean up locals created in an if/else branch.
    /// `post_depth` is stack_depth() after the branch body, `pre_depth` before.
    /// `keep` is the number of words to preserve on top (e.g. a tail expression value).
    pub(super) fn append_branch_cleanup(
        body: &mut Vec<TIROp>,
        post_depth: u32,
        pre_depth: u32,
        keep: u32,
    ) {
        let leftover = post_depth.saturating_sub(pre_depth + keep);
        if leftover > 0 {
            // Rotate each dead word past the preserved suffix, keeping both
            // the outer stack and the order of multi-word results intact.
            for _ in 0..leftover {
                for depth in 1..=keep {
                    body.push(TIROp::Swap(depth));
                }
                body.push(TIROp::Pop(1));
            }
        }
    }
}

#[cfg(test)]
mod cleanup_tests {
    use super::*;

    #[test]
    fn aggregate_reassignment_replaces_all_words_without_disturbing_neighbors() {
        for width in 1..=20u32 {
            for above in 0..=20u32 {
                let mut builder = TIRBuilder::new(crate::target::TerrainConfig::nox());
                builder.store_top_words_into(width + above, width);
                let mut stack: Vec<_> = (0..3 + width + above + width).collect();
                let mut expected = stack[..(3 + width + above) as usize].to_vec();
                expected[3..(3 + width) as usize]
                    .copy_from_slice(&stack[(3 + width + above) as usize..]);
                for op in builder.ops {
                    match op {
                        TIROp::Swap(depth) => {
                            let top = stack.len() - 1;
                            stack.swap(top, top - depth as usize);
                        }
                        TIROp::Pop(count) => stack.truncate(stack.len() - count as usize),
                        _ => unreachable!(),
                    }
                }
                assert_eq!(stack, expected, "width={width} above={above}");
            }
        }
    }

    #[test]
    fn branch_cleanup_preserves_outer_stack_and_result_order() {
        for outer in 0..5 {
            for dead in 0..40 {
                for keep in 0..20 {
                    let mut ops = Vec::new();
                    TIRBuilder::append_branch_cleanup(&mut ops, outer + dead + keep, outer, keep);
                    let mut stack: Vec<_> = (0..outer + dead + keep).collect();
                    let expected: Vec<_> = stack[..outer as usize]
                        .iter()
                        .chain(&stack[(outer + dead) as usize..])
                        .copied()
                        .collect();
                    for op in ops {
                        match op {
                            TIROp::Swap(d) => {
                                let top = stack.len() - 1;
                                stack.swap(top, top - d as usize);
                            }
                            TIROp::Pop(n) => stack.truncate(stack.len() - n as usize),
                            _ => unreachable!(),
                        }
                    }
                    assert_eq!(stack, expected, "outer={outer} dead={dead} keep={keep}");
                }
            }
        }
    }
}
