//! Body-derived scalar calls. Contracts remain obligations, never summaries.
use super::*;
use std::collections::BTreeSet;

pub(super) fn validate_helpers(root: &FnDef, file: &File, zero: bool) -> Result<(), String> {
    fn calls_expr(expr: &Expr, calls: &mut Vec<String>) {
        match expr {
            Expr::Call { path, args, .. } => {
                calls.push(path.node.as_dotted());
                for arg in args {
                    calls_expr(&arg.node, calls);
                }
            }
            Expr::BinOp { lhs, rhs, .. } => {
                calls_expr(&lhs.node, calls);
                calls_expr(&rhs.node, calls);
            }
            _ => {}
        }
    }
    fn calls_block(block: &Block, calls: &mut Vec<String>) {
        for stmt in &block.stmts {
            match &stmt.node {
                Stmt::Let { init, .. } => calls_expr(&init.node, calls),
                Stmt::Assign { value, .. } | Stmt::Expr(value) => calls_expr(&value.node, calls),
                Stmt::Return(Some(value)) => calls_expr(&value.node, calls),
                Stmt::If {
                    cond,
                    then_block,
                    else_block,
                } => {
                    calls_expr(&cond.node, calls);
                    calls_block(&then_block.node, calls);
                    if let Some(block) = else_block {
                        calls_block(&block.node, calls);
                    }
                }
                _ => {}
            }
        }
        if let Some(tail) = &block.tail_expr {
            calls_expr(&tail.node, calls);
        }
    }
    fn visit(
        function: &FnDef,
        file: &File,
        zero: bool,
        helper: bool,
        stack: &mut BTreeSet<String>,
        done: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        if stack.contains(&function.name.node) {
            return Err("recursive scalar helper call".into());
        }
        if done.contains(&function.name.node) {
            return Ok(());
        }
        if stack.len() >= 64 || done.len() >= 256 {
            return Err("scalar helper graph budget exceeded".into());
        }
        if !function.type_params.is_empty()
            || function.intrinsic.is_some()
            || function.cfg.is_some()
            || function.is_test
        {
            return Err(
                "generic, intrinsic or conditional helper definition is not modeled".into(),
            );
        }
        super::coverage::coverage_one(function, file, zero)?;
        let mut calls = Vec::new();
        calls_block(&function.body.as_ref().unwrap().node, &mut calls);
        stack.insert(function.name.node.clone());
        for name in calls {
            if let Some(callee) = file.items.iter().find_map(|item| match &item.node {
                Item::Fn(f) if f.name.node == name => Some(f),
                _ => None,
            }) {
                visit(callee, file, zero, true, stack, done)?;
            } else if helper && matches!(name.as_str(), "pub_read" | "pub_write" | "divine") {
                return Err("effectful scalar helper is not modeled".into());
            }
        }
        stack.remove(&function.name.node);
        done.insert(function.name.node.clone());
        Ok(())
    }
    let mut definitions = BTreeSet::new();
    for item in &file.items {
        if let Item::Fn(f) = &item.node {
            if !definitions.insert(f.name.node.clone()) {
                return Err("ambiguous local function definition".into());
            }
        }
    }
    visit(
        root,
        file,
        zero,
        false,
        &mut BTreeSet::new(),
        &mut BTreeSet::new(),
    )
}

impl SymExecutor {
    pub(super) fn eval_scalar_call(
        &mut self,
        function: &FnDef,
        arguments: &[Spanned<Expr>],
    ) -> SymValue {
        self.scalar_calls += 1;
        if self.call_depth >= self.max_call_depth || self.scalar_calls > 256 {
            self.system
                .unsupported
                .push("scalar helper call budget exceeded".into());
            return SymValue::Const(0);
        }
        // All arguments are evaluated before any parameter can shadow the caller.
        let values: Vec<_> = arguments
            .iter()
            .map(|arg| self.eval_expr(&arg.node))
            .collect();
        let saved_env = self.env.clone();
        let saved_path = self.path_condition.clone();
        self.env = function
            .params
            .iter()
            .zip(values)
            .map(|(param, value)| (param.name.node.clone(), value))
            .collect();
        self.call_depth += 1;
        let result = self.scalar_call_body(function);
        self.call_depth -= 1;
        self.env = saved_env;
        self.path_condition = saved_path;
        match result {
            Ok(value) => value,
            Err(reason) => {
                self.system.unsupported.push(reason);
                SymValue::Const(0)
            }
        }
    }

