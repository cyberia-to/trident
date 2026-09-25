//! Visit types and calls, including bodies deferred for generic specialization.
use super::*;

pub(crate) trait SurfaceVisitor {
    fn ty(&mut self, ty: &Type);
    fn call(&mut self, name: &str);
    fn expression(&mut self, _: &Expr) {}
    fn destination(&mut self, _: &Place) {}
}

pub(crate) fn item(item: &Item, visitor: &mut impl SurfaceVisitor) {
    match item {
        Item::Fn(f) => {
            for p in &f.params {
                visitor.ty(&p.ty.node);
            }
            if let Some(t) = &f.return_ty {
                visitor.ty(&t.node);
            }
            if let Some(b) = &f.body {
                block(&b.node, visitor);
            }
        }
        Item::Struct(s) => {
            for f in &s.fields {
                visitor.ty(&f.ty.node);
            }
        }
        Item::Event(e) => {
            for f in &e.fields {
                visitor.ty(&f.ty.node);
            }
        }
        Item::Const(c) => {
            visitor.ty(&c.ty.node);
            expr(&c.value.node, visitor);
        }
    }
}

pub(crate) fn declarations(file: &File, visitor: &mut impl SurfaceVisitor) {
    for d in &file.declarations {
        match d {
            Declaration::PubInput(t) | Declaration::PubOutput(t) | Declaration::SecInput(t) => {
                visitor.ty(&t.node)
            }
            Declaration::SecRam(slots) => {
                for (_, t) in slots {
                    visitor.ty(&t.node);
                }
            }
        }
    }
}

fn block(b: &Block, v: &mut impl SurfaceVisitor) {
    for s in &b.stmts {
        match &s.node {
            Stmt::Let { ty, init, .. } => {
                if let Some(t) = ty {
                    v.ty(&t.node);
                }
                expr(&init.node, v);
            }
            Stmt::Assign { place: p, value } => {
                place(&p.node, v);
                expr(&value.node, v);
            }
            Stmt::TupleAssign { value, .. } | Stmt::Expr(value) => expr(&value.node, v),
            Stmt::Return(value) => {
                if let Some(e) = value {
                    expr(&e.node, v);
                }
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                expr(&cond.node, v);
                block(&then_block.node, v);
                if let Some(b) = else_block {
                    block(&b.node, v);
                }
            }
            Stmt::For {
                start, end, body, ..
            } => {
                expr(&start.node, v);
                expr(&end.node, v);
                block(&body.node, v);
            }
            Stmt::Match { expr: e, arms } => {
                expr(&e.node, v);
                for a in arms {
                    if let MatchPattern::Struct { name, .. } = &a.pattern.node {
                        v.ty(&Type::Named(ModulePath(
                            name.node.split('.').map(str::to_string).collect(),
                        )));
                    }
                    block(&a.body.node, v);
                }
            }
            Stmt::Reveal { fields, .. } | Stmt::Seal { fields, .. } => {
                for (_, e) in fields {
                    expr(&e.node, v);
                }
            }
            Stmt::Asm { .. } => {}
        }
    }
    if let Some(e) = &b.tail_expr {
        expr(&e.node, v);
    }
}

fn place(p: &Place, v: &mut impl SurfaceVisitor) {
    v.destination(p);
    match p {
        Place::Var(_) => {}
        Place::FieldAccess(p, _) => place(&p.node, v),
        Place::Index(p, e) => {
            place(&p.node, v);
            expr(&e.node, v);
        }
    }
}

fn expr(e: &Expr, v: &mut impl SurfaceVisitor) {
    v.expression(e);
    match e {
        Expr::Literal(_) | Expr::Var(_) => {}
        Expr::BinOp { lhs, rhs, .. } => {
            expr(&lhs.node, v);
            expr(&rhs.node, v);
        }
        Expr::Call { path, args, .. } => {
            v.call(&path.node.as_dotted());
            for a in args {
                expr(&a.node, v);
            }
        }
        Expr::FieldAccess { expr: e, .. } => expr(&e.node, v),
        Expr::Index { expr: e, index } => {
            expr(&e.node, v);
            expr(&index.node, v);
        }
        Expr::StructInit { path, fields } => {
            v.ty(&Type::Named(path.node.clone()));
            for (_, e) in fields {
                expr(&e.node, v);
            }
        }
        Expr::ArrayInit(elements) | Expr::Tuple(elements) => {
            for e in elements {
                expr(&e.node, v);
            }
        }
    }
}

impl Type {
    /// Whether the spelling contains Noun (named types require resolution).
    pub(crate) fn contains_noun(&self) -> bool {
        match self {
            Self::Noun => true,
            Self::Array(t, _) => t.contains_noun(),
            Self::Tuple(ts) => ts.iter().any(Self::contains_noun),
            _ => false,
        }
    }
}
