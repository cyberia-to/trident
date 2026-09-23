//! Native source values keep their entire subtree in a single subject slot.
use super::*;

pub(super) fn return_type(name: &str) -> Option<ast::Type> {
    Some(match name {
        "nox_noun_atom" | "nox_noun_pair" | "nox_noun_head" | "nox_noun_tail" => ast::Type::Noun,
        "nox_noun_as_field" => ast::Type::Field,
        "nox_noun_eq" => ast::Type::Bool,
        "nox_noun_identity" => ast::Type::Digest,
        _ => return None,
    })
}

impl NoxCompiler {
    pub(super) fn compile_noun_intrinsic(
        &mut self,
        name: &str,
        args: &[Spanned<Expr>],
    ) -> LowerResult {
        let arity = if matches!(name, "nox_noun_pair" | "nox_noun_eq") {
            2
        } else {
            1
        };
        if args.len() != arity {
            return Err(format!("{name} takes {arity} arguments"));
        }
        let left = self.compile_expr(&args[0].node)?;
        Ok(match name {
            "nox_noun_atom" => left,
            "nox_noun_head" => seq(left, nox_axis(2)),
            "nox_noun_tail" => seq(left, nox_axis(3)),
            "nox_noun_identity" => seq(left, nox_axis(0)),
            "nox_noun_as_field" => nox_add(left, nox_unit()),
            "nox_noun_pair" => nox_cons(left, self.compile_expr(&args[1].node)?),
            "nox_noun_eq" => nox_eq(left, self.compile_expr(&args[1].node)?),
            _ => return Err(format!("unknown native noun intrinsic {name}")),
        })
    }

    pub(super) fn contains_noun(&self, ty: &ast::Type) -> bool {
        fn visit(c: &NoxCompiler, ty: &ast::Type, seen: &mut BTreeSet<String>) -> bool {
            match ty {
                ast::Type::Noun => true,
                ast::Type::Array(t, _) => visit(c, t, seen),
                ast::Type::Tuple(ts) => ts.iter().any(|t| visit(c, t, seen)),
                ast::Type::Named(p) => {
                    let name = p.as_dotted();
                    seen.insert(name.clone())
                        && c.structs
                            .get(&name)
                            .is_some_and(|fields| fields.iter().any(|(_, t)| visit(c, t, seen)))
                }
                _ => false,
            }
        }
        visit(self, ty, &mut BTreeSet::new())
    }
}