    fn scalar_call_body(&mut self, function: &FnDef) -> Result<SymValue, String> {
        for param in &function.params {
            let value = self
                .env
                .get(&param.name.node)
                .ok_or("missing helper argument")?
                .clone();
            match param.ty.node {
                Type::U32 => self.add_constraint(Constraint::RangeU32(value)),
                Type::Bool => self.add_constraint(Constraint::AssertTrue(SymValue::Lt(
                    Box::new(value),
                    Box::new(SymValue::Const(2)),
                ))),
                _ => {}
            }
        }
        for predicate in &function.requires {
            let expr = super::coverage::contract_expr(&predicate.node)?;
            let value = self.eval_expr(&expr);
            self.add_constraint(Constraint::AssertTrue(self.assertion_truth(value)));
        }
        let initial = super::scalar::PathState {
            env: self.env.clone(),
            conditions: self.path_condition.clone(),
            returned: None,
            shadows: BTreeMap::new(),
        };
        let paths = self.scalar_block(
            &function.body.as_ref().ok_or("missing helper body")?.node,
            vec![initial],
            true,
        )?;
        let mut result = SymValue::Const(0);
        for mut path in paths.into_iter().rev() {
            let returned = match path.returned.clone() {
                Some(value) => value,
                None if function.return_ty.is_none() => SymValue::Const(0),
                None => return Err("scalar helper has a path without a return".into()),
            };
            path.env.insert("result".into(), returned.clone());
            for predicate in &function.ensures {
                let expr = super::coverage::contract_expr(&predicate.node)?;
                let value = self.eval_on_path(&path, &expr);
                self.add_constraint(Constraint::AssertTrue(self.assertion_truth(value)));
            }
            let guard = path
                .conditions
                .into_iter()
                .fold(SymValue::Const(1), |a, b| {
                    SymValue::Mul(Box::new(a), Box::new(b)).simplify()
                });
            if !self.charge_scalar_nodes(value_nodes(&guard) + value_nodes(&returned) + 1) {
                return Err("scalar helper result expansion budget exceeded".into());
            }
            result =
                SymValue::Ite(Box::new(guard), Box::new(returned), Box::new(result)).simplify();
        }
        Ok(result)
    }
}

const MAX_SCALAR_NODES: usize = 65_536;
pub(super) fn value_nodes(value: &SymValue) -> usize {
    let mut pending = vec![value];
    let mut count = 0;
    while let Some(value) = pending.pop() {
        count += 1;
        if count > MAX_SCALAR_NODES {
            return count;
        }
        match value {
            SymValue::Add(a, b)
            | SymValue::Mul(a, b)
            | SymValue::Sub(a, b)
            | SymValue::Eq(a, b)
            | SymValue::Lt(a, b) => pending.extend([a.as_ref(), b.as_ref()]),
            SymValue::Neg(a) | SymValue::Inv(a) | SymValue::FieldAccess(a, _) => pending.push(a),
            SymValue::Ite(a, b, c) => pending.extend([a.as_ref(), b.as_ref(), c.as_ref()]),
            SymValue::Hash(values, _) => pending.extend(values),
            _ => {}
        }
    }
    count
}
pub(super) fn constraint_nodes(c: &Constraint) -> usize {
    match c {
        Constraint::Equal(a, b) => value_nodes(a) + value_nodes(b),
        Constraint::AssertTrue(a) | Constraint::RangeU32(a) => value_nodes(a),
        Constraint::Conditional(a, c) => value_nodes(a) + constraint_nodes(c),
        Constraint::DigestEqual(a, b) => a.iter().chain(b).map(value_nodes).sum(),
    }
}
impl SymExecutor {
    pub(super) fn charge_scalar_nodes(&mut self, count: usize) -> bool {
        if self.scalar_nodes > MAX_SCALAR_NODES {
            return false;
        }
        self.scalar_nodes = self.scalar_nodes.saturating_add(count);
        if self.scalar_nodes > MAX_SCALAR_NODES {
            self.system
                .unsupported
                .push("scalar symbolic expansion budget exceeded".into());
            false
        } else {
            true
        }
    }
}
