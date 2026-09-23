use super::*;

pub(super) fn contract_expr(text: &str) -> Result<Expr, String> {
    let source = format!("module contract\nfn predicate() {{ {text} }}");
    let file = crate::parse_source(&source, "contract.tri")
        .map_err(|_| "cannot parse contract predicate".to_string())?;
    if file.items.len() != 1 {
        return Err("contract must contain one expression".into());
    }
    let Item::Fn(f) = &file.items[0].node else {
        return Err("invalid contract".into());
    };
    let body = &f.body.as_ref().unwrap().node;
    if !body.stmts.is_empty() {
        return Err("contract must be one pure scalar expression".into());
    }
    body.tail_expr
        .as_ref()
        .map(|e| e.node.clone())
        .ok_or("missing contract expression".into())
}

pub(super) fn coverage_one(func: &FnDef, file: &File, zero_is_true: bool) -> Result<(), String> {
    fn expr(
        e: &Expr,
        file: &File,
        contract: bool,
        names: &std::collections::BTreeSet<String>,
    ) -> Result<(), String> {
        match e {
            Expr::Literal(_) => Ok(()),
            Expr::Var(name) if names.contains(name) => Ok(()),
            Expr::BinOp {
                op: BinOp::Add | BinOp::Mul | BinOp::Eq | BinOp::Lt,
                lhs,
                rhs,
            } => {
                expr(&lhs.node, file, contract, names)?;
                expr(&rhs.node, file, contract, names)
            }
            Expr::Call {
                path,
                args,
                generic_args,
            } => {
                if !generic_args.is_empty() {
                    return Err("generic scalar call not modeled".into());
                }
                let name = path.node.as_dotted();
                if let Some(function) = file.items.iter().find_map(|item| match &item.node {
                    Item::Fn(f) if f.name.node == name => Some(f),
                    _ => None,
                }) {
                    if contract {
                        return Err("helper calls in contracts are not modeled".into());
                    }
                    if function.params.len() != args.len() {
                        return Err("wrong helper arity".into());
                    }
                    for arg in args {
                        expr(&arg.node, file, contract, names)?;
                    }
                    return Ok(());
                }
                let pure = matches!(name.as_str(), "as_field" | "sub" | "neg");
                let allowed = pure
                    || (!contract
                        && matches!(
                            name.as_str(),
                            "pub_read" | "divine" | "pub_write" | "assert" | "assert_eq" | "as_u32"
                        ));
                if !allowed
                    || file
                        .items
                        .iter()
                        .any(|i| matches!(&i.node, Item::Fn(f) if f.name.node == name))
                {
                    return Err(format!("unmodeled call {name}"));
                }
                let arity = match name.as_str() {
                    "pub_read" | "divine" => 0,
                    "sub" | "assert_eq" => 2,
                    _ => 1,
                };
                if args.len() != arity {
                    return Err("wrong scalar builtin arity".into());
                }
                for a in args {
                    expr(&a.node, file, contract, names)?;
                }
                Ok(())
            }
            _ => Err("unmodeled scalar/aggregate expression".into()),
        }
    }
    if func
        .return_ty
        .as_ref()
        .is_some_and(|t| !matches!(t.node, Type::Field | Type::U32 | Type::Bool))
    {
        return Err("aggregate return".into());
    }
    if func
        .params
        .iter()
        .any(|p| !matches!(p.ty.node, Type::Field | Type::U32 | Type::Bool))
    {
        return Err("aggregate parameter".into());
    }
    let mut names: std::collections::BTreeSet<String> =
        func.params.iter().map(|p| p.name.node.clone()).collect();
    for c in &func.requires {
        expr(&contract_expr(&c.node)?, file, true, &names)?;
    }
    let Some(body) = &func.body else {
        return Err("no function body".into());
    };
    fn block(
        body: &Block,
        file: &File,
        names: &mut std::collections::BTreeSet<String>,
        zero_is_true: bool,
    ) -> Result<bool, String> {
        for statement in &body.stmts {
            match &statement.node {
                Stmt::Let {
                    pattern: Pattern::Name(name),
                    init,
                    ty,
                    ..
                } => {
                    if ty
                        .as_ref()
                        .is_some_and(|t| !matches!(t.node, Type::Field | Type::U32 | Type::Bool))
                    {
                        return Err("aggregate binding".into());
                    }
                    expr(&init.node, file, false, names)?;
                    names.insert(name.node.clone());
                }
                Stmt::Assign { place, value } => {
                    match &place.node {
                        Place::Var(name) if names.contains(name) => {}
                        _ => return Err("unmodeled mutation".into()),
                    }
                    expr(&value.node, file, false, names)?;
                }
                Stmt::Expr(value) => expr(&value.node, file, false, names)?,
                Stmt::Return(value) => {
                    if let Some(value) = value {
                        expr(&value.node, file, false, names)?;
                    }
                    return Ok(true);
                }
                Stmt::If {
                    cond,
                    then_block,
                    else_block,
                } => {
                    expr(&cond.node, file, false, names)?;
                    let literal = match cond.node {
                        Expr::Literal(Literal::Bool(value)) => Some(value),
                        Expr::Literal(Literal::Integer(value)) => Some(if zero_is_true {
                            value % GOLDILOCKS_P == 0
                        } else {
                            value % GOLDILOCKS_P != 0
                        }),
                        _ => None,
                    };
                    let then_returns = if literal != Some(false) {
                        block(&then_block.node, file, &mut names.clone(), zero_is_true)?
                    } else {
                        false
                    };
                    let else_returns = if literal != Some(true) {
                        match else_block {
                            Some(body) => {
                                block(&body.node, file, &mut names.clone(), zero_is_true)?
                            }
                            None => false,
                        }
                    } else {
                        false
                    };
                    if match literal {
                        Some(true) => then_returns,
                        Some(false) => else_returns,
                        None => then_returns && else_returns,
                    } {
                        return Ok(true);
                    }
                }
                _ => return Err("unmodeled control flow, mutation or event".into()),
            }
        }
        if let Some(tail) = &body.tail_expr {
            expr(&tail.node, file, false, names)?;
        }
        Ok(false)
    }
    block(&body.node, file, &mut names, zero_is_true)?;
    if func.return_ty.is_some() {
        names.insert("result".into());
    }
    for c in &func.ensures {
        expr(&contract_expr(&c.node)?, file, true, &names)?;
    }
    Ok(())
}

