// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Expression emission: build_expr, build_var_expr, build_field_access, build_index.

use crate::ast::*;
use crate::span::Spanned;
use crate::tir::TIROp;

use super::TIRBuilder;

impl TIRBuilder {
    pub(crate) fn build_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Literal(Literal::Integer(n)) => {
                self.emit_and_push(TIROp::Push(*n), 1);
            }
            Expr::Literal(Literal::Bool(b)) => {
                self.emit_and_push(TIROp::Push(if *b { 1 } else { 0 }), 1);
            }

            Expr::Var(name) => {
                self.build_var_expr(name);
            }

            Expr::BinOp { op, lhs, rhs } => {
                if matches!(op, BinOp::Lt) {
                    // Triton VM lt: result = (st0 < st1).
                    // For `a < b`, we need a at st0, b at st1.
                    // Push b first (deeper), then a (top).
                    self.build_expr(&rhs.node);
                    self.build_expr(&lhs.node);
                } else {
                    self.build_expr(&lhs.node);
                    self.build_expr(&rhs.node);
                }
                match op {
                    BinOp::Add => self.ops.push(TIROp::Add),
                    BinOp::Mul => self.ops.push(TIROp::Mul),
                    BinOp::Eq => self.ops.push(TIROp::Eq),
                    BinOp::Lt => self.ops.push(TIROp::Lt),
                    BinOp::BitAnd => self.ops.push(TIROp::And),
                    BinOp::BitXor => self.ops.push(TIROp::Xor),
                    BinOp::DivMod => {
                        self.ops.push(TIROp::Swap(1));
                        self.ops.push(TIROp::DivMod);
                    }
                    BinOp::XFieldMul => self.ops.push(TIROp::ExtMul),
                }
                self.stack.pop(); // rhs temp
                self.stack.pop(); // lhs temp
                let result_width = match op {
                    BinOp::DivMod => 2,
                    BinOp::XFieldMul => self.target_config.xfield_width,
                    _ => 1,
                };
                self.stack.push_temp(result_width);
                self.flush_stack_effects();
            }

            Expr::Call {
                path,
                generic_args,
                args,
            } => {
                let fn_name = path.node.as_dotted();
                self.build_call(&fn_name, generic_args, args);
            }

            Expr::Tuple(elements) => {
                for elem in elements {
                    self.build_expr(&elem.node);
                }
                let n = elements.len();
                let mut total_width = 0u32;
                for _ in 0..n {
                    if let Some(e) = self.stack.pop() {
                        total_width += e.width;
                    }
                }
                self.stack.push_temp(total_width);
                self.flush_stack_effects();
            }

            Expr::ArrayInit(elements) => {
                for elem in elements {
                    self.build_expr(&elem.node);
                }
                let n = elements.len();
                let mut total_width = 0u32;
                for _ in 0..n {
                    if let Some(e) = self.stack.pop() {
                        total_width += e.width;
                    }
                }
                self.stack.push_temp(total_width);
                if n > 0 {
                    if let Some(top) = self.stack.last_mut() {
                        top.elem_width = Some(total_width / n as u32);
                    }
                }
                self.flush_stack_effects();
            }

            Expr::FieldAccess { expr: inner, field } => {
                self.build_field_access(inner, field);
            }

            Expr::Index { expr: inner, index } => {
                self.build_index(inner, index);
            }

