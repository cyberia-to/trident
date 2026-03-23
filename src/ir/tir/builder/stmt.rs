// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Block and statement compilation.

use std::collections::BTreeMap;

use crate::ast::*;
use crate::tir::TIROp;

use super::layout::resolve_type_width;
use super::TIRBuilder;

// ─── Block and statement emission ─────────────────────────────────

impl TIRBuilder {
    /// Append Pop ops to clean up locals created in an if/else branch.
    /// `post_depth` is stack_depth() after the branch body, `pre_depth` before.
    /// `keep` is the number of words to preserve on top (e.g. a tail expression value).
    fn append_branch_cleanup(body: &mut Vec<TIROp>, post_depth: u32, pre_depth: u32, keep: u32) {
        let leftover = post_depth.saturating_sub(pre_depth + keep);
        if leftover > 0 {
            if keep > 0 {
                // Swap the result value(s) past the dead locals, then pop.
                if leftover <= 15 {
                    body.push(TIROp::Swap(leftover));
                } else {
                    for _ in 0..leftover {
                        body.push(TIROp::Swap(1));
                    }
                }
            }
            let mut remaining = leftover;
            while remaining > 0 {
                let batch = remaining.min(5);
                body.push(TIROp::Pop(batch));
                remaining -= batch;
            }
        }
    }

