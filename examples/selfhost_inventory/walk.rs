use serde::Serialize;
use std::collections::BTreeMap;
use trident::ast::*;
use trident::span::{Span, Spanned};

#[derive(Serialize)]
pub struct Occurrence {
    pub count: usize,
    pub first_line: usize,
}

#[derive(Serialize)]
pub struct Module {
    path: String,
    source_blake3: String,
    pub source_bytes: usize,
    pub source_lines: usize,
    pub imports: Vec<String>,
    pub functions: Vec<String>,
    pub features: BTreeMap<String, Occurrence>,
    /// Syntactic call paths; this inventory does not infer callee reachability.
    calls: BTreeMap<String, usize>,
    intrinsic_declarations: BTreeMap<String, String>,
    loop_bounds: BTreeMap<u64, usize>,
}

struct Visitor {
    line_starts: Vec<usize>,
    module: Module,
}

pub fn inspect(file: &File, source: &str, path: &str) -> Module {
    let mut imports: Vec<_> = file.uses.iter().map(|u| u.node.as_dotted()).collect();
    imports.sort();
    imports.dedup();
    let mut v = Visitor {
        line_starts: std::iter::once(0)
            .chain(
                source
                    .bytes()
                    .enumerate()
                    .filter_map(|(i, b)| (b == b'\n').then_some(i + 1)),
            )
            .collect(),
        module: Module {
            path: path.to_owned(),
            source_blake3: blake3::hash(source.as_bytes()).to_hex().to_string(),
            source_bytes: source.len(),
            source_lines: source.lines().count(),
            imports,
            functions: Vec::new(),
            features: BTreeMap::new(),
            calls: BTreeMap::new(),
            intrinsic_declarations: BTreeMap::new(),
            loop_bounds: BTreeMap::new(),
        },
    };
    v.hit(
        match file.kind {
            FileKind::Module => "file.module",
            FileKind::Program => "file.program",
        },
        file.name.span,
    );
    for u in &file.uses {
        v.hit("item.use", u.span);
    }
    for declaration in &file.declarations {
        match declaration {
            Declaration::PubInput(t) => {
                v.hit("declaration.pub_input", t.span);
                v.ty(t);
            }
            Declaration::PubOutput(t) => {
                v.hit("declaration.pub_output", t.span);
                v.ty(t);
            }
            Declaration::SecInput(t) => {
                v.hit("declaration.sec_input", t.span);
                v.ty(t);
            }
            Declaration::SecRam(slots) => {
                v.hit("declaration.sec_ram", file.name.span);
                for (_, t) in slots {
                    v.ty(t);
                }
            }
        }
    }
    for item in &file.items {
        v.item(item);
    }
    v.module.functions.sort();
    v.module
}

impl Visitor {
    fn hit(&mut self, feature: &str, span: Span) {
        let line = self
            .line_starts
            .partition_point(|&start| start <= span.start as usize);
        let o = self
            .module
            .features
            .entry(feature.to_owned())
            .or_insert(Occurrence {
                count: 0,
                first_line: line,
            });
        o.count += 1;
        o.first_line = o.first_line.min(line);
    }

    fn cfg(&mut self, cfg: &Option<Spanned<String>>) {
        if let Some(cfg) = cfg {
            self.hit("attribute.cfg", cfg.span);
        }
    }

