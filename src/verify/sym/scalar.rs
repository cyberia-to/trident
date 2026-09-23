//! Bounded path enumeration for the formally supported scalar subset.
use super::*;

const MAX_PATHS: usize = 256;
const MAX_STEPS: usize = 4096;

#[derive(Clone)]
pub(super) struct PathState {
    pub(super) env: BTreeMap<String, SymValue>,
    pub(super) conditions: Vec<SymValue>,
    pub(super) returned: Option<SymValue>,
    // Saved value before a block's first declaration of each local name.
    // Saving at declaration preserves earlier assignments to outer bindings.
    pub(super) shadows: BTreeMap<String, Option<SymValue>>,
}

impl SymExecutor {
    pub(crate) fn truth_word(&self) -> u64 {
        if self.zero_is_true { 0 } else { 1 }
    }

    pub(crate) fn source_bool(&self, logical: SymValue) -> SymValue {
        if self.zero_is_true {
            SymValue::Sub(Box::new(SymValue::Const(1)), Box::new(logical)).simplify()
        } else {
            logical
        }
    }

    pub(crate) fn assertion_truth(&self, value: SymValue) -> SymValue {
        SymValue::Eq(
            Box::new(value),
            Box::new(SymValue::Const(self.truth_word())),
        )
        .simplify()
    }

    fn branch_truth(&self, value: SymValue) -> SymValue {
        let zero = SymValue::Eq(Box::new(value), Box::new(SymValue::Const(0))).simplify();
        if self.zero_is_true {
            zero
        } else {
            SymValue::Sub(Box::new(SymValue::Const(1)), Box::new(zero)).simplify()
        }
    }

    pub(super) fn eval_on_path(&mut self, path: &PathState, expression: &Expr) -> SymValue {
        self.env = path.env.clone();
        self.path_condition = path.conditions.clone();
        let value = self.eval_expr(expression);
        if self.charge_scalar_nodes(super::scalar_call::value_nodes(&value)) {
            value
        } else {
            SymValue::Const(0)
        }
    }

    pub(crate) fn execute_scalar_function(&mut self, function: &FnDef) {
        let initial = PathState {
            env: self.env.clone(),
            conditions: self.path_condition.clone(),
            returned: None,
            shadows: BTreeMap::new(),
        };
        let paths =
            match self.scalar_block(&function.body.as_ref().unwrap().node, vec![initial], true) {
                Ok(paths) => paths,
                Err(reason) => {
                    self.system.unsupported.push(reason);
                    return;
                }
            };
        for mut path in paths {
            if function.return_ty.is_some() {
                path.env.insert(
                    "result".into(),
                    path.returned.clone().unwrap_or(SymValue::Const(0)),
                );
            }
            for predicate in &function.ensures {
                let expression = super::coverage::contract_expr(&predicate.node).unwrap();
                let value = self.eval_on_path(&path, &expression);
                self.add_constraint(Constraint::AssertTrue(self.assertion_truth(value)));
            }
        }
    }

    pub(super) fn scalar_block(
        &mut self,
        block: &Block,
        mut paths: Vec<PathState>,
        function_body: bool,
    ) -> Result<Vec<PathState>, String> {
        for statement in &block.stmts {
            let mut next = Vec::new();
            for mut path in paths {
                if path.returned.is_some() {
                    next.push(path);
                    continue;
                }
                self.scalar_steps += 1;
                if self.scalar_steps > MAX_STEPS {
                    return Err("scalar audit path/statement budget exceeded".into());
                }
                match &statement.node {
                    Stmt::Let {
                        pattern: Pattern::Name(name),
                        init,
                        ..
                    } => {
                        let value = self.eval_on_path(&path, &init.node);
                        path.shadows
                            .entry(name.node.clone())
                            .or_insert_with(|| path.env.get(&name.node).cloned());
                        self.fresh_var(&name.node);
                        path.env.insert(name.node.clone(), value);
                        next.push(path);
                    }
                    Stmt::Assign { place, value } => {
                        let value = self.eval_on_path(&path, &value.node);
                        let Place::Var(name) = &place.node else {
                            return Err("unsupported scalar assignment".into());
                        };
                        self.fresh_var(name);
                        path.env.insert(name.clone(), value);
                        next.push(path);
                    }
                    Stmt::Expr(expression) => {
                        self.eval_on_path(&path, &expression.node);
                        next.push(path);
                    }
                    Stmt::Return(value) => {
                        path.returned = Some(match value {
                            Some(value) => self.eval_on_path(&path, &value.node),
                            None => SymValue::Const(0),
                        });
                        next.push(path);
                    }
                    Stmt::If {
                        cond,
                        then_block,
                        else_block,
                    } => {
                        let raw = self.eval_on_path(&path, &cond.node);
                        let condition = self.branch_truth(raw);
                        for (take_then, body) in [
                            (true, Some(&then_block.node)),
                            (false, else_block.as_ref().map(|b| &b.node)),
                        ] {
                            let predicate = if take_then {
                                condition.clone()
                            } else {
                                SymValue::Sub(
                                    Box::new(SymValue::Const(1)),
                                    Box::new(condition.clone()),
                                )
                                .simplify()
                            };
                            if predicate.as_const() == Some(0) {
                                continue;
                            }
                            let mut branch = path.clone();
                            branch.conditions.push(predicate);
                            branch.shadows.clear();
                            let exits = match body {
                                Some(body) => self.scalar_block(body, vec![branch], false)?,
                                None => vec![branch],
                            };
                            for mut exit in exits {
                                for (name, previous) in &exit.shadows {
                                    match previous {
                                        Some(value) => {
                                            exit.env.insert(name.clone(), value.clone());
                                        }
                                        None => {
                                            exit.env.remove(name);
                                        }
                                    }
                                }
                                exit.shadows = path.shadows.clone();
                                next.push(exit);
                            }
                        }
                    }
                    _ => return Err("unsupported scalar statement".into()),
                }
                if next.len() > MAX_PATHS {
                    return Err("scalar audit path count exceeded".into());
                }
            }
            paths = next;
        }
        for path in &mut paths {
            if path.returned.is_none() {
                if let Some(tail) = &block.tail_expr {
                    let value = self.eval_on_path(path, &tail.node);
                    if function_body {
                        path.returned = Some(value);
                    }
                }
            }
        }
        Ok(paths)
    }
}
