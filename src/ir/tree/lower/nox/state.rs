//! State effects follow the same lexical function/intrinsic resolution as calls.
use super::*;

impl NoxCompiler {
    pub(super) fn scan_state_functions(&mut self) {
        let saved = self.current_module.clone();
        loop {
            let mut additions = Vec::new();
            for function in self.fns.values() {
                self.current_module = function
                    .name
                    .node
                    .rsplit_once('.')
                    .map(|(module, _)| module.to_string())
                    .unwrap_or_default();
                if self.function_uses_state(function) {
                    additions.push(function.name.node.clone());
                }
            }
            let before = self.state_functions.len();
            self.state_functions.extend(additions);
            if before == self.state_functions.len() {
                break;
            }
        }
        self.current_module = saved;
    }

    pub(super) fn function_uses_state(&self, function: &FnDef) -> bool {
        function.body.as_ref().is_some_and(|body| {
            block_calls(&body.node, &|name| {
                if let Some(callee) = self.fns.get(&self.symbol(name)) {
                    if let Some(intrinsic) = &callee.intrinsic {
                        let intrinsic = intrinsic
                            .node
                            .strip_prefix("intrinsic(")
                            .and_then(|s| s.strip_suffix(')'))
                            .unwrap_or(&intrinsic.node);
                        return intrinsic == "os.state.read";
                    }
                    return self.state_functions.contains(&callee.name.node);
                }
                name == "os.state.read"
            })
        })
    }
}

fn block_calls(block: &Block, relevant: &impl Fn(&str) -> bool) -> bool {
    block
        .stmts
        .iter()
        .any(|stmt| stmt_calls(&stmt.node, relevant))
        || block
            .tail_expr
            .as_ref()
            .is_some_and(|expr| expr_calls(&expr.node, relevant))
}
fn stmt_calls(stmt: &Stmt, relevant: &impl Fn(&str) -> bool) -> bool {
    match stmt {
        Stmt::Let { init, .. } => expr_calls(&init.node, relevant),
        Stmt::Assign { value, .. } | Stmt::TupleAssign { value, .. } => {
            expr_calls(&value.node, relevant)
        }
        Stmt::If {
            cond,
            then_block,
            else_block,
        } => {
            expr_calls(&cond.node, relevant)
                || block_calls(&then_block.node, relevant)
                || else_block
                    .as_ref()
                    .is_some_and(|block| block_calls(&block.node, relevant))
        }
        Stmt::For {
            start, end, body, ..
        } => {
            expr_calls(&start.node, relevant)
                || expr_calls(&end.node, relevant)
                || block_calls(&body.node, relevant)
        }
        Stmt::Expr(expr) => expr_calls(&expr.node, relevant),
        Stmt::Return(expr) => expr
            .as_ref()
            .is_some_and(|expr| expr_calls(&expr.node, relevant)),
        Stmt::Match { expr, arms } => {
            expr_calls(&expr.node, relevant)
                || arms.iter().any(|arm| block_calls(&arm.body.node, relevant))
        }
        Stmt::Reveal { fields, .. } | Stmt::Seal { fields, .. } => fields
            .iter()
            .any(|(_, expr)| expr_calls(&expr.node, relevant)),
        Stmt::Asm { .. } => false,
    }
}
fn expr_calls(expr: &Expr, relevant: &impl Fn(&str) -> bool) -> bool {
    match expr {
        Expr::Call { path, args, .. } => {
            relevant(&path.node.as_dotted())
                || args.iter().any(|arg| expr_calls(&arg.node, relevant))
        }
        Expr::BinOp { lhs, rhs, .. } => {
            expr_calls(&lhs.node, relevant) || expr_calls(&rhs.node, relevant)
        }
        Expr::FieldAccess { expr, .. } => expr_calls(&expr.node, relevant),
        Expr::Index { expr, index } => {
            expr_calls(&expr.node, relevant) || expr_calls(&index.node, relevant)
        }
        Expr::StructInit { fields, .. } => fields
            .iter()
            .any(|(_, expr)| expr_calls(&expr.node, relevant)),
        Expr::ArrayInit(exprs) | Expr::Tuple(exprs) => {
            exprs.iter().any(|expr| expr_calls(&expr.node, relevant))
        }
        Expr::Literal(_) | Expr::Var(_) => false,
    }
}
