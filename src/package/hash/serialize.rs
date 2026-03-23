// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use crate::ast::*;
use crate::hash::ContentHash;

use super::normalize::*;

impl Normalizer {
    // ─── Type Serialization ────────────────────────────────────

    pub(crate) fn serialize_array_size(&mut self, size: &ArraySize) {
        match size {
            ArraySize::Literal(n) => {
                self.write_u8(0);
                self.write_u32(*n as u32);
            }
            ArraySize::Param(name) => {
                self.write_u8(1);
                self.write_str(name);
            }
            ArraySize::Add(a, b) => {
                self.write_u8(2);
                self.serialize_array_size(a);
                self.serialize_array_size(b);
            }
            ArraySize::Mul(a, b) => {
                self.write_u8(3);
                self.serialize_array_size(a);
                self.serialize_array_size(b);
            }
        }
    }

    pub(crate) fn serialize_type(&mut self, ty: &Type) {
        match ty {
            Type::Field => self.write_u8(TAG_TY_FIELD),
            Type::Bool => self.write_u8(TAG_TY_BOOL),
            Type::U32 => self.write_u8(TAG_TY_U32),
            Type::Digest => self.write_u8(TAG_TY_DIGEST),
            Type::XField => self.write_u8(TAG_TY_XFIELD),
            Type::Array(elem, size) => {
                self.write_u8(TAG_TY_ARRAY);
                self.serialize_type(elem);
                self.serialize_array_size(size);
            }
            Type::Tuple(elems) => {
                self.write_u8(TAG_TY_TUPLE);
                self.write_u16(elems.len() as u16);
                for elem in elems {
                    self.serialize_type(elem);
                }
            }
            Type::Named(path) => {
                self.write_u8(TAG_TY_NAMED);
                self.write_str(&path.as_dotted());
            }
        }
    }

    // ─── Block Serialization ───────────────────────────────────

    pub(crate) fn serialize_block(&mut self, block: &Block) {
        self.write_u8(TAG_BLOCK);
        self.write_u16(block.stmts.len() as u16);
        for stmt in &block.stmts {
            self.serialize_stmt(&stmt.node);
        }
        if let Some(ref tail) = block.tail_expr {
            self.write_u8(1); // has tail
            self.serialize_expr(&tail.node);
        } else {
            self.write_u8(0); // no tail
        }
    }

    // ─── Statement Serialization ───────────────────────────────

