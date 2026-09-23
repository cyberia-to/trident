//! Concrete AST instances retain their defining module's lexical environment.
use super::*;

pub(crate) fn concrete_function(
    function: &FnDef,
    args: &[u64],
    name: String,
    constants: &BTreeMap<String, u64>,
) -> Result<FnDef, String> {
    if function.type_params.len() != args.len() {
        return Err("generic arity mismatch".into());
    }
    let mut copy = function.clone();
    let mut subs = constants.clone();
    for (param, value) in function.type_params.iter().zip(args) {
        subs.insert(param.node.clone(), *value);
    }
    copy.name.node = name;
    copy.type_params.clear();
    let mut visitor = Visitor {
        subs,
        calls: None,
        function: String::new(),
    };
    for param in &mut copy.params {
        visitor.ty(&mut param.ty.node)?;
    }
    if let Some(ty) = &mut copy.return_ty {
        visitor.ty(&mut ty.node)?;
    }
    let mut visible: BTreeMap<_, _> = function
        .type_params
        .iter()
        .zip(args)
        .map(|(param, value)| (param.node.clone(), *value))
        .collect();
    for param in &copy.params {
        visible.remove(&param.name.node);
    }
    if let Some(body) = &mut copy.body {
        visitor.block(&mut body.node, &mut visible)?;
    }
    Ok(copy)
}

pub(crate) fn rewrite_calls(
    file: &mut File,
    replacements: &BTreeMap<(String, u32, u32), ModulePath>,
) -> Result<(), String> {
    for item in &mut file.items {
        if let Item::Fn(function) = &mut item.node {
            if function.type_params.is_empty() {
                let mut visitor = Visitor {
                    subs: BTreeMap::new(),
                    calls: Some(replacements),
                    function: function.name.node.clone(),
                };
                if let Some(body) = &mut function.body {
                    visitor.block(&mut body.node, &mut BTreeMap::new())?;
                }
            }
        }
    }
    Ok(())
}

