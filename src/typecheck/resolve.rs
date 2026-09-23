// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Type resolution: constant detection, size inference, type unification, AST->Ty lowering.

use std::collections::BTreeMap;

use crate::ast::*;
use crate::span::Span;
use crate::types::Ty;

use super::{GenericFnDef, TypeChecker};

pub(crate) fn checked_size(size: &ArraySize, subs: &BTreeMap<String, u64>) -> Result<u64, String> {
    match size {
        ArraySize::Literal(n) => Ok(*n),
        ArraySize::Param(name) => subs
            .get(name)
            .copied()
            .ok_or_else(|| format!("unresolved array size '{name}'")),
        ArraySize::Add(a, b) => checked_size(a, subs)?
            .checked_add(checked_size(b, subs)?)
            .ok_or_else(|| "array size addition overflow".into()),
        ArraySize::Mul(a, b) => checked_size(a, subs)?
            .checked_mul(checked_size(b, subs)?)
            .ok_or_else(|| "array size multiplication overflow".into()),
    }
}

impl TypeChecker {
    pub(super) fn is_constant_expr(&self, expr: &Expr) -> bool {
        matches!(expr, Expr::Literal(Literal::Integer(_)))
            || matches!(expr, Expr::Var(name) if self.constants.contains_key(name))
    }

    /// Infer size arguments for a generic function from argument types.
    /// E.g. if param is `[Field; N]` and arg type is `[Field; 5]`, infer N=5.
    pub(super) fn infer_size_args(
        &mut self,
        gdef: &GenericFnDef,
        arg_tys: &[Ty],
        span: Span,
    ) -> Vec<u64> {
        let mut subs: BTreeMap<String, u64> = BTreeMap::new();

        for ((_, param_ty), arg_ty) in gdef.params.iter().zip(arg_tys.iter()) {
            Self::unify_sizes(param_ty, arg_ty, &mut subs);
        }

        let mut result = Vec::new();
        for param_name in &gdef.type_params {
            if let Some(&val) = subs.get(param_name) {
                result.push(val);
            } else {
                self.error(
                    format!(
                        "cannot infer size parameter '{}'; provide explicit size argument",
                        param_name
                    ),
                    span,
                );
                result.push(0);
            }
        }
        result
    }

    /// Recursively match an AST type pattern against a concrete Ty to extract
    /// size parameter bindings. E.g. `[Field; N]` vs `[Field; 5]` -> N=5.
    pub(super) fn unify_sizes(pattern: &Type, concrete: &Ty, subs: &mut BTreeMap<String, u64>) {
        match (pattern, concrete) {
            (Type::Array(inner_pat, ArraySize::Param(name)), Ty::Array(inner_ty, size)) => {
                subs.insert(name.clone(), *size);
                Self::unify_sizes(inner_pat, inner_ty, subs);
            }
            (Type::Array(inner_pat, _), Ty::Array(inner_ty, _)) => {
                Self::unify_sizes(inner_pat, inner_ty, subs);
            }
            (Type::Tuple(pats), Ty::Tuple(tys)) => {
                for (p, t) in pats.iter().zip(tys.iter()) {
                    Self::unify_sizes(p, t, subs);
                }
            }
            _ => {}
        }
    }

    pub(super) fn resolve_type(&mut self, ty: &Type) -> Ty {
        self.resolve_type_with_subs(ty, &self.constants.clone())
    }

    /// Resolve an AST type to a semantic type, substituting size parameters.
    pub(super) fn resolve_type_with_subs(&mut self, ty: &Type, subs: &BTreeMap<String, u64>) -> Ty {
        match ty {
            Type::Noun => {
                if self.target_config.name != "nox" {
                    self.error("Noun requires the native nox target".into(), Span::dummy());
                }
                Ty::Noun
            }
            Type::Field => Ty::Field,
            Type::XField => Ty::XField(self.target_config.xfield_width),
            Type::Bool => Ty::Bool,
            Type::U32 => Ty::U32,
            Type::Digest => Ty::Digest(self.target_config.digest_width),
            Type::Array(inner, n) => {
                let size = match checked_size(n, subs) {
                    Ok(n) => n,
                    Err(error) => {
                        self.error(error, Span::dummy());
                        0
                    }
                };
                Ty::Array(Box::new(self.resolve_type_with_subs(inner, subs)), size)
            }
            Type::Tuple(elems) => {
                let resolved: Vec<Ty> = elems
                    .iter()
                    .map(|t| self.resolve_type_with_subs(t, subs))
                    .collect();
                Ty::Tuple(resolved)
            }
            Type::Named(path) => {
                let name = path.as_dotted();
                if let Some(sty) = self.structs.get(&name) {
                    Ty::Struct(sty.clone())
                } else {
                    self.error(format!("unknown type '{}'", name), Span::dummy());
                    Ty::Field
                }
            }
        }
    }
}
