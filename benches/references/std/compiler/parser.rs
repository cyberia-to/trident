// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use std::time::Instant;

use trident::ast::*;
use trident::lexeme::Lexeme;
use trident::lexer::Lexer;
use trident::span::Spanned;

// NK_* constants — must match std/compiler/parser.tri
const NK_FILE: u64 = 1;
const NK_USE: u64 = 2;
const NK_FN: u64 = 3;
const NK_CONST: u64 = 4;
const NK_STRUCT: u64 = 5;
const NK_EVENT: u64 = 6;
const NK_PARAM: u64 = 7;
const NK_STRUCT_FIELD: u64 = 8;
const NK_TYPE_FIELD: u64 = 10;
const NK_TYPE_XFIELD: u64 = 11;
const NK_TYPE_BOOL: u64 = 12;
const NK_TYPE_U32: u64 = 13;
const NK_TYPE_DIGEST: u64 = 14;
const NK_TYPE_ARRAY: u64 = 15;
const NK_TYPE_TUPLE: u64 = 16;
const NK_TYPE_NAMED: u64 = 17;
const NK_LET: u64 = 20;
const NK_ASSIGN: u64 = 21;
const NK_IF: u64 = 22;
const NK_FOR: u64 = 23;
const NK_RETURN: u64 = 24;
const NK_EXPR_STMT: u64 = 25;
const NK_BLOCK: u64 = 26;
const NK_REVEAL: u64 = 27;
const NK_SEAL: u64 = 28;
const NK_ASM: u64 = 29;
const NK_MATCH: u64 = 30;
const NK_MATCH_ARM: u64 = 31;
const NK_LIT_INT: u64 = 40;
const NK_LIT_BOOL: u64 = 41;
const NK_VAR: u64 = 42;
const NK_BINOP: u64 = 43;
const NK_CALL: u64 = 44;
const NK_FIELD_ACCESS: u64 = 45;
const NK_INDEX: u64 = 46;
const NK_STRUCT_INIT: u64 = 47;
const NK_ARRAY_INIT: u64 = 48;
const NK_TUPLE: u64 = 49;
const NK_INIT_FIELD: u64 = 50;
const NK_PAT_NAME: u64 = 51;
const _NK_PAT_TUPLE: u64 = 52;
const NK_PAT_WILDCARD: u64 = 53;
const NK_PAT_LIT: u64 = 54;
const NK_PAT_STRUCT: u64 = 55;

// OP_* binop codes — must match parser.tri
const OP_EQ: u64 = 1;
const OP_LT: u64 = 2;
const OP_ADD: u64 = 3;
const OP_MUL: u64 = 4;
const OP_XFMUL: u64 = 5;
const OP_BAND: u64 = 6;
const OP_BXOR: u64 = 7;
const OP_DIVMOD: u64 = 8;

/// Map a Lexeme to the TK_* integer used by std/compiler/lexer.tri.
fn lexeme_to_tk(lexeme: &Lexeme) -> u64 {
    match lexeme {
        Lexeme::Program => 1,
        Lexeme::Module => 2,
        Lexeme::Use => 3,
        Lexeme::Fn => 4,
        Lexeme::Pub => 5,
        Lexeme::Sec => 6,
        Lexeme::Let => 7,
        Lexeme::Mut => 8,
        Lexeme::Const => 9,
        Lexeme::Struct => 10,
        Lexeme::If => 11,
        Lexeme::Else => 12,
        Lexeme::For => 13,
        Lexeme::In => 14,
        Lexeme::Bounded => 15,
        Lexeme::Return => 16,
        Lexeme::True => 17,
        Lexeme::False => 18,
        Lexeme::Event => 19,
        Lexeme::Reveal => 20,
        Lexeme::Seal => 21,
        Lexeme::Match => 22,
        Lexeme::FieldTy => 23,
        Lexeme::XFieldTy => 24,
        Lexeme::BoolTy => 25,
        Lexeme::U32Ty => 26,
        Lexeme::DigestTy => 27,
        Lexeme::LParen => 28,
        Lexeme::RParen => 29,
        Lexeme::LBrace => 30,
        Lexeme::RBrace => 31,
        Lexeme::LBracket => 32,
        Lexeme::RBracket => 33,
        Lexeme::Comma => 34,
        Lexeme::Colon => 35,
        Lexeme::Semicolon => 36,
        Lexeme::Dot => 37,
        Lexeme::DotDot => 38,
        Lexeme::Arrow => 39,
        Lexeme::Eq => 40,
        Lexeme::FatArrow => 41,
        Lexeme::EqEq => 42,
        Lexeme::Plus => 43,
        Lexeme::Star => 44,
        Lexeme::StarDot => 45,
        Lexeme::Lt => 46,
        Lexeme::Gt => 47,
        Lexeme::Amp => 48,
        Lexeme::Caret => 49,
        Lexeme::SlashPercent => 50,
        Lexeme::Hash => 51,
        Lexeme::Underscore => 52,
        Lexeme::Integer(_) => 53,
        Lexeme::Ident(_) => 54,
        Lexeme::AsmBlock { .. } => 55,
        Lexeme::Eof => 56,
    }
}

