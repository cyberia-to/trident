// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use super::*;

impl SymExecutor {
    pub(crate) fn eval_expr(&mut self, expr: &Expr) -> SymValue {
        match expr {
            Expr::Literal(Literal::Integer(n)) => SymValue::Const(*n % GOLDILOCKS_P),
            Expr::Literal(Literal::Bool(b)) => SymValue::Const(if *b {
                self.truth_word()
            } else {
                1 - self.truth_word()
            }),
            Expr::Var(name) => {
                self.env.get(name).cloned().unwrap_or_else(|| {
                    // Unknown variable — treat as fresh symbolic
                    let var = self.fresh_var(name);
                    SymValue::Var(var)
                })
            }
            Expr::BinOp { op, lhs, rhs } => {
                let l = self.eval_expr(&lhs.node);
                let r = self.eval_expr(&rhs.node);
                match op {
                    BinOp::Add => SymValue::Add(Box::new(l), Box::new(r)).simplify(),
                    BinOp::Mul => SymValue::Mul(Box::new(l), Box::new(r)).simplify(),
                    BinOp::Eq => {
                        self.source_bool(SymValue::Eq(Box::new(l), Box::new(r)).simplify())
                    }
                    BinOp::Lt => {
                        self.source_bool(SymValue::Lt(Box::new(l), Box::new(r)).simplify())
                    }
                    _ => {
                        // BitAnd, BitXor, DivMod, XFieldMul — leave as opaque
                        SymValue::Var(self.fresh_var("__binop"))
                    }
                }
            }
            Expr::Call { path, args, .. } => self.eval_call(&path.node, args),
            Expr::Tuple(elems) => {
                // Tuples are represented as the first element for simplicity.
                // Full tuple tracking would require a SymValue::Tuple variant.
                if elems.len() == 1 {
                    self.eval_expr(&elems[0].node)
                } else {
                    // Create fresh variables for each tuple element
                    let var = self.fresh_var("__tuple");
                    SymValue::Var(var)
                }
            }
            Expr::FieldAccess { expr, .. } => {
                let _ = self.eval_expr(&expr.node);
                let var = self.fresh_var("__field");
                SymValue::Var(var)
            }
            Expr::Index { expr, .. } => {
                let _ = self.eval_expr(&expr.node);
                let var = self.fresh_var("__index");
                SymValue::Var(var)
            }
            Expr::StructInit { fields, .. } => {
                for (_, val) in fields {
                    let _ = self.eval_expr(&val.node);
                }
                let var = self.fresh_var("__struct");
                SymValue::Var(var)
            }
            Expr::ArrayInit(elems) => {
                for e in elems {
                    let _ = self.eval_expr(&e.node);
                }
                let var = self.fresh_var("__array");
                SymValue::Var(var)
            }
        }
    }

    /// Evaluate a function call (builtin or user-defined).
    pub(crate) fn eval_call(&mut self, path: &ModulePath, args: &[Spanned<Expr>]) -> SymValue {
        let name = path.as_dotted();
        let func_name = path.0.last().map(|s| s.as_str()).unwrap_or("");
        if let Some(function) = self.functions.get(&name).cloned() {
            return self.eval_scalar_call(&function, args);
        }

        // Native digest aliases follow the ABI; explicit numeric suffixes count
        // fixed field elements and must not be silently treated as scalar calls.
        let pub_width = if func_name == "read_digest" {
            Some(self.digest_width)
        } else {
            tuple_width(func_name, "pub_read").or_else(|| tuple_width(func_name, "read"))
        };
        if let Some(width) = pub_width {
            for _ in 0..width {
                self.fresh_pub_input();
            }
            let var = self.fresh_var("__read_tuple");
            return SymValue::Var(var);
        }
        let divine_width = if func_name == "divine_digest" {
            Some(self.digest_width)
        } else {
            tuple_width(func_name, "divine")
        };
        if let Some(width) = divine_width {
            for _ in 0..width {
                self.fresh_divine();
            }
            let var = self.fresh_var("__divine_tuple");
            return SymValue::Var(var);
        }

        // Handle builtins
        match func_name {
            "pub_read" | "read" => return self.fresh_pub_input(),
            "pub_write" | "write" => {
                if let Some(arg) = args.first() {
                    let val = self.eval_expr(&arg.node);
                    self.system.pub_outputs.push(val);
                }
                return SymValue::Const(0);
            }
            "divine" => return self.fresh_divine(),
            "hash" | "tip5" => {
                let inputs: Vec<SymValue> = args.iter().map(|a| self.eval_expr(&a.node)).collect();
                return SymValue::Hash(inputs, 0);
            }
            "assert" => {
                if let Some(arg) = args.first() {
                    let val = self.eval_expr(&arg.node);
                    self.add_constraint(Constraint::AssertTrue(self.assertion_truth(val)));
                }
                return SymValue::Const(0);
            }
            "assert_eq" | "eq" => {
                if args.len() >= 2 {
                    let a = self.eval_expr(&args[0].node);
                    let b = self.eval_expr(&args[1].node);
                    self.add_constraint(Constraint::Equal(a, b));
                }
                return SymValue::Const(0);
            }
            "assert_digest" | "digest" => {
                // Digest equality uses the selected target representation.
                if args.len() >= 2 {
                    let a = self.eval_expr(&args[0].node);
                    let b = self.eval_expr(&args[1].node);
                    self.add_constraint(Constraint::Equal(a, b));
                }
                return SymValue::Const(0);
            }
            "as_u32" => {
                if let Some(arg) = args.first() {
                    let val = self.eval_expr(&arg.node);
                    self.add_constraint(Constraint::RangeU32(val.clone()));
                    return val;
                }
                return SymValue::Const(0);
            }
            "as_field" => {
                // Type conversion: U32 → Field (identity in the field)
                if let Some(arg) = args.first() {
                    return self.eval_expr(&arg.node);
                }
                return SymValue::Const(0);
            }
            "sub" => {
                if args.len() >= 2 {
                    let a = self.eval_expr(&args[0].node);
                    let b = self.eval_expr(&args[1].node);
                    return SymValue::Sub(Box::new(a), Box::new(b)).simplify();
                }
                return SymValue::Const(0);
            }
            "neg" => {
                if let Some(arg) = args.first() {
                    let val = self.eval_expr(&arg.node);
                    return SymValue::Neg(Box::new(val)).simplify();
                }
                return SymValue::Const(0);
            }
            "inv" => {
                if let Some(arg) = args.first() {
                    let val = self.eval_expr(&arg.node);
                    return SymValue::Inv(Box::new(val));
                }
                return SymValue::Const(0);
            }
            _ => {}
        }

        self.system
            .unsupported
            .push(format!("unmodeled call {name}"));

        // Default: return a fresh symbolic variable
        let var = self.fresh_var(&format!("__call_{}", func_name));
        SymValue::Var(var)
    }

    /// Project element `i` from a tuple-like symbolic value.
    #[cfg(test)]
    pub(crate) fn project_tuple(&mut self, val: &SymValue, i: usize) -> SymValue {
        // If projecting from a hash, preserve the Hash origin with the index
        if let SymValue::Hash(inputs, _) = val {
            return SymValue::Hash(inputs.clone(), i);
        }
        let var = self.fresh_var(&format!("__proj_{}", i));
        SymValue::Var(var)
    }
}

fn tuple_width(name: &str, prefix: &str) -> Option<u32> {
    let suffix = name.strip_prefix(prefix)?;
    let width: u32 = suffix.parse().ok()?;
    (width > 1 && width <= 64 && suffix == width.to_string()).then_some(width)
}