    pub(crate) fn serialize_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { pattern, init, .. } => {
                self.write_u8(TAG_LET);
                match pattern {
                    Pattern::Name(name) => {
                        self.write_u8(0); // single binding
                        let idx = self.env.push(&name.node);
                        self.write_u16(idx);
                    }
                    Pattern::Tuple(names) => {
                        self.write_u8(1); // tuple destructure
                        self.write_u16(names.len() as u16);
                        for name in names {
                            let idx = self.env.push(&name.node);
                            self.write_u16(idx);
                        }
                    }
                }
                self.serialize_expr(&init.node);
            }
            Stmt::Assign { place, value } => {
                self.write_u8(TAG_ASSIGN);
                self.serialize_place(&place.node);
                self.serialize_expr(&value.node);
            }
            Stmt::TupleAssign { names, value } => {
                self.write_u8(TAG_LET);
                self.write_u8(1); // tuple destructure
                self.write_u16(names.len() as u16);
                for name in names {
                    let idx = self.env.push(&name.node);
                    self.write_u16(idx);
                }
                self.serialize_expr(&value.node);
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                self.write_u8(TAG_IF);
                self.serialize_expr(&cond.node);
                self.serialize_block(&then_block.node);
                if let Some(ref else_blk) = else_block {
                    self.write_u8(1);
                    self.serialize_block(&else_blk.node);
                } else {
                    self.write_u8(0);
                }
            }
            Stmt::For {
                var,
                start,
                end,
                bound,
                body,
            } => {
                self.write_u8(TAG_FOR);
                let saved = self.env.save();
                let idx = self.env.push(&var.node);
                self.write_u16(idx);
                self.serialize_expr(&start.node);
                self.serialize_expr(&end.node);
                self.write_u32(bound.unwrap_or(0) as u32);
                self.serialize_block(&body.node);
                self.env.restore(saved);
            }
            Stmt::Expr(expr) => {
                self.write_u8(TAG_EXPR_STMT);
                self.serialize_expr(&expr.node);
            }
            Stmt::Return(val) => {
                self.write_u8(TAG_RETURN);
                if let Some(ref v) = val {
                    self.write_u8(1);
                    self.serialize_expr(&v.node);
                } else {
                    self.write_u8(0);
                }
            }
            Stmt::Reveal { event_name, fields } | Stmt::Seal { event_name, fields } => {
                // Emit and Seal are structurally identical for hashing
                self.write_u8(TAG_STRUCT_INIT);
                self.write_str(&event_name.node);
                self.write_u16(fields.len() as u16);
                for (name, val) in fields {
                    self.write_str(&name.node);
                    self.serialize_expr(&val.node);
                }
            }
            Stmt::Asm {
                body,
                effect,
                target,
            } => {
                self.write_u8(TAG_ASM);
                self.write_str(body);
                self.write_u16(*effect as u16);
                if let Some(ref t) = target {
                    self.write_u8(1);
                    self.write_str(t);
                } else {
                    self.write_u8(0);
                }
            }
            Stmt::Match { expr, arms } => {
                self.write_u8(TAG_MATCH);
                self.serialize_expr(&expr.node);
                self.write_u16(arms.len() as u16);
                for arm in arms {
                    self.serialize_match_pattern(&arm.pattern.node);
                    self.serialize_block(&arm.body.node);
                }
            }
        }
    }

    pub(crate) fn serialize_match_pattern(&mut self, pattern: &MatchPattern) {
        match pattern {
            MatchPattern::Literal(Literal::Integer(n)) => {
                self.write_u8(TAG_FIELD_LIT);
                self.write_u64(*n);
            }
            MatchPattern::Literal(Literal::Bool(b)) => {
                self.write_u8(TAG_BOOL_LIT);
                self.write_u8(if *b { 1 } else { 0 });
            }
            MatchPattern::Wildcard => {
                self.write_u8(0xFF); // wildcard marker
            }
            MatchPattern::Struct { name, fields } => {
                self.write_u8(TAG_STRUCT_PAT);
                self.write_str(&name.node);
                self.write_u32(fields.len() as u32);
                for spf in fields {
                    self.write_str(&spf.field_name.node);
                    match &spf.pattern.node {
                        FieldPattern::Binding(v) => {
                            self.write_u8(0x01);
                            self.write_str(v);
                        }
                        FieldPattern::Literal(Literal::Integer(n)) => {
                            self.write_u8(TAG_FIELD_LIT);
                            self.write_u64(*n);
                        }
                        FieldPattern::Literal(Literal::Bool(b)) => {
                            self.write_u8(TAG_BOOL_LIT);
                            self.write_u8(if *b { 1 } else { 0 });
                        }
                        FieldPattern::Wildcard => {
                            self.write_u8(0xFF);
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn serialize_place(&mut self, place: &Place) {
        match place {
            Place::Var(name) => {
                if let Some(idx) = self.env.lookup(name) {
                    self.write_u8(TAG_VAR);
                    self.write_u16(idx);
                } else {
                    // Unknown variable — use name
                    self.write_u8(TAG_VAR);
                    self.write_u16(0xFFFF);
                    self.write_str(name);
                }
            }
            Place::FieldAccess(base, field) => {
                self.write_u8(TAG_FIELD_ACCESS);
                self.serialize_place(&base.node);
                self.write_str(&field.node);
            }
            Place::Index(base, index) => {
                self.write_u8(TAG_ARRAY_INDEX);
                self.serialize_place(&base.node);
                self.serialize_expr(&index.node);
            }
        }
    }

    // ─── Expression Serialization ──────────────────────────────

    pub(crate) fn serialize_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Literal(Literal::Integer(n)) => {
                self.write_u8(TAG_FIELD_LIT);
                self.write_u64(*n);
            }
            Expr::Literal(Literal::Bool(b)) => {
                self.write_u8(TAG_BOOL_LIT);
                self.write_u8(if *b { 1 } else { 0 });
            }
            Expr::Var(name) => {
                if let Some(idx) = self.env.lookup(name) {
                    self.write_u8(TAG_VAR);
                    self.write_u16(idx);
                } else {
                    // Free variable (e.g., global constant) — use name
                    self.write_u8(TAG_VAR);
                    self.write_u16(0xFFFF);
                    self.write_str(name);
                }
            }
            Expr::BinOp { op, lhs, rhs } => {
                let tag = match op {
                    BinOp::Add => TAG_ADD,
                    BinOp::Mul => TAG_MUL,
                    BinOp::Eq => TAG_EQ,
                    BinOp::Lt => TAG_LT,
                    BinOp::BitAnd => TAG_BIT_AND,
                    BinOp::BitXor => TAG_BIT_XOR,
                    BinOp::DivMod => TAG_DIV_MOD,
                    BinOp::XFieldMul => TAG_XFIELD_MUL,
                };
                self.write_u8(tag);
                self.serialize_expr(&lhs.node);
                self.serialize_expr(&rhs.node);
            }
            Expr::Call {
                path,
                generic_args,
                args,
            } => {
                let name = path.node.as_dotted();
                let func_name = path.node.0.last().map(|s| s.as_str()).unwrap_or("");

                // Check if we have a hash for this function
                let resolved_hash = self
                    .fn_hashes
                    .get(&name)
                    .or_else(|| self.fn_hashes.get(func_name))
                    .copied();

                if let Some(hash) = resolved_hash {
                    self.write_u8(TAG_CALL);
                    self.write_hash(&hash);
                } else {
                    // Unknown function — use name-based call
                    self.write_u8(TAG_CALL);
                    self.write_hash(&ContentHash::zero());
                    self.write_str(&name);
                }

                // Generic args
                self.write_u16(generic_args.len() as u16);
                for ga in generic_args {
                    self.serialize_array_size(&ga.node);
                }

                // Args
                self.write_u16(args.len() as u16);
                for arg in args {
                    self.serialize_expr(&arg.node);
                }
            }
            Expr::FieldAccess { expr, field } => {
                self.write_u8(TAG_FIELD_ACCESS);
                self.serialize_expr(&expr.node);
                self.write_str(&field.node);
            }
            Expr::Index { expr, index } => {
                self.write_u8(TAG_ARRAY_INDEX);
                self.serialize_expr(&expr.node);
                self.serialize_expr(&index.node);
            }
            Expr::StructInit { path, fields } => {
                self.write_u8(TAG_STRUCT_INIT);
                self.write_str(&path.node.as_dotted());
                // Sort fields alphabetically for canonical order
                let mut sorted_fields: Vec<_> = fields.iter().collect();
                sorted_fields.sort_by_key(|(name, _)| &name.node);
                self.write_u16(sorted_fields.len() as u16);
                for (name, val) in sorted_fields {
                    self.write_str(&name.node);
                    self.serialize_expr(&val.node);
                }
            }
            Expr::ArrayInit(elems) => {
                self.write_u8(TAG_ARRAY_INIT);
                self.write_u16(elems.len() as u16);
                for elem in elems {
                    self.serialize_expr(&elem.node);
                }
            }
            Expr::Tuple(elems) => {
                self.write_u8(TAG_TUPLE);
                self.write_u16(elems.len() as u16);
                for elem in elems {
                    self.serialize_expr(&elem.node);
                }
            }
        }
    }
}