/// Flat AST builder — accumulates stride-8 nodes.
struct AstBuilder {
    nodes: Vec<[u64; 8]>,
    /// Map from Rust token index to self-hosted token index.
    /// The self-hosted parser uses token indices into the flat token array.
    tokens: Vec<Spanned<Lexeme>>,
}

impl AstBuilder {
    fn new(tokens: Vec<Spanned<Lexeme>>) -> Self {
        Self {
            nodes: Vec::new(),
            tokens,
        }
    }

    /// Emit a node, return its index.
    fn emit(&mut self, kind: u64, f: [u64; 7]) -> u64 {
        let idx = self.nodes.len() as u64;
        self.nodes.push([kind, f[0], f[1], f[2], f[3], f[4], f[5], f[6]]);
        idx
    }

    /// Backpatch a field in an already-emitted node.
    fn backpatch(&mut self, node_idx: u64, field_offset: usize, value: u64) {
        self.nodes[node_idx as usize][field_offset] = value;
    }

    /// Find the token index closest to a span start.
    fn tok_idx_for_span(&self, span: trident::span::Span) -> u64 {
        for (i, tok) in self.tokens.iter().enumerate() {
            if tok.span.start == span.start {
                return i as u64;
            }
        }
        // Fallback: find nearest
        let mut best = 0u64;
        let mut best_dist = u32::MAX;
        for (i, tok) in self.tokens.iter().enumerate() {
            let dist = tok.span.start.abs_diff(span.start);
            if dist < best_dist {
                best_dist = dist;
                best = i as u64;
            }
        }
        best
    }

    fn binop_code(op: &BinOp) -> u64 {
        match op {
            BinOp::Eq => OP_EQ,
            BinOp::Lt => OP_LT,
            BinOp::Add => OP_ADD,
            BinOp::Mul => OP_MUL,
            BinOp::XFieldMul => OP_XFMUL,
            BinOp::BitAnd => OP_BAND,
            BinOp::BitXor => OP_BXOR,
            BinOp::DivMod => OP_DIVMOD,
        }
    }

    // --- Serialize Rust AST to flat nodes ---

    fn serialize_file(&mut self, file: &File) -> u64 {
        let file_kind = match file.kind {
            FileKind::Program => 1,
            FileKind::Module => 2,
        };
        let name_tok = self.tok_idx_for_span(file.name.span);

        // Reserve file node
        let file_node = self.emit(NK_FILE, [file_kind, name_tok, 0, 0, 0, 0, 0]);

        // Serialize uses
        let uses_start = self.nodes.len() as u64;
        for use_path in &file.uses {
            self.serialize_use(use_path);
        }
        let uses_count = file.uses.len() as u64;

        // Serialize items
        let items_start = self.nodes.len() as u64;
        for item in &file.items {
            self.serialize_item(item);
        }
        let items_count = file.items.len() as u64;

        // Backpatch file node
        self.backpatch(file_node, 3, uses_start);
        self.backpatch(file_node, 4, uses_count);
        self.backpatch(file_node, 5, items_start);
        self.backpatch(file_node, 6, items_count);

        file_node
    }