    pub(crate) fn build_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.build_stmt(&stmt.node);
        }
        if let Some(tail) = &block.tail_expr {
            self.build_expr(&tail.node);
        }
    }

    pub(crate) fn build_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let {
                pattern, init, ty, ..
            } => {
                self.build_expr(&init.node);

                match pattern {
                    Pattern::Name(name) => {
                        if name.node != "_" {
                            if let Some(top) = self.stack.last_mut() {
                                top.name = Some(name.node.clone());
                            }
                            // If type is an array, record elem_width.
                            if let Some(sp_ty) = ty {
                                if let Type::Array(inner_ty, _) = &sp_ty.node {
                                    let ew = resolve_type_width(inner_ty, &self.target_config);
                                    if let Some(top) = self.stack.last_mut() {
                                        top.elem_width = Some(ew);
                                    }
                                }
                            }
                            // Record struct field layout from struct init.
                            if let Expr::StructInit { fields, .. } = &init.node {
                                let mut field_map = BTreeMap::new();
                                let widths = self.compute_struct_field_widths(ty, fields);
                                let total: u32 = widths.iter().sum();
                                let mut offset = 0u32;
                                for (i, (fname, _)) in fields.iter().enumerate() {
                                    let fw = widths.get(i).copied().unwrap_or(1);
                                    let from_top = total - offset - fw;
                                    field_map.insert(fname.node.clone(), (from_top, fw));
                                    offset += fw;
                                }
                                self.struct_layouts.insert(name.node.clone(), field_map);
                            } else if let Some(sp_ty) = ty {
                                self.register_struct_layout_from_type(&name.node, &sp_ty.node);
                            }
                        }
                    }
                    Pattern::Tuple(names) => {
                        let top = self.stack.pop();
                        if let Some(entry) = top {
                            let total_width = entry.width;
                            let n = names.len() as u32;
                            let elem_width = if n > 0 { total_width / n } else { 1 };

                            for name in names.iter() {
                                let var_name = if name.node == "_" {
                                    "__anon"
                                } else {
                                    &name.node
                                };
                                self.stack.push_named(var_name, elem_width);
                                self.flush_stack_effects();
                            }

                            // Eagerly pop trailing wildcard bindings.
                            // For `let (h1, _, _, _, _) = digest`, wildcards on top
                            // of the stack are immediately discarded.
                            let mut trailing_wildcards = 0u32;
                            for name in names.iter().rev() {
                                if name.node == "_" {
                                    trailing_wildcards += elem_width;
                                } else {
                                    break;
                                }
                            }
                            if trailing_wildcards > 0 {
                                for _ in 0..(trailing_wildcards / elem_width) {
                                    self.stack.pop();
                                }
                                self.emit_pop(trailing_wildcards);
                            }
                        }
                    }
                }
            }

            Stmt::Assign { place, value } => {
                if let Place::Var(name) = &place.node {
                    self.build_expr(&value.node);
                    let depth = self.stack.access_var(name);
                    self.flush_stack_effects();
                    if depth <= 15 {
                        self.ops.push(TIROp::Swap(depth));
                        self.ops.push(TIROp::Pop(1));
                    }
                    self.stack.pop();
                }
            }

            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                self.build_expr(&cond.node);
                self.stack.pop(); // cond consumed

                if let Some(else_blk) = else_block {
                    let saved = self.stack.save_state();
                    let pre_depth = self.stack.stack_depth();
                    let mut then_body = self.build_block_as_ir(&then_block.node);
                    let then_depth = self.stack.stack_depth();
                    self.stack.restore_state(saved.clone());
                    let mut else_body = self.build_block_as_ir(&else_blk.node);
                    let else_depth = self.stack.stack_depth();
                    self.stack.restore_state(saved);

                    // If both branches grow the stack by the same amount,
                    // they produce a value. Preserve it, clean up only locals.
                    let then_grow = then_depth.saturating_sub(pre_depth);
                    let else_grow = else_depth.saturating_sub(pre_depth);
                    let keep = if then_grow > 0 && then_grow == else_grow {
                        then_grow
                    } else {
                        0
                    };

                    Self::append_branch_cleanup(&mut then_body, then_depth, pre_depth, keep);
                    Self::append_branch_cleanup(&mut else_body, else_depth, pre_depth, keep);

                    self.ops.push(TIROp::IfElse {
                        then_body,
                        else_body,
                    });

                    if keep > 0 {
                        self.stack.push_temp(keep);
                    }
                } else {
                    let saved = self.stack.save_state();
                    let pre_depth = self.stack.stack_depth();
                    let mut then_body = self.build_block_as_ir(&then_block.node);
                    Self::append_branch_cleanup(&mut then_body, self.stack.stack_depth(), pre_depth, 0);
                    self.stack.restore_state(saved);

                    self.ops.push(TIROp::IfOnly { then_body });
                }
            }

            Stmt::For {
                var,
                start,
                end,
                body,
                ..
            } => {
                let loop_label = self.fresh_label("loop");

                // Push index (start) and counter (end - start) onto the stack.
                // Stack after: [..., index, counter]  (counter on top)
                self.build_expr(&start.node);
                self.build_expr(&end.node);
                // counter = end - start: dup start, then Sub (st1 - st0)
                self.ops.push(TIROp::Dup(1));  // [..., start, end, start]
                self.ops.push(TIROp::Sub);     // [..., start, end - start]

                self.ops.push(TIROp::Call(loop_label.clone()));
                // After return: [..., index, 0] — pop both counter and index
                self.ops.push(TIROp::Pop(2));
                self.stack.pop(); // pop counter model
                self.stack.pop(); // pop index model

                let saved = self.stack.save_state();
                let pre_loop_depth = self.stack.stack_depth();
                // The loop subroutine's real stack has all outer variables
                // plus [index, counter] on top. Keep outer vars in the model
                // so the loop body can reference them at the correct depths.
                self.stack.push_named(&var.node, 1); // index (depth 1)
                self.stack.push_temp(1);              // counter (depth 0)

                let mut body_ir = self.build_block_as_ir(&body.node);

                // Clean up any locals created in the loop body.
                // Keep everything that existed before the body: outer vars + index + counter.
                let total_depth = self.stack.stack_depth();
                let keep = pre_loop_depth + 2; // outer vars + index + counter
                let leftover = total_depth.saturating_sub(keep);
                if leftover > 0 {
                    let mut remaining = leftover;
                    while remaining > 0 {
                        let batch = remaining.min(5);
                        body_ir.push(TIROp::Pop(batch));
                        remaining -= batch;
                    }
                }

                // Increment the index.
                // After cleanup, stack is [..., index, counter] (counter at st0).
                // Swap to bring index to top, add 1, swap back.
                body_ir.push(TIROp::Swap(1));  // [..., counter, index]
                body_ir.push(TIROp::Push(1));
                body_ir.push(TIROp::Add);      // [..., counter, index+1]
                body_ir.push(TIROp::Swap(1));  // [..., index+1, counter]
                // recurse is added by the lowering

                self.stack.restore_state(saved);

                self.ops.push(TIROp::Loop {
                    label: loop_label,
                    body: body_ir,
                });
            }

            Stmt::TupleAssign { names, value } => {
                self.build_expr(&value.node);
                let top = self.stack.pop();
                if let Some(entry) = top {
                    let total_width = entry.width;
                    let n = names.len() as u32;
                    let elem_width = if n > 0 { total_width / n } else { 1 };

                    for name in names.iter().rev() {
                        let depth = self.stack.access_var(&name.node);
                        self.flush_stack_effects();
                        if elem_width == 1 {
                            self.ops.push(TIROp::Swap(depth));
                            self.ops.push(TIROp::Pop(1));
                        }
                    }
                    let _ = total_width;
                }
            }

            Stmt::Expr(expr) => {
                let before = self.stack.stack_len();
                self.build_expr(&expr.node);
                while self.stack.stack_len() > before {
                    if let Some(top) = self.stack.last() {
                        let w = top.width;
                        if w > 0 {
                            self.emit_pop(w);
                        }
                    }
                    self.stack.pop();
                }
            }

            Stmt::Return(value) => {
                if let Some(val) = value {
                    self.build_expr(&val.node);
                }
            }

            Stmt::Reveal { event_name, fields } => {
                let tag = match self.event_tags.get(&event_name.node).copied() {
                    Some(t) => t,
                    None => {
                        self.ops.push(TIROp::Comment(format!(
                            "BUG: unregistered event '{}'",
                            event_name.node
                        )));
                        0
                    }
                };
                let decl_order = self
                    .event_defs
                    .get(&event_name.node)
                    .cloned()
                    .unwrap_or_default();

                for def_name in &decl_order {
                    if let Some((_name, val)) = fields.iter().find(|(n, _)| n.node == *def_name) {
                        self.build_expr(&val.node);
                        self.stack.pop();
                    }
                }

                self.ops.push(TIROp::Reveal {
                    name: event_name.node.clone(),
                    tag,
                    field_count: decl_order.len() as u32,
                });
            }

            Stmt::Asm {
                body,
                effect,
                target,
            } => {
                if let Some(tag) = target {
                    if tag != &self.target_config.name {
                        return;
                    }
                }

                self.stack.spill_all_named();
                self.flush_stack_effects();

                let lines: Vec<String> = body
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect();

                if !lines.is_empty() {
                    self.ops.push(TIROp::Asm {
                        lines,
                        effect: *effect,
                    });
                }

                if *effect > 0 {
                    for _ in 0..*effect {
                        self.stack.push_temp(1);
                    }
                } else if *effect < 0 {
                    for _ in 0..effect.unsigned_abs() {
                        self.stack.pop();
                    }
                }
            }

            Stmt::Match { expr, arms } => {
                self.build_match(expr, arms);
            }

            Stmt::Seal { event_name, fields } => {
                let tag = match self.event_tags.get(&event_name.node).copied() {
                    Some(t) => t,
                    None => {
                        self.ops.push(TIROp::Comment(format!(
                            "BUG: unregistered event '{}'",
                            event_name.node
                        )));
                        0
                    }
                };
                let decl_order = self
                    .event_defs
                    .get(&event_name.node)
                    .cloned()
                    .unwrap_or_default();
                let field_count = decl_order.len() as u32;

                // Push fields in reverse declaration order (so first declared
                // field ends up on top after all pushes).
                for def_name in decl_order.iter().rev() {
                    if let Some((_name, val)) = fields.iter().find(|(n, _)| n.node == *def_name) {
                        self.build_expr(&val.node);
                        self.stack.pop();
                    }
                }

                self.ops.push(TIROp::Seal {
                    name: event_name.node.clone(),
                    tag,
                    field_count,
                });
            }
        }
    }
}