            Expr::StructInit { path, fields } => {
                let name = self.qualified_name(&path.node.as_dotted());
                let Some(definition) = self.struct_types.get(&name).cloned() else {
                    self.ops
                        .push(TIROp::Comment(format!("ERROR: unresolved struct '{name}'")));
                    return;
                };
                if definition.fields.len() != fields.len()
                    || definition.fields.iter().any(|declared| {
                        fields
                            .iter()
                            .filter(|(name, _)| name.node == declared.name.node)
                            .count()
                            != 1
                    })
                {
                    self.ops.push(TIROp::Comment(format!(
                        "ERROR: invalid fields for struct '{name}'"
                    )));
                    return;
                }
                let mut total_width = 0u32;
                // Evaluation order and representation both follow the declaration.
                for declared in &definition.fields {
                    let (_, value) = fields
                        .iter()
                        .find(|(name, _)| name.node == declared.name.node)
                        .expect("struct field set validated above");
                    self.build_expr(&value.node);
                }
                for _ in fields {
                    if let Some(e) = self.stack.pop() {
                        total_width += e.width;
                    }
                }
                self.stack.push_temp(total_width);
                self.flush_stack_effects();
            }
        }
    }

    // ── Var expression (dotted and simple) ────────────────────────

    pub(crate) fn build_var_expr(&mut self, name: &str) {
        if name.contains('.') {
            let parts: Vec<&str> = name.split('.').collect();

            // Try increasingly long prefixes as the base variable.
            let mut resolved = false;
            for split in 1..parts.len() {
                let var_name = parts[..split].join(".");
                let var_depth_info = self.find_var_depth_and_width(&var_name);
                if let Some((base_depth, _var_width)) = var_depth_info {
                    let fields = &parts[split..];
                    if let Some((combined_offset, field_width)) =
                        self.resolve_nested_field_offset(&var_name, fields)
                    {
                        let real_depth = base_depth + combined_offset;
                        self.stack.ensure_space(field_width);
                        self.flush_stack_effects();
                        for _ in 0..field_width {
                            self.ops.push(TIROp::Dup(real_depth + field_width - 1));
                        }
                        self.stack.push_temp(field_width);
                    } else {
                        let depth = base_depth;
                        self.emit_and_push(TIROp::Dup(depth), 1);
                    }
                    resolved = true;
                    break;
                }
            }

            if !resolved {
                // Module constant fallback.
                let qualified = self.qualified_name(name);
                if let Some(&val) = self.constants.get(&qualified) {
                    self.emit_and_push(TIROp::Push(val), 1);
                } else {
                    self.ops.push(TIROp::Comment(format!(
                        "ERROR: unresolved constant '{}'",
                        name
                    )));
                    self.emit_and_push(TIROp::Push(0), 1);
                }
            }
        } else {
            // Ensure variable is on stack (reload if spilled).
            self.stack.access_var(name);
            self.flush_stack_effects();

            let var_info = self.stack.find_var_depth_and_width(name);
            self.flush_stack_effects();

            if let Some((_depth, width)) = var_info {
                self.stack.ensure_space(width);
                self.flush_stack_effects();
                let depth = self.stack.access_var(name);
                self.flush_stack_effects();

                // Deep accesses are legalized after building all stack ops.
                for _ in 0..width {
                    self.ops.push(TIROp::Dup(depth + width - 1));
                }
                self.stack.push_temp(width);
            } else if let Some(&value) = self.constants.get(name) {
                self.emit_and_push(TIROp::Push(value), 1);
            } else {
                self.ops.push(TIROp::Comment(format!(
                    "ERROR: unresolved variable '{name}'"
                )));
                self.emit_and_push(TIROp::Push(0), 1);
            }
        }
    }

    // ── Field access ──────────────────────────────────────────────

    pub(crate) fn build_field_access(&mut self, inner: &Spanned<Expr>, field: &Spanned<String>) {
        self.build_expr(&inner.node);
        let inner_entry = self.stack.last().cloned();
        if let Some(entry) = inner_entry {
            let struct_width = entry.width;
            let field_offset = self.resolve_field_offset(&inner.node, &field.node);
            if let Some((offset, field_width)) = field_offset {
                for _ in 0..field_width {
                    self.ops.push(TIROp::Dup(offset + field_width - 1));
                }
                self.stack.pop();
                Self::append_branch_cleanup(
                    &mut self.ops,
                    struct_width + field_width,
                    0,
                    field_width,
                );
                self.stack.push_temp(field_width);
                self.flush_stack_effects();
            } else {
                // No layout from variable — search struct_types.
                let mut found: Option<(u32, u32)> = None;
                for sdef in self.struct_types.values() {
                    let total: u32 = sdef
                        .fields
                        .iter()
                        .map(|f| self.type_width(&f.ty.node))
                        .sum();
                    if total != struct_width {
                        continue;
                    }
                    let mut off = 0u32;
                    for sf in &sdef.fields {
                        let fw = self.type_width(&sf.ty.node);
                        if sf.name.node == field.node {
                            found = Some((total - off - fw, fw));
                            break;
                        }
                        off += fw;
                    }
                    if found.is_some() {
                        break;
                    }
                }
                if let Some((from_top, fw)) = found {
                    for _ in 0..fw {
                        self.ops.push(TIROp::Dup(from_top + fw - 1));
                    }
                    self.stack.pop();
                    Self::append_branch_cleanup(&mut self.ops, struct_width + fw, 0, fw);
                    self.stack.push_temp(fw);
                    self.flush_stack_effects();
                } else {
                    self.ops.push(TIROp::Comment(format!(
                        "ERROR: unresolved field '{}'",
                        field.node
                    )));
                    self.stack.pop();
                    self.stack.push_temp(1);
                    self.flush_stack_effects();
                }
            }
        } else {
            self.stack.push_temp(1);
            self.flush_stack_effects();
        }
    }

    // ── Index expression ──────────────────────────────────────────

    pub(crate) fn build_index(&mut self, inner: &Spanned<Expr>, index: &Spanned<Expr>) {
        let element_type = match self.expr_type(&inner.node) {
            Some(Type::Array(element, _)) => Some(*element),
            _ => None,
        };
        let named = if let Expr::Var(name) = &inner.node {
            self.stack
                .find_var_with_elem_width(name)
                .map(|info| (name.clone(), info))
        } else {
            None
        };
        self.flush_stack_effects();
        let (array_width, fallback_width) = if let Some((_, (_, width, element))) = &named {
            (*width, *element)
        } else {
            self.build_expr(&inner.node);
            self.stack
                .last()
                .map_or((0, 1), |e| (e.width, e.elem_width.unwrap_or(1)))
        };
        let elem_width = element_type
            .as_ref()
            .map(|ty| self.type_width(ty))
            .unwrap_or(fallback_width);
        let count = if elem_width > 0 {
            array_width / elem_width
        } else {
            0
        };
        // Proven constant access to a named array needs only element copies.
        if let (Some((_, (depth, _, _))), Expr::Literal(Literal::Integer(i))) =
            (&named, &index.node)
        {
            if *i < count as u64 {
                let source = depth + array_width - (*i as u32 + 1) * elem_width;
                self.ops
                    .extend((0..elem_width).map(|_| TIROp::Dup(source + elem_width - 1)));
                self.stack.push_temp(elem_width);
                return;
            }
        }
        self.build_expr(&index.node);
        self.assert_index_bound(count);
        let source_depth = if let Some((name, _)) = &named {
            self.find_var_depth_and_width(name)
                .map_or(1, |(depth, _)| depth)
        } else {
            1
        };
        if count > 0 {
            let leaf = |i: u32| {
                (0..elem_width)
                    .map(|_| TIROp::Dup(source_depth + array_width - i * elem_width - 1))
                    .collect()
            };
            if let Expr::Literal(Literal::Integer(i)) = index.node {
                if i < count as u64 {
                    self.ops.extend(leaf(i as u32));
                }
            } else {
                self.ops.extend(super::index::select(0, 0, count, &leaf));
            }
            let dead = if named.is_some() { 1 } else { array_width + 1 };
            Self::append_branch_cleanup(&mut self.ops, dead + elem_width, 0, elem_width);
        }
        self.stack.pop();
        if named.is_none() {
            self.stack.pop();
        }
        self.stack.push_temp(elem_width);
        if let Some(Type::Array(nested, _)) = element_type {
            let width = self.type_width(&nested);
            if let Some(top) = self.stack.last_mut() {
                top.elem_width = Some(width);
            }
        }
        self.flush_stack_effects();
    }
}