    fn serialize_use(&mut self, use_path: &Spanned<ModulePath>) -> u64 {
        // Use node: NK_USE, path_start_tok, path_end_tok
        // Find first and last token of the path
        let start_tok = self.tok_idx_for_span(use_path.span);
        // The use keyword is before the path, so start_tok points to "use".
        // The path starts at the next token.
        let path_start = start_tok + 1;
        // For the end, we need to count dot-separated parts
        let num_parts = use_path.node.0.len();
        // Each part is ident, and parts are separated by dots
        // So total tokens = num_parts + (num_parts - 1) dots
        let path_end = path_start + (num_parts as u64 * 2).saturating_sub(1);
        self.emit(NK_USE, [path_start, path_end, 0, 0, 0, 0, 0])
    }

    fn serialize_item(&mut self, item: &Spanned<Item>) -> u64 {
        match &item.node {
            Item::Fn(f) => self.serialize_fn(f),
            Item::Const(c) => self.serialize_const(c),
            Item::Struct(s) => self.serialize_struct(s),
            Item::Event(e) => self.serialize_event(e),
        }
    }

    fn serialize_fn(&mut self, f: &FnDef) -> u64 {
        let name_tok = self.tok_idx_for_span(f.name.span);
        let mut flags: u64 = 0;
        if f.is_pub {
            flags |= 1;
        }
        if f.is_test {
            flags |= 2;
        }

        // Reserve fn node
        let fn_node = self.emit(NK_FN, [name_tok, 0, 0, 0, 0, flags, 0]);

        // Params
        let params_start = self.nodes.len() as u64;
        for param in &f.params {
            self.serialize_param(param);
        }
        let params_count = f.params.len() as u64;
        self.backpatch(fn_node, 2, params_start);
        self.backpatch(fn_node, 3, params_count);

        // Return type
        let ret_node = if let Some(ret_ty) = &f.return_ty {
            self.serialize_type(&ret_ty.node)
        } else {
            0
        };
        self.backpatch(fn_node, 4, ret_node);

        // Body
        let body_node = if let Some(body) = &f.body {
            self.serialize_block(&body.node)
        } else {
            0
        };
        self.backpatch(fn_node, 5, body_node);

        // Move flags to field_6
        self.backpatch(fn_node, 6, flags);
        // Fix field_5 to be body_node
        self.backpatch(fn_node, 5, body_node);

        fn_node
    }

    fn serialize_param(&mut self, param: &Param) -> u64 {
        let name_tok = self.tok_idx_for_span(param.name.span);
        let type_node = self.serialize_type(&param.ty.node);
        self.emit(NK_PARAM, [name_tok, type_node, 0, 0, 0, 0, 0])
    }

    fn serialize_const(&mut self, c: &ConstDef) -> u64 {
        let name_tok = self.tok_idx_for_span(c.name.span);
        let type_node = self.serialize_type(&c.ty.node);
        let value_node = self.serialize_expr(&c.value);
        let flags = if c.is_pub { 1 } else { 0 };
        self.emit(NK_CONST, [name_tok, type_node, value_node, flags, 0, 0, 0])
    }

    fn serialize_struct(&mut self, s: &StructDef) -> u64 {
        let name_tok = self.tok_idx_for_span(s.name.span);
        let flags = if s.is_pub { 1 } else { 0 };
        let struct_node = self.emit(NK_STRUCT, [name_tok, 0, 0, flags, 0, 0, 0]);

        let fields_start = self.nodes.len() as u64;
        for field in &s.fields {
            let fname_tok = self.tok_idx_for_span(field.name.span);
            let ftype_node = self.serialize_type(&field.ty.node);
            let fflags = if field.is_pub { 1 } else { 0 };
            self.emit(NK_STRUCT_FIELD, [fname_tok, ftype_node, fflags, 0, 0, 0, 0]);
        }
        let fields_count = s.fields.len() as u64;
        self.backpatch(struct_node, 2, fields_start);
        self.backpatch(struct_node, 3, fields_count);

        struct_node
    }

    fn serialize_event(&mut self, e: &EventDef) -> u64 {
        let name_tok = self.tok_idx_for_span(e.name.span);
        let event_node = self.emit(NK_EVENT, [name_tok, 0, 0, 0, 0, 0, 0]);

        let fields_start = self.nodes.len() as u64;
        for field in &e.fields {
            let fname_tok = self.tok_idx_for_span(field.name.span);
            let ftype_node = self.serialize_type(&field.ty.node);
            self.emit(NK_STRUCT_FIELD, [fname_tok, ftype_node, 0, 0, 0, 0, 0]);
        }
        let fields_count = e.fields.len() as u64;
        self.backpatch(event_node, 2, fields_start);
        self.backpatch(event_node, 3, fields_count);

        event_node
    }

