//! Resolve source type spellings once, before canonical layouts reach TIR.
use crate::ast::*;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Nominal<'a> {
    pub aliases: &'a BTreeMap<String, String>,
    pub constants: &'a BTreeMap<String, u64>,
    pub parameters: BTreeSet<String>,
}
impl Nominal<'_> {
    fn name(&self, name: &str) -> String {
        self.aliases
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.into())
    }
    pub fn ty(&self, ty: &mut Type) {
        match ty {
            Type::Named(path) => {
                path.0 = self
                    .name(&path.as_dotted())
                    .split('.')
                    .map(str::to_string)
                    .collect()
            }
            Type::Array(inner, size) => {
                self.ty(inner);
                self.size(size);
            }
            Type::Tuple(parts) => {
                for part in parts {
                    self.ty(part);
                }
            }
            _ => {}
        }
    }
    fn size(&self, size: &mut ArraySize) {
        match size {
            ArraySize::Param(name) if !self.parameters.contains(name) => {
                if let Some(n) = self.constants.get(name) {
                    *size = ArraySize::Literal(*n);
                }
            }
            ArraySize::Add(a, b) | ArraySize::Mul(a, b) => {
                self.size(a);
                self.size(b);
            }
            _ => {}
        }
    }
    pub fn file(&mut self, file: &mut File) {
        for declaration in &mut file.declarations {
            match declaration {
                Declaration::PubInput(t) | Declaration::PubOutput(t) | Declaration::SecInput(t) => {
                    self.ty(&mut t.node)
                }
                Declaration::SecRam(slots) => {
                    for (_, t) in slots {
                        self.ty(&mut t.node);
                    }
                }
            }
        }
        for item in &mut file.items {
            match &mut item.node {
                Item::Struct(s) => {
                    for f in &mut s.fields {
                        self.ty(&mut f.ty.node);
                    }
                    s.name.node = self.name(&s.name.node);
                }
                Item::Event(e) => {
                    for f in &mut e.fields {
                        self.ty(&mut f.ty.node);
                    }
                }
                Item::Const(c) => {
                    self.ty(&mut c.ty.node);
                    self.expr(&mut c.value.node);
                }
                Item::Fn(f) => {
                    self.parameters = f.type_params.iter().map(|p| p.node.clone()).collect();
                    for p in &mut f.params {
                        self.ty(&mut p.ty.node);
                    }
                    if let Some(t) = &mut f.return_ty {
                        self.ty(&mut t.node);
                    }
                    if let Some(b) = &mut f.body {
                        self.block(&mut b.node);
                    }
                    self.parameters.clear();
                }
            }
        }
    }
    fn block(&self, b: &mut Block) {
        for stmt in &mut b.stmts {
            match &mut stmt.node {
                Stmt::Let { ty, init, .. } => {
                    if let Some(t) = ty {
                        self.ty(&mut t.node);
                    }
                    self.expr(&mut init.node);
                }
                Stmt::Assign { place, value } => {
                    self.place(&mut place.node);
                    self.expr(&mut value.node);
                }
                Stmt::TupleAssign { value, .. } | Stmt::Expr(value) => self.expr(&mut value.node),
                Stmt::Return(value) => {
                    if let Some(e) = value {
                        self.expr(&mut e.node);
                    }
                }
                Stmt::If {
                    cond,
                    then_block,
                    else_block,
                } => {
                    self.expr(&mut cond.node);
                    self.block(&mut then_block.node);
                    if let Some(b) = else_block {
                        self.block(&mut b.node);
                    }
                }
                Stmt::For {
                    start, end, body, ..
                } => {
                    self.expr(&mut start.node);
                    self.expr(&mut end.node);
                    self.block(&mut body.node);
                }
                Stmt::Match { expr, arms } => {
                    self.expr(&mut expr.node);
                    for arm in arms {
                        if let MatchPattern::Struct { name, .. } = &mut arm.pattern.node {
                            name.node = self.name(&name.node);
                        }
                        self.block(&mut arm.body.node);
                    }
                }
                Stmt::Reveal { fields, .. } | Stmt::Seal { fields, .. } => {
                    for (_, e) in fields {
                        self.expr(&mut e.node);
                    }
                }
                Stmt::Asm { .. } => {}
            }
        }
        if let Some(e) = &mut b.tail_expr {
            self.expr(&mut e.node);
        }
    }
    fn place(&self, p: &mut Place) {
        match p {
            Place::Var(_) => {}
            Place::FieldAccess(p, _) => self.place(&mut p.node),
            Place::Index(p, e) => {
                self.place(&mut p.node);
                self.expr(&mut e.node);
            }
        }
    }
    fn expr(&self, e: &mut Expr) {
        match e {
            Expr::Literal(_) | Expr::Var(_) => {}
            Expr::BinOp { lhs, rhs, .. } => {
                self.expr(&mut lhs.node);
                self.expr(&mut rhs.node);
            }
            Expr::Call { args, .. } | Expr::ArrayInit(args) | Expr::Tuple(args) => {
                for e in args {
                    self.expr(&mut e.node);
                }
            }
            Expr::FieldAccess { expr, .. } => self.expr(&mut expr.node),
            Expr::Index { expr, index } => {
                self.expr(&mut expr.node);
                self.expr(&mut index.node);
            }
            Expr::StructInit { path, fields } => {
                path.node.0 = self
                    .name(&path.node.as_dotted())
                    .split('.')
                    .map(str::to_string)
                    .collect();
                for (_, e) in fields {
                    self.expr(&mut e.node);
                }
            }
        }
    }
}