impl ConstraintSystem {
    /// Disjoint symbolic namespaces for independent functions/modules.
    pub fn append_independent(&mut self, mut other: Self, namespace: &str) {
        fn value(v: &mut SymValue, ns: &str) {
            match v {
                SymValue::Var(v) => v.name = format!("{ns}_{}", v.name),
                SymValue::Add(a, b)
                | SymValue::Mul(a, b)
                | SymValue::Sub(a, b)
                | SymValue::Eq(a, b)
                | SymValue::Lt(a, b) => {
                    value(a, ns);
                    value(b, ns);
                }
                SymValue::Neg(a) | SymValue::Inv(a) | SymValue::FieldAccess(a, _) => value(a, ns),
                SymValue::Hash(v, _) => {
                    for x in v {
                        value(x, ns);
                    }
                }
                SymValue::Ite(a, b, c) => {
                    value(a, ns);
                    value(b, ns);
                    value(c, ns);
                }
                _ => {}
            }
        }
        fn constraint(c: &mut Constraint, ns: &str) {
            match c {
                Constraint::Equal(a, b) => {
                    value(a, ns);
                    value(b, ns);
                }
                Constraint::AssertTrue(a) | Constraint::RangeU32(a) => value(a, ns),
                Constraint::Conditional(a, c) => {
                    value(a, ns);
                    constraint(c, ns);
                }
                Constraint::DigestEqual(a, b) => {
                    for x in a.iter_mut().chain(b) {
                        value(x, ns);
                    }
                }
            }
        }
        if other.constraints.is_empty() {
            other
                .unsupported
                .push(format!("{namespace}: no obligations"));
        }
        for c in &mut other.constraints {
            constraint(c, namespace);
        }
        for v in other.pub_inputs.iter_mut().chain(&mut other.divine_inputs) {
            v.name = format!("{namespace}_{}", v.name);
        }
        for v in &mut other.pub_outputs {
            value(v, namespace);
        }
        self.variables.extend(
            other
                .variables
                .into_iter()
                .map(|(k, v)| (format!("{namespace}_{k}"), v)),
        );
        self.constraints.extend(other.constraints);
        self.unsupported.extend(other.unsupported);
        self.num_variables += other.num_variables;
        self.pub_inputs.extend(other.pub_inputs);
        self.pub_outputs.extend(other.pub_outputs);
        self.divine_inputs.extend(other.divine_inputs);
    }
}

pub(super) fn coverage(func: &FnDef, file: &File, zero_is_true: bool) -> Result<(), String> {
    super::scalar_call::validate_helpers(func, file, zero_is_true)
}