struct Visitor<'a> {
    subs: BTreeMap<String, u64>,
    calls: Option<&'a BTreeMap<(String, u32, u32), ModulePath>>,
    function: String,
}
impl Visitor<'_> {
    fn size(&self, size: &mut ArraySize) -> Result<(), String> {
        match size {
            ArraySize::Param(name) => {
                if let Some(n) = self.subs.get(name) {
                    *size = ArraySize::Literal(*n);
                }
            }
            ArraySize::Add(a, b) | ArraySize::Mul(a, b) => {
                self.size(a)?;
                self.size(b)?;
            }
            _ => {}
        }
        let value = match size {
            ArraySize::Add(a, b) => match (a.as_literal(), b.as_literal()) {
                (Some(a), Some(b)) => {
                    Some(a.checked_add(b).ok_or("generic size addition overflow")?)
                }
                _ => None,
            },
            ArraySize::Mul(a, b) => match (a.as_literal(), b.as_literal()) {
                (Some(a), Some(b)) => Some(
                    a.checked_mul(b)
                        .ok_or("generic size multiplication overflow")?,
                ),
                _ => None,
            },
            _ => None,
        };
        if let Some(value) = value {
            *size = ArraySize::Literal(value);
        }
        Ok(())
    }
    fn ty(&self, ty: &mut Type) -> Result<(), String> {
        match ty {
            Type::Array(inner, size) => {
                self.ty(inner)?;
                self.size(size)?;
            }
            Type::Tuple(types) => {
                for ty in types {
                    self.ty(ty)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn expr(
        &mut self,
        expression: &mut Spanned<Expr>,
        visible: &BTreeMap<String, u64>,
    ) -> Result<(), String> {
        if let Expr::Var(name) = &expression.node {
            if let Some(n) = visible.get(name) {
                expression.node = Expr::Literal(Literal::Integer(*n));
            }
        }
        match &mut expression.node {
            Expr::Call {
                path,
                generic_args,
                args,
            } => {
                for arg in args {
                    self.expr(arg, visible)?;
                }
                for size in generic_args.iter_mut() {
                    self.size(&mut size.node)?;
                }
                if let Some(path_new) = self.calls.and_then(|calls| {
                    calls.get(&(
                        self.function.clone(),
                        expression.span.start,
                        expression.span.end,
                    ))
                }) {
                    path.node = path_new.clone();
                    generic_args.clear();
                }
            }
            Expr::BinOp { lhs, rhs, .. } => {
                self.expr(lhs, visible)?;
                self.expr(rhs, visible)?;
            }
            Expr::FieldAccess { expr, .. } => self.expr(expr, visible)?,
            Expr::Index { expr, index } => {
                self.expr(expr, visible)?;
                self.expr(index, visible)?;
            }
            Expr::Tuple(values) | Expr::ArrayInit(values) => {
                for value in values {
                    self.expr(value, visible)?;
                }
            }
            Expr::StructInit { fields, .. } => {
                for (_, value) in fields {
                    self.expr(value, visible)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn place(&mut self, place: &mut Place, visible: &BTreeMap<String, u64>) -> Result<(), String> {
        match place {
            Place::FieldAccess(base, _) => self.place(&mut base.node, visible)?,
            Place::Index(base, index) => {
                self.place(&mut base.node, visible)?;
                self.expr(index, visible)?;
            }
            _ => {}
        }
        Ok(())
    }
    fn block(
        &mut self,
        body: &mut Block,
        visible: &mut BTreeMap<String, u64>,
    ) -> Result<(), String> {
        for statement in &mut body.stmts {
            match &mut statement.node {
                Stmt::Let {
                    pattern, ty, init, ..
                } => {
                    self.expr(init, visible)?;
                    if let Some(ty) = ty {
                        self.ty(&mut ty.node)?;
                    }
                    match pattern {
                        Pattern::Name(name) => {
                            visible.remove(&name.node);
                        }
                        Pattern::Tuple(names) => {
                            for name in names {
                                visible.remove(&name.node);
                            }
                        }
                    }
                }
                Stmt::Assign { place, value } => {
                    self.place(&mut place.node, visible)?;
                    self.expr(value, visible)?;
                }
                Stmt::TupleAssign { value, .. } | Stmt::Expr(value) => self.expr(value, visible)?,
                Stmt::Return(value) => {
                    if let Some(value) = value {
                        self.expr(value, visible)?;
                    }
                }
                Stmt::If {
                    cond,
                    then_block,
                    else_block,
                } => {
                    self.expr(cond, visible)?;
                    self.block(&mut then_block.node, &mut visible.clone())?;
                    if let Some(body) = else_block {
                        self.block(&mut body.node, &mut visible.clone())?;
                    }
                }
                Stmt::For {
                    var,
                    start,
                    end,
                    body,
                    ..
                } => {
                    self.expr(start, visible)?;
                    self.expr(end, visible)?;
                    let mut inner = visible.clone();
                    inner.remove(&var.node);
                    self.block(&mut body.node, &mut inner)?;
                }
                Stmt::Reveal { fields, .. } | Stmt::Seal { fields, .. } => {
                    for (_, value) in fields {
                        self.expr(value, visible)?;
                    }
                }
                Stmt::Match { expr, arms } => {
                    self.expr(expr, visible)?;
                    for arm in arms {
                        let mut inner = visible.clone();
                        if let MatchPattern::Struct { fields, .. } = &arm.pattern.node {
                            for field in fields {
                                if let FieldPattern::Binding(name) = &field.pattern.node {
                                    inner.remove(name);
                                }
                            }
                        }
                        self.block(&mut arm.body.node, &mut inner)?;
                    }
                }
                Stmt::Asm { .. } => {}
            }
        }
        if let Some(tail) = &mut body.tail_expr {
            self.expr(tail, visible)?;
        }
        Ok(())
    }
}