    fn item(&mut self, item: &Spanned<Item>) {
        match &item.node {
            Item::Fn(f) => {
                self.hit("item.fn", item.span);
                self.module.functions.push(f.name.node.clone());
                self.cfg(&f.cfg);
                if f.is_pub {
                    self.hit("visibility.pub", item.span);
                }
                if f.is_test {
                    self.hit("attribute.test", item.span);
                }
                if f.is_pure {
                    self.hit("attribute.pure", item.span);
                }
                for p in &f.requires {
                    self.hit("attribute.requires", p.span);
                }
                for p in &f.ensures {
                    self.hit("attribute.ensures", p.span);
                }
                if let Some(i) = &f.intrinsic {
                    self.hit("attribute.intrinsic", i.span);
                    self.module
                        .intrinsic_declarations
                        .insert(f.name.node.clone(), intrinsic_name(&i.node).to_owned());
                }
                for p in &f.type_params {
                    self.hit("generic.parameter", p.span);
                }
                for p in &f.params {
                    self.ty(&p.ty);
                }
                if let Some(t) = &f.return_ty {
                    self.ty(t);
                } else {
                    self.hit("return.implicit_unit", f.name.span);
                }
                if let Some(b) = &f.body {
                    self.block(b);
                } else {
                    self.hit("fn.declaration_only", f.name.span);
                }
            }
            Item::Const(c) => {
                self.hit("item.const", item.span);
                self.cfg(&c.cfg);
                if c.is_pub {
                    self.hit("visibility.pub", item.span);
                }
                self.ty(&c.ty);
                self.expr(&c.value);
            }
            Item::Struct(s) => {
                self.hit("item.struct", item.span);
                self.cfg(&s.cfg);
                if s.is_pub {
                    self.hit("visibility.pub", item.span);
                }
                for f in &s.fields {
                    if f.is_pub {
                        self.hit("visibility.pub", f.name.span);
                    }
                    self.ty(&f.ty);
                }
            }
            Item::Event(e) => {
                self.hit("item.event", item.span);
                self.cfg(&e.cfg);
                for f in &e.fields {
                    self.ty(&f.ty);
                }
            }
        }
    }

    fn ty(&mut self, ty: &Spanned<Type>) {
        self.type_node(&ty.node, ty.span);
    }

    fn type_node(&mut self, ty: &Type, span: Span) {
        let name = match ty {
            Type::Noun => "type.Noun",
            Type::Field => "type.Field",
            Type::U32 => "type.U32",
            Type::Bool => "type.Bool",
            Type::XField => "type.XField",
            Type::Digest => "type.Digest",
            Type::Array(inner, size) => {
                self.type_node(inner, span);
                self.size(size, span);
                "type.Array"
            }
            Type::Tuple(types) => {
                for t in types {
                    self.type_node(t, span);
                }
                "type.Tuple"
            }
            Type::Named(_) => "type.Named",
        };
        self.hit(name, span);
    }

    fn size(&mut self, size: &ArraySize, span: Span) {
        let name = match size {
            ArraySize::Literal(_) => "size.literal",
            ArraySize::Param(_) => "size.parameter",
            ArraySize::Add(a, b) => {
                self.size(a, span);
                self.size(b, span);
                "size.add"
            }
            ArraySize::Mul(a, b) => {
                self.size(a, span);
                self.size(b, span);
                "size.mul"
            }
        };
        self.hit(name, span);
    }

    fn block(&mut self, block: &Spanned<Block>) {
        for stmt in &block.node.stmts {
            self.stmt(stmt);
        }
        if let Some(tail) = &block.node.tail_expr {
            self.hit("block.tail", tail.span);
            self.expr(tail);
        }
    }

