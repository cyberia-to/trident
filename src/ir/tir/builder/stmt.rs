// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Block and statement compilation.

use std::collections::BTreeMap;

use crate::ast::*;
use crate::span::Spanned;
use crate::tir::TIROp;

use super::layout::resolve_type_width;
use super::TIRBuilder;

// ─── Block and statement emission ─────────────────────────────────

impl TIRBuilder {
    /// Lower an assignment `place = value` on the stack machine.
    ///
    /// Handles three place forms produced by the parser:
    /// - simple variable `x`
    /// - dotted struct field `p.x` / `p.q.r` (a dotted `Place::Var`)
    /// - array/struct element `a[i]` (a `Place::Index`), constant index
    ///
    /// Each writes a single-word slot in place (evaluate the value, then
    /// `swap`/`pop` it into the target slot). Multi-word field/element writes
    /// and runtime array indices are not yet supported on the stack machine and
    /// emit an explicit error rather than miscompiling.
    pub(crate) fn build_assign(&mut self, place: &Place, value: &Expr) {
        match place {
            Place::Var(name) if !name.contains('.') => {
                // Simple variable reassignment (width-1 slot).
                self.build_expr(value);
                let depth = self.stack.access_var(name);
                self.flush_stack_effects();
                self.store_top_into(depth);
                self.stack.pop();
            }
            // Dotted field access `p.x` — the parser encodes this as a dotted
            // Place::Var; a structured FieldAccess reduces to the same store.
            Place::Var(name) => {
                self.build_dotted_field_store(name, value);
            }
            Place::FieldAccess(inner, field) => {
                // Flatten a structured field-access place into a dotted name
                // when the base is a (possibly dotted) variable.
                if let Some(base) = Self::place_dotted_name(inner) {
                    let full = format!("{}.{}", base, field.node);
                    self.build_dotted_field_store(&full, value);
                } else {
                    self.build_expr(value);
                    self.ops.push(TIROp::Comment(
                        "ERROR: unsupported field-assignment target".to_string(),
                    ));
                    self.ops.push(TIROp::Pop(1));
                    self.stack.pop();
                }
            }
            Place::Index(inner, index) => {
                self.build_index_store(inner, index, value);
            }
        }
    }

    /// Store the single word on top of the stack into the slot at `depth`
    /// (measured from the current top, with the value already pushed).
    fn store_top_into(&mut self, depth: u32) {
        if depth <= 15 {
            self.ops.push(TIROp::Swap(depth));
            self.ops.push(TIROp::Pop(1));
        } else {
            self.ops.push(TIROp::Comment(format!(
                "ERROR: assignment target at depth {} exceeds stack window (16)",
                depth
            )));
            self.ops.push(TIROp::Pop(1));
        }
    }

    /// Recover a dotted variable name from a place whose base is a variable.
    fn place_dotted_name(place: &Spanned<Place>) -> Option<String> {
        match &place.node {
            Place::Var(name) => Some(name.clone()),
            Place::FieldAccess(inner, field) => {
                Self::place_dotted_name(inner).map(|b| format!("{}.{}", b, field.node))
            }
            Place::Index(..) => None,
        }
    }

    /// Store into a dotted struct field `base.f0.f1…` (width-1 field only).
    fn build_dotted_field_store(&mut self, name: &str, value: &Expr) {
        let parts: Vec<&str> = name.split('.').collect();
        // Find the longest prefix that names a live variable, and bring it onto
        // the stack BEFORE evaluating the value (so a reload can't land on top
        // of the value word).
        let mut base_split = None;
        for split in 1..parts.len() {
            let var_name = parts[..split].join(".");
            if self.stack.has_var(&var_name) {
                self.stack.access_var(&var_name);
                self.flush_stack_effects();
                base_split = Some(split);
                break;
            }
        }
        self.build_expr(value);
        let Some(split) = base_split else {
            self.ops.push(TIROp::Comment(format!(
                "ERROR: unresolved assignment target '{}'",
                name
            )));
            self.ops.push(TIROp::Pop(1));
            self.stack.pop();
            return;
        };
        let var_name = parts[..split].join(".");
        let fields = &parts[split..];
        let base_info = self.stack.find_var_depth_and_width(&var_name);
        self.flush_stack_effects();
        let Some((base_depth, _)) = base_info else {
            self.ops.push(TIROp::Comment(format!(
                "ERROR: unresolved assignment target '{}'",
                name
            )));
            self.ops.push(TIROp::Pop(1));
            self.stack.pop();
            return;
        };
        match self.resolve_nested_field_offset(&var_name, fields) {
            Some((combined_offset, 1)) => {
                let real_depth = base_depth + combined_offset;
                self.store_top_into(real_depth);
            }
            Some((_, field_width)) => {
                self.ops.push(TIROp::Comment(format!(
                    "ERROR: assignment to multi-word field '{}' (width {}) not yet supported on the stack machine",
                    name, field_width
                )));
                self.ops.push(TIROp::Pop(1));
            }
            None => {
                self.ops.push(TIROp::Comment(format!(
                    "ERROR: unresolved field path '{}'",
                    name
                )));
                self.ops.push(TIROp::Pop(1));
            }
        }
        self.stack.pop();
    }

    /// Store into `a[idx]` with a constant index into a named variable
    /// (width-1 element only).
    fn build_index_store(
        &mut self,
        inner: &Spanned<Place>,
        index: &Spanned<Expr>,
        value: &Expr,
    ) {
        let base_name = match &inner.node {
            Place::Var(name) if !name.contains('.') => Some(name.clone()),
            _ => None,
        };
        let const_idx = match &index.node {
            Expr::Literal(Literal::Integer(i)) => Some(*i as u32),
            _ => None,
        };
        let (Some(name), Some(idx)) = (base_name, const_idx) else {
            self.build_expr(value);
            self.ops.push(TIROp::Comment(
                "ERROR: only constant-index assignment into a named array is supported on the stack machine".to_string(),
            ));
            self.ops.push(TIROp::Pop(1));
            self.stack.pop();
            return;
        };
        if self.stack.has_var(&name) {
            self.stack.access_var(&name);
            self.flush_stack_effects();
        }
        self.build_expr(value);
        let info = self.stack.find_var_with_elem_width(&name);
        self.flush_stack_effects();
        let Some((var_depth, var_width, elem_width)) = info else {
            self.ops.push(TIROp::Comment(format!(
                "ERROR: unresolved array '{}'",
                name
            )));
            self.ops.push(TIROp::Pop(1));
            self.stack.pop();
            return;
        };
        if elem_width != 1 {
            self.ops.push(TIROp::Comment(format!(
                "ERROR: assignment to multi-word array element '{}[{}]' (elem width {}) not yet supported",
                name, idx, elem_width
            )));
            self.ops.push(TIROp::Pop(1));
            self.stack.pop();
            return;
        }
        if (idx + 1) * elem_width > var_width {
            self.ops.push(TIROp::Comment(format!(
                "ERROR: index {} out of bounds for '{}' (width {})",
                idx, name, var_width
            )));
            self.ops.push(TIROp::Pop(1));
            self.stack.pop();
            return;
        }
        let base_offset = var_width - (idx + 1) * elem_width;
        let real_depth = var_depth + base_offset;
        self.store_top_into(real_depth);
        self.stack.pop();
    }

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
                self.build_assign(&place.node, &value.node);
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