    fn serialize_type(&mut self, ty: &Type) -> u64 {
        match ty {
            Type::Field => self.emit(NK_TYPE_FIELD, [0; 7]),
            Type::XField => self.emit(NK_TYPE_XFIELD, [0; 7]),
            Type::Bool => self.emit(NK_TYPE_BOOL, [0; 7]),
            Type::U32 => self.emit(NK_TYPE_U32, [0; 7]),
            Type::Digest => self.emit(NK_TYPE_DIGEST, [0; 7]),
            Type::Array(inner, size) => {
                let inner_node = self.serialize_type(inner);
                let size_val = size.as_literal().unwrap_or(0);
                self.emit(NK_TYPE_ARRAY, [inner_node, size_val, 0, 0, 0, 0, 0])
            }
            Type::Tuple(types) => {
                let tuple_node = self.emit(NK_TYPE_TUPLE, [0, 0, 0, 0, 0, 0, 0]);
                let types_start = self.nodes.len() as u64;
                for t in types {
                    self.serialize_type(t);
                }
                let types_count = types.len() as u64;
                self.backpatch(tuple_node, 1, types_start);
                self.backpatch(tuple_node, 2, types_count);
                tuple_node
            }
            Type::Named(_path) => {
                // For named types, we store a pseudo token range
                // Since we can't easily look up the exact tokens, store 0s
                self.emit(NK_TYPE_NAMED, [0, 0, 0, 0, 0, 0, 0])
            }
        }
    }

    fn serialize_block(&mut self, block: &Block) -> u64 {
        let block_node = self.emit(NK_BLOCK, [0, 0, 0, 0, 0, 0, 0]);

        let stmts_start = self.nodes.len() as u64;
        for stmt in &block.stmts {
            self.serialize_stmt(stmt);
        }
        let stmts_count = block.stmts.len() as u64;

        let tail_node = if let Some(tail) = &block.tail_expr {
            self.serialize_expr(tail)
        } else {
            0
        };

        self.backpatch(block_node, 1, stmts_start);
        self.backpatch(block_node, 2, stmts_count);
        self.backpatch(block_node, 3, tail_node);

        block_node
    }

