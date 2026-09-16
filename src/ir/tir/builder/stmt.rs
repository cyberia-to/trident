// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Block and statement compilation.

use crate::ast::*;
use crate::tir::TIROp;

use super::TIRBuilder;

// ─── Block and statement emission ─────────────────────────────────

impl TIRBuilder {
    pub(crate) fn build_block(&mut self, block: &Block) -> u32 {
        let mut result_width = 0;
        for stmt in &block.stmts {
            let before = self.stack.stack_depth();
            self.build_stmt(&stmt.node);
            result_width = match &stmt.node {
                Stmt::If {
                    else_block: Some(_),
                    ..
                }
                | Stmt::Match { .. }
                | Stmt::Return(Some(_)) => self.stack.stack_depth().saturating_sub(before),
                _ => 0,
            };
        }
        if let Some(tail) = &block.tail_expr {
            let before = self.stack.stack_depth();
            self.build_expr(&tail.node);
            result_width = self.stack.stack_depth().saturating_sub(before);
        }
        result_width
    }

    pub(crate) fn build_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let {
                pattern, init, ty, ..
            } => {
                let inferred_ty = ty
                    .as_ref()
                    .map(|t| t.node.clone())
                    .or_else(|| self.expr_type(&init.node));
                self.build_expr(&init.node);

                match pattern {
                    Pattern::Name(name) => {
                        if name.node != "_" {
                            if let Some(ty) = &inferred_ty {
                                self.var_types.insert(name.node.clone(), ty.clone());
                                self.register_struct_layout_from_type(&name.node, ty);
                            }
                            if let Some(top) = self.stack.last_mut() {
                                top.name = Some(name.node.clone());
                            }
                            // If type is an array, record elem_width.
                            if let Some(sp_ty) = ty {
                                if let Type::Array(inner_ty, _) = &sp_ty.node {
                                    let ew = self.type_width(inner_ty);
                                    if let Some(top) = self.stack.last_mut() {
                                        top.elem_width = Some(ew);
                                    }
                                }
                            }
                        }
                    }
                    Pattern::Tuple(names) => {
                        let top = self.stack.pop();
                        if let Some(entry) = top {
                            let total_width = entry.width;
                            let n = names.len() as u32;
                            let fallback_width = if n > 0 { total_width / n } else { 1 };

                            let tuple_types = match &inferred_ty {
                                Some(Type::Tuple(parts)) => Some(parts),
                                _ => None,
                            };
                            let widths: Vec<_> = (0..names.len())
                                .map(|index| {
                                    tuple_types
                                        .and_then(|ts| ts.get(index))
                                        .map(|t| self.type_width(t))
                                        .unwrap_or(fallback_width)
                                })
                                .collect();
                            for (index, name) in names.iter().enumerate() {
                                let elem_width = widths[index];
                                if let Some(ty) = tuple_types.and_then(|ts| ts.get(index)) {
                                    self.var_types.insert(name.node.clone(), ty.clone());
                                    self.register_struct_layout_from_type(&name.node, ty);
                                }
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
                            for (name, width) in names.iter().zip(&widths).rev() {
                                if name.node == "_" {
                                    trailing_wildcards += width;
                                    self.stack.pop();
                                } else {
                                    break;
                                }
                            }
                            if trailing_wildcards > 0 {
                                self.emit_pop(trailing_wildcards);
                            }
                        }
                    }
                }
            }

            Stmt::Assign { place, value } => {
                self.build_assign(&place.node, &value.node);
            }

            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                self.build_expr(&cond.node);
                self.stack.pop(); // cond consumed
                let types = self.var_types.clone();
                let layouts = self.struct_layouts.clone();

                if let Some(else_blk) = else_block {
                    let saved = self.stack.save_state();
                    let pre_depth = self.stack.stack_depth();
                    let (mut then_body, then_width) =
                        self.build_value_block_as_ir(&then_block.node);
                    let then_depth = self.stack.stack_depth();
                    self.stack.restore_state(saved.clone());
                    self.var_types = types.clone();
                    self.struct_layouts = layouts.clone();
                    let (mut else_body, else_width) = self.build_value_block_as_ir(&else_blk.node);
                    let else_depth = self.stack.stack_depth();
                    self.stack.restore_state(saved);

                    // Local bindings do not form part of a branch result.
                    // Branches may have different local counts but must agree
                    // on the width of the value they return.
                    let keep = if then_width == else_width {
                        then_width
                    } else {
                        0
                    };
                    if then_width != else_width {
                        self.ops.push(TIROp::Comment(
                            "ERROR: conditional branches have different result widths".into(),
                        ));
                    }

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
                    Self::append_branch_cleanup(
                        &mut then_body,
                        self.stack.stack_depth(),
                        pre_depth,
                        0,
                    );
                    self.stack.restore_state(saved);

                    self.ops.push(TIROp::IfOnly { then_body });
                }
                self.var_types = types;
                self.struct_layouts = layouts;
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
                self.ops.push(TIROp::Dup(1)); // [..., start, end, start]
                self.ops.push(TIROp::Sub); // [..., start, end - start]

                self.ops.push(TIROp::Call(loop_label.clone()));
                // After return: [..., index, 0] — pop both counter and index
                self.ops.push(TIROp::Pop(2));
                self.stack.pop(); // pop counter model
                self.stack.pop(); // pop index model

                let saved = self.stack.save_state();
                let types = self.var_types.clone();
                let layouts = self.struct_layouts.clone();
                let pre_loop_depth = self.stack.stack_depth();
                // The loop subroutine's real stack has all outer variables
                // plus [index, counter] on top. Keep outer vars in the model
                // so the loop body can reference them at the correct depths.
                self.stack.push_named(&var.node, 1); // index (depth 1)
                self.stack.push_temp(1); // counter (depth 0)
                let return_flag_depth = self
                    .stack
                    .find_var_depth_and_width(super::early_return::FLAG)
                    .map(|(depth, _)| depth);

                let mut body_ir = self.build_block_as_ir(&body.node);

                // Clean up any locals created in the loop body.
                // Keep everything that existed before the body: outer vars + index + counter.
                let total_depth = self.stack.stack_depth();
                let keep = pre_loop_depth + 2; // outer vars + index + counter
                let leftover = total_depth.saturating_sub(keep);
                if leftover > 0 {
                    body_ir.push(TIROp::Pop(leftover));
                }

                // A source return terminates every enclosing counted loop.
                // Its flag is a function-local slot; clearing this loop's
                // counter makes the existing loop epilogue exit immediately.
                if let Some(depth) = return_flag_depth {
                    body_ir.push(TIROp::Dup(depth));
                    body_ir.push(TIROp::IfOnly {
                        then_body: vec![TIROp::Pop(1), TIROp::Push(0)],
                    });
                }

                // Increment the index.
                // After cleanup, stack is [..., index, counter] (counter at st0).
                // Swap to bring index to top, add 1, swap back.
                body_ir.push(TIROp::Swap(1)); // [..., counter, index]
                body_ir.push(TIROp::Push(1));
                body_ir.push(TIROp::Add); // [..., counter, index+1]
                body_ir.push(TIROp::Swap(1)); // [..., index+1, counter]
                                              // recurse is added by the lowering

                self.stack.restore_state(saved);
                self.var_types = types;
                self.struct_layouts = layouts;

                self.ops.push(TIROp::Loop {
                    label: loop_label,
                    body: body_ir,
                });
            }