    fn stmt(&mut self, stmt: &Spanned<Stmt>) {
        let name = match &stmt.node {
            Stmt::Let {
                mutable,
                pattern,
                ty,
                init,
            } => {
                if *mutable {
                    self.hit("binding.mutable", stmt.span);
                }
                self.hit(
                    match pattern {
                        Pattern::Name(_) => "binding.name",
                        Pattern::Tuple(_) => "binding.tuple",
                    },
                    stmt.span,
                );
                if let Some(t) = ty {
                    self.ty(t);
                } else {
                    self.hit("binding.inferred_type", stmt.span);
                }
                self.expr(init);
                "stmt.let"
            }
            Stmt::Assign { place, value } => {
                self.place(place);
                self.expr(value);
                "stmt.assign"
            }
            Stmt::TupleAssign { value, .. } => {
                self.expr(value);
                "stmt.tuple_assign"
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                self.expr(cond);
                self.block(then_block);
                if let Some(b) = else_block {
                    self.block(b);
                }
                "stmt.if"
            }
            Stmt::For {
                start,
                end,
                bound,
                body,
                ..
            } => {
                self.expr(start);
                self.expr(end);
                self.block(body);
                if let Some(n) = bound {
                    self.hit("loop.bounded", stmt.span);
                    *self.module.loop_bounds.entry(*n).or_default() += 1;
                } else {
                    self.hit("loop.no_explicit_bound", stmt.span);
                }
                "stmt.for"
            }
            Stmt::Expr(e) => {
                self.expr(e);
                "stmt.expr"
            }
            Stmt::Return(e) => {
                if let Some(e) = e {
                    self.expr(e);
                }
                "stmt.return"
            }
            Stmt::Reveal { fields, .. } => {
                for (_, e) in fields {
                    self.expr(e);
                }
                "stmt.reveal"
            }
            Stmt::Seal { fields, .. } => {
                for (_, e) in fields {
                    self.expr(e);
                }
                "stmt.seal"
            }
            Stmt::Asm { .. } => "stmt.asm_opaque",
            Stmt::Match { expr, arms } => {
                self.expr(expr);
                for arm in arms {
                    self.pattern(&arm.pattern);
                    self.block(&arm.body);
                }
                "stmt.match"
            }
        };
        self.hit(name, stmt.span);
    }

    fn pattern(&mut self, pattern: &Spanned<MatchPattern>) {
        let name = match &pattern.node {
            MatchPattern::Literal(l) => {
                self.literal(l, pattern.span);
                "pattern.literal"
            }
            MatchPattern::Wildcard => "pattern.wildcard",
            MatchPattern::Struct { fields, .. } => {
                for f in fields {
                    match &f.pattern.node {
                        FieldPattern::Binding(_) => {
                            self.hit("pattern.field_binding", f.pattern.span)
                        }
                        FieldPattern::Literal(l) => self.literal(l, f.pattern.span),
                        FieldPattern::Wildcard => {
                            self.hit("pattern.field_wildcard", f.pattern.span)
                        }
                    }
                }
                "pattern.struct"
            }
        };
        self.hit(name, pattern.span);
    }

    fn place(&mut self, place: &Spanned<Place>) {
        let name = match &place.node {
            Place::Var(_) => "place.variable",
            Place::FieldAccess(p, _) => {
                self.place(p);
                "place.field"
            }
            Place::Index(p, e) => {
                self.place(p);
                self.expr(e);
                "place.index"
            }
        };
        self.hit(name, place.span);
    }

    fn literal(&mut self, literal: &Literal, span: Span) {
        self.hit(
            match literal {
                Literal::Integer(_) => "literal.integer",
                Literal::Bool(_) => "literal.bool",
            },
            span,
        );
    }

    fn expr(&mut self, expr: &Spanned<Expr>) {
        let name = match &expr.node {
            Expr::Literal(l) => {
                self.literal(l, expr.span);
                "expr.literal"
            }
            Expr::Var(_) => "expr.variable",
            Expr::BinOp { op, lhs, rhs } => {
                self.hit(&format!("operator.{}", op.as_str()), expr.span);
                self.expr(lhs);
                self.expr(rhs);
                "expr.binary"
            }
            Expr::Call {
                path,
                generic_args,
                args,
            } => {
                *self.module.calls.entry(path.node.as_dotted()).or_default() += 1;
                for s in generic_args {
                    self.hit("generic.argument", s.span);
                    self.size(&s.node, s.span);
                }
                for a in args {
                    self.expr(a);
                }
                "expr.call"
            }
            Expr::FieldAccess { expr, .. } => {
                self.expr(expr);
                "expr.field"
            }
            Expr::Index { expr, index } => {
                self.expr(expr);
                self.expr(index);
                "expr.index"
            }
            Expr::StructInit { fields, .. } => {
                for (_, e) in fields {
                    self.expr(e);
                }
                "expr.struct"
            }
            Expr::ArrayInit(values) => {
                for e in values {
                    self.expr(e);
                }
                "expr.array"
            }
            Expr::Tuple(values) => {
                for e in values {
                    self.expr(e);
                }
                "expr.tuple"
            }
        };
        self.hit(name, expr.span);
    }
}