    fn serialize_stmt(&mut self, stmt: &Spanned<Stmt>) -> u64 {
        match &stmt.node {
            Stmt::Let { mutable, pattern, ty, init } => {
                let name_tok = match pattern {
                    Pattern::Name(n) => self.tok_idx_for_span(n.span),
                    Pattern::Tuple(names) => {
                        if let Some(first) = names.first() {
                            self.tok_idx_for_span(first.span)
                        } else {
                            0
                        }
                    }
                };
                let type_node = if let Some(t) = ty {
                    self.serialize_type(&t.node)
                } else {
                    0
                };
                let init_node = self.serialize_expr(init);
                let flags = if *mutable { 1 } else { 0 };
                self.emit(NK_LET, [name_tok, type_node, init_node, flags, 0, 0, 0])
            }
            Stmt::Assign { place, value } => {
                let place_node = self.serialize_place(place);
                let value_node = self.serialize_expr(value);
                self.emit(NK_ASSIGN, [place_node, value_node, 0, 0, 0, 0, 0])
            }
            Stmt::TupleAssign { names, value } => {
                // Serialize as assign with a tuple pattern
                let value_node = self.serialize_expr(value);
                let first_tok = if let Some(n) = names.first() {
                    self.tok_idx_for_span(n.span)
                } else {
                    0
                };
                self.emit(NK_ASSIGN, [first_tok, value_node, 0, 0, 0, 0, 0])
            }
            Stmt::If { cond, then_block, else_block } => {
                let cond_node = self.serialize_expr(cond);
                let then_node = self.serialize_block(&then_block.node);
                let else_node = if let Some(eb) = else_block {
                    self.serialize_block(&eb.node)
                } else {
                    0
                };
                self.emit(NK_IF, [cond_node, then_node, else_node, 0, 0, 0, 0])
            }
            Stmt::For { var, start, end, bound, body } => {
                let var_tok = self.tok_idx_for_span(var.span);
                let start_node = self.serialize_expr(start);
                let end_node = self.serialize_expr(end);
                let bound_val = bound.unwrap_or(0);
                let body_node = self.serialize_block(&body.node);
                self.emit(NK_FOR, [var_tok, start_node, end_node, bound_val, body_node, 0, 0])
            }
            Stmt::Return(value) => {
                let value_node = if let Some(v) = value {
                    self.serialize_expr(v)
                } else {
                    0
                };
                self.emit(NK_RETURN, [value_node, 0, 0, 0, 0, 0, 0])
            }
            Stmt::Expr(expr) => {
                let expr_node = self.serialize_expr(expr);
                self.emit(NK_EXPR_STMT, [expr_node, 0, 0, 0, 0, 0, 0])
            }
            Stmt::Reveal { event_name, fields } => {
                let name_tok = self.tok_idx_for_span(event_name.span);
                let reveal_node = self.emit(NK_REVEAL, [name_tok, 0, 0, 0, 0, 0, 0]);
                let fields_start = self.nodes.len() as u64;
                for (fname, fval) in fields {
                    let ft = self.tok_idx_for_span(fname.span);
                    let fv = self.serialize_expr(fval);
                    self.emit(NK_INIT_FIELD, [ft, fv, 0, 0, 0, 0, 0]);
                }
                let fields_count = fields.len() as u64;
                self.backpatch(reveal_node, 2, fields_start);
                self.backpatch(reveal_node, 3, fields_count);
                reveal_node
            }
            Stmt::Seal { event_name, fields } => {
                let name_tok = self.tok_idx_for_span(event_name.span);
                let seal_node = self.emit(NK_SEAL, [name_tok, 0, 0, 0, 0, 0, 0]);
                let fields_start = self.nodes.len() as u64;
                for (fname, fval) in fields {
                    let ft = self.tok_idx_for_span(fname.span);
                    let fv = self.serialize_expr(fval);
                    self.emit(NK_INIT_FIELD, [ft, fv, 0, 0, 0, 0, 0]);
                }
                let fields_count = fields.len() as u64;
                self.backpatch(seal_node, 2, fields_start);
                self.backpatch(seal_node, 3, fields_count);
                seal_node
            }
            Stmt::Asm { .. } => {
                let tok = self.tok_idx_for_span(stmt.span);
                self.emit(NK_ASM, [tok, 0, 0, 0, 0, 0, 0])
            }
            Stmt::Match { expr, arms } => {
                let expr_node = self.serialize_expr(expr);
                let match_node = self.emit(NK_MATCH, [expr_node, 0, 0, 0, 0, 0, 0]);
                let arms_start = self.nodes.len() as u64;
                for arm in arms {
                    self.serialize_match_arm(arm);
                }
                let arms_count = arms.len() as u64;
                self.backpatch(match_node, 2, arms_start);
                self.backpatch(match_node, 3, arms_count);
                match_node
            }
        }
    }

    fn serialize_match_arm(&mut self, arm: &MatchArm) -> u64 {
        let pattern_node = self.serialize_match_pattern(&arm.pattern);
        let body_node = self.serialize_block(&arm.body.node);
        self.emit(NK_MATCH_ARM, [pattern_node, body_node, 0, 0, 0, 0, 0])
    }

    fn serialize_match_pattern(&mut self, pattern: &Spanned<MatchPattern>) -> u64 {
        match &pattern.node {
            MatchPattern::Literal(lit) => match lit {
                Literal::Integer(n) => self.emit(NK_PAT_LIT, [*n, 0, 0, 0, 0, 0, 0]),
                Literal::Bool(b) => self.emit(NK_PAT_LIT, [if *b { 1 } else { 0 }, 0, 0, 0, 0, 0, 0]),
            },
            MatchPattern::Wildcard => self.emit(NK_PAT_WILDCARD, [0; 7]),
            MatchPattern::Struct { name, fields } => {
                let name_tok = self.tok_idx_for_span(name.span);
                let pat_node = self.emit(NK_PAT_STRUCT, [name_tok, 0, 0, 0, 0, 0, 0]);
                let fields_start = self.nodes.len() as u64;
                for field in fields {
                    let ft = self.tok_idx_for_span(field.field_name.span);
                    self.emit(NK_PAT_NAME, [ft, 0, 0, 0, 0, 0, 0]);
                }
                let fields_count = fields.len() as u64;
                self.backpatch(pat_node, 2, fields_start);
                self.backpatch(pat_node, 3, fields_count);
                pat_node
            }
        }
    }

