//! Reusable lexical calls; builtin expressions preserve once-only evaluation.
use super::*;

impl Compiler<'_> {
    pub(super) fn call(&mut self, source: &str, args: &[Spanned<Expr>]) -> LowerResult {
        let symbol = self.owner.function_symbol(source);
        if let Some(function) = self.plan.functions.get(&symbol) {
            if args.len() != function.definition.params.len() {
                return Err("native call arity mismatch".into());
            }
            let count = function.count;
            let values = args
                .iter()
                .map(|a| self.expr(&a.node))
                .collect::<Result<Vec<_>, _>>()?;
            let frame = layout::tree(values, count, true)?;
            return self
                .plan
                .invoke(&Code::Function(symbol), layout::subject(nox_axis(2), frame));
        }
        let name = self
            .owner
            .fns
            .get(&symbol)
            .and_then(|f| f.intrinsic.as_ref())
            .map(|i| ast::intrinsic_name(&i.node))
            .unwrap_or(source)
            .to_string();
        if matches!(name.as_str(), "divine" | "std.io.divine" | "os.state.read") {
            return Err("raw ART1 profile forbids host services".into());
        }
        if matches!(name.as_str(), "hash" | "std.crypto.hash") {
            if args.is_empty() {
                return Err("hash takes at least 1 argument".into());
            }
            return Ok(nox_hash(cons_list(
                args.iter()
                    .map(|a| self.expr(&a.node))
                    .collect::<Result<Vec<_>, _>>()?,
            )));
        }
        let arity = match name.as_str() {
            "nox_noun_pair" | "nox_noun_eq" | "assert_eq" | "assert_digest" | "sub"
            | "field_add" | "field_mul" => 2,
            "nox_noun_atom" | "nox_noun_head" | "nox_noun_tail" | "nox_noun_identity"
            | "nox_noun_as_field" | "assert" | "invert" | "std.field.inverse" | "inv" | "neg"
            | "as_field" | "as_u32" => 1,
            _ => return Err(format!("native builtin {name} has no implemented lowering")),
        };
        if args.len() != arity {
            return Err(format!("{name} takes {arity} arguments"));
        }
        let a = self.expr(&args[0].node)?;
        if arity == 2 {
            let b = self.expr(&args[1].node)?;
            return Ok(match name.as_str() {
                "nox_noun_pair" => nox_cons(a, b),
                "nox_noun_eq" => nox_eq(a, b),
                "assert_eq" | "assert_digest" => nox_branch(nox_eq(a, b), nox_unit(), nox_crash()),
                "sub" => nox_sub(a, b),
                "field_add" => nox_add(a, b),
                "field_mul" => nox_mul(a, b),
                _ => return Err("invalid native binary intrinsic".into()),
            });
        }
        Ok(match name.as_str() {
            "nox_noun_atom" | "as_field" => a,
            "nox_noun_head" => seq(a, nox_axis(2)),
            "nox_noun_tail" => seq(a, nox_axis(3)),
            "nox_noun_identity" => seq(a, nox_axis(0)),
            "nox_noun_as_field" => nox_add(a, nox_unit()),
            "assert" => nox_branch(a, nox_unit(), nox_crash()),
            "invert" | "std.field.inverse" | "inv" => nox_inv(a),
            "neg" => nox_sub(nox_unit(), a),
            "as_u32" => seq(
                a,
                nox_branch(
                    nox_lt(nox_axis(1), nox_quote(Noun::atom(1 << 32))),
                    nox_axis(1),
                    nox_crash(),
                ),
            ),
            _ => return Err("invalid native unary intrinsic".into()),
        })
    }
}