            Stmt::TupleAssign { names, value } => {
                self.build_tuple_assign(names, &value.node);
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

                // Keep evaluated payloads in the model until every expression is
                // built, so later variable loads see their actual stack depth.
                let mut field_count = 0;
                for def_name in &decl_order {
                    if let Some((_name, val)) = fields.iter().find(|(n, _)| n.node == *def_name) {
                        self.build_expr(&val.node);
                        field_count += self.stack.last().map_or(0, |v| v.width);
                    }
                }
                for _ in &decl_order {
                    self.stack.pop();
                }

                self.ops.push(TIROp::Reveal {
                    name: event_name.node.clone(),
                    tag,
                    field_count,
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

                // Assembly shares source RAM. Spilling named locals around an
                // opaque block would expose them to its arbitrary RAM accesses.
                // Keep the compiler-owned stack prefix in place; the assembly
                // contract requires the block to preserve that prefix.
                if *effect < 0 && !self.stack.can_pop_anonymous(effect.unsigned_abs()) {
                    self.ops.push(TIROp::Comment(
                        "ERROR: inline assembly stack effect consumes a named binding or exceeds the available anonymous stack".into(),
                    ));
                    return;
                }
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
                    self.stack.pop_anonymous(effect.unsigned_abs());
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
                // Same declaration-order evaluation and bottom-first flattened
                // payload as Reveal. The machine adapter chooses hash word order.
                let mut field_count = 0;
                for def_name in &decl_order {
                    if let Some((_name, val)) = fields.iter().find(|(n, _)| n.node == *def_name) {
                        self.build_expr(&val.node);
                        field_count += self.stack.last().map_or(0, |v| v.width);
                    }
                }
                for _ in &decl_order {
                    self.stack.pop();
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