    fn serialize_place(&mut self, place: &Spanned<Place>) -> u64 {
        match &place.node {
            Place::Var(_name) => {
                let tok = self.tok_idx_for_span(place.span);
                self.emit(NK_VAR, [tok, tok, 0, 0, 0, 0, 0])
            }
            Place::FieldAccess(base, field) => {
                let base_node = self.serialize_place(base);
                let field_tok = self.tok_idx_for_span(field.span);
                self.emit(NK_FIELD_ACCESS, [base_node, field_tok, 0, 0, 0, 0, 0])
            }
            Place::Index(base, index) => {
                let base_node = self.serialize_place(base);
                let index_node = self.serialize_expr(index);
                self.emit(NK_INDEX, [base_node, index_node, 0, 0, 0, 0, 0])
            }
        }
    }

    fn serialize_expr(&mut self, expr: &Spanned<Expr>) -> u64 {
        match &expr.node {
            Expr::Literal(Literal::Integer(n)) => {
                self.emit(NK_LIT_INT, [*n, 0, 0, 0, 0, 0, 0])
            }
            Expr::Literal(Literal::Bool(b)) => {
                self.emit(NK_LIT_BOOL, [if *b { 1 } else { 0 }, 0, 0, 0, 0, 0, 0])
            }
            Expr::Var(_) => {
                let tok = self.tok_idx_for_span(expr.span);
                self.emit(NK_VAR, [tok, tok, 0, 0, 0, 0, 0])
            }
            Expr::BinOp { op, lhs, rhs } => {
                let op_code = Self::binop_code(op);
                let lhs_node = self.serialize_expr(lhs);
                let rhs_node = self.serialize_expr(rhs);
                self.emit(NK_BINOP, [op_code, lhs_node, rhs_node, 0, 0, 0, 0])
            }
            Expr::Call { path, args, .. } => {
                let path_tok = self.tok_idx_for_span(path.span);
                // Count path tokens: parts + dots
                let num_parts = path.node.0.len();
                let path_end = path_tok + (num_parts as u64 * 2).saturating_sub(1);
                let call_node = self.emit(NK_CALL, [path_tok, path_end, 0, 0, 0, 0, 0]);
                let args_start = self.nodes.len() as u64;
                for arg in args {
                    self.serialize_expr(arg);
                }
                let args_count = args.len() as u64;
                self.backpatch(call_node, 3, args_start);
                self.backpatch(call_node, 4, args_count);
                call_node
            }
            Expr::FieldAccess { expr: base, field } => {
                let base_node = self.serialize_expr(base);
                let field_tok = self.tok_idx_for_span(field.span);
                self.emit(NK_FIELD_ACCESS, [base_node, field_tok, 0, 0, 0, 0, 0])
            }
            Expr::Index { expr: base, index } => {
                let base_node = self.serialize_expr(base);
                let index_node = self.serialize_expr(index);
                self.emit(NK_INDEX, [base_node, index_node, 0, 0, 0, 0, 0])
            }
            Expr::StructInit { path, fields } => {
                let path_tok = self.tok_idx_for_span(path.span);
                let si_node = self.emit(NK_STRUCT_INIT, [path_tok, 0, 0, 0, 0, 0, 0]);
                let fields_start = self.nodes.len() as u64;
                for (fname, fval) in fields {
                    let ft = self.tok_idx_for_span(fname.span);
                    let fv = self.serialize_expr(fval);
                    self.emit(NK_INIT_FIELD, [ft, fv, 0, 0, 0, 0, 0]);
                }
                let fields_count = fields.len() as u64;
                self.backpatch(si_node, 2, fields_start);
                self.backpatch(si_node, 3, fields_count);
                si_node
            }
            Expr::ArrayInit(elems) => {
                let arr_node = self.emit(NK_ARRAY_INIT, [0, 0, 0, 0, 0, 0, 0]);
                let elems_start = self.nodes.len() as u64;
                for elem in elems {
                    self.serialize_expr(elem);
                }
                let elems_count = elems.len() as u64;
                self.backpatch(arr_node, 1, elems_start);
                self.backpatch(arr_node, 2, elems_count);
                arr_node
            }
            Expr::Tuple(elems) => {
                let tup_node = self.emit(NK_TUPLE, [0, 0, 0, 0, 0, 0, 0]);
                let elems_start = self.nodes.len() as u64;
                for elem in elems {
                    self.serialize_expr(elem);
                }
                let elems_count = elems.len() as u64;
                self.backpatch(tup_node, 1, elems_start);
                self.backpatch(tup_node, 2, elems_count);
                tup_node
            }
        }
    }
}

/// Test source: the same small Trident program used by the lexer bench.
const TEST_SOURCE: &str = r#"program test

use vm.io.io

fn add(a: Field, b: Field) -> Field {
    let result: Field = a + b
    return result
}

fn main() {
    let x: Field = pub_read()
    let y: Field = 42
    let z: Field = add(x, y)
    pub_write(z)
}
"#;

fn main() {
    let source = TEST_SOURCE;

    // Lex
    let (tokens, _comments, diags) = Lexer::new(source, 0).tokenize();
    assert!(diags.is_empty(), "lex errors: {:?}", diags);

    let tok_count = tokens.len();
    let _tk_values: Vec<u64> = tokens.iter().map(|t| lexeme_to_tk(&t.node)).collect();

    // Build flat token array (stride 4: kind, start, end, int_val)
    let mut flat_tokens: Vec<u64> = Vec::new();
    for tok in &tokens {
        flat_tokens.push(lexeme_to_tk(&tok.node));
        flat_tokens.push(tok.span.start as u64);
        flat_tokens.push(tok.span.end as u64);
        let int_val = if let Lexeme::Integer(n) = &tok.node { *n } else { 0 };
        flat_tokens.push(int_val);
    }

    // Parse with Rust parser
    let file = trident::parse_source_silent(source, "test.tri").expect("parse failed");

    // Serialize AST to flat format
    let mut builder = AstBuilder::new(tokens);
    builder.serialize_file(&file);

    let node_count = builder.nodes.len();

    eprintln!("=== Parser Reference ===");
    eprintln!("Source: {} bytes", source.len());
    eprintln!("Tokens: {}", tok_count);
    eprintln!("AST nodes: {}", node_count);
    eprintln!();

    for (i, node) in builder.nodes.iter().enumerate() {
        eprintln!(
            "  [{:3}] kind={:2}  [{}, {}, {}, {}, {}, {}, {}]",
            i, node[0], node[1], node[2], node[3], node[4], node[5], node[6], node[7]
        );
    }
    eprintln!();

    // Generate .inputs format
    // Layout: tok_base=1000, ast_base=5000, err_base=8000, state_base=9000, stack_base=10000
    let tok_base: u64 = 1000;
    let ast_base: u64 = 5000;
    let err_base: u64 = 8000;
    let state_base: u64 = 9000;
    let stack_base: u64 = 10000;

    let mut vals: Vec<String> = Vec::new();

    // Parameters (6): tok_base, tok_count, ast_base, err_base, state_base, stack_base
    vals.push(tok_base.to_string());
    vals.push(tok_count.to_string());
    vals.push(ast_base.to_string());
    vals.push(err_base.to_string());
    vals.push(state_base.to_string());
    vals.push(stack_base.to_string());

    // Expected results (2): expected_node_count, expected_err_count
    vals.push(node_count.to_string());
    vals.push("0".to_string()); // expected 0 errors

    // Flat token data (tok_count * 4 values)
    for v in &flat_tokens {
        vals.push(v.to_string());
    }

    // Expected AST: first node kind (NK_FILE) for spot-check
    vals.push(builder.nodes[0][0].to_string());

    eprintln!("values: {}", vals.join(", "));

    // Benchmark
    for _ in 0..1000 {
        let _ = std::hint::black_box(trident::parse_source_silent(source, "test.tri"));
    }

    let n = 100_000u128;
    let start = Instant::now();
    for _ in 0..n {
        let _ = std::hint::black_box(trident::parse_source_silent(std::hint::black_box(source), "test.tri"));
    }
    println!("rust_ns: {}", start.elapsed().as_nanos() / n);
}
