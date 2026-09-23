//! Native tree signatures and fixed-layout boundary checks.
use super::{FnSig, TypeChecker};
use crate::ast::{self, surface::SurfaceVisitor, Declaration, File, Type};
use crate::span::Span;
use crate::types::Ty;

impl TypeChecker {
    pub(super) fn check_scalar_index(
        &mut self,
        expr: &crate::ast::Expr,
        span: Span,
        context: &str,
    ) {
        let ty = self.check_expr(expr, span);
        if !matches!(ty, Ty::Field | Ty::U32) {
            self.error(
                format!("{context} requires Field or U32, got {}", ty.display()),
                span,
            );
        }
    }

    pub(super) fn register_noun_builtins(&mut self) {
        if self.target_config.name != "nox" {
            return;
        }
        use Ty::{Bool, Digest, Field, Noun};
        for (name, params, return_ty) in [
            ("atom", vec![Field], Noun),
            ("pair", vec![Noun, Noun], Noun),
            ("head", vec![Noun], Noun),
            ("tail", vec![Noun], Noun),
            ("as_field", vec![Noun], Field),
            ("eq", vec![Noun, Noun], Bool),
            ("identity", vec![Noun], Digest(4)),
        ] {
            self.functions.insert(
                format!("nox_noun_{name}"),
                FnSig {
                    params: params
                        .into_iter()
                        .enumerate()
                        .map(|(i, ty)| (format!("arg{i}"), ty))
                        .collect(),
                    return_ty,
                },
            );
        }
    }

    pub(super) fn check_noun_boundaries(&mut self, file: &File) {
        for declaration in &file.declarations {
            let types: Vec<_> = match declaration {
                Declaration::PubInput(t) | Declaration::PubOutput(t) | Declaration::SecInput(t) => {
                    vec![t]
                }
                Declaration::SecRam(slots) => slots.iter().map(|(_, t)| t).collect(),
            };
            for ty in types {
                if self.resolve_type(&ty.node).width().is_none() {
                    self.error(
                        "Noun has no flat I/O or RAM declaration layout".into(),
                        ty.span,
                    );
                }
            }
        }
        if self.target_config.name != "nox" {
            // Generic bodies may never be instantiated, but the native-only
            // primitive must still be rejected in active foreign source.
            struct NativeType(bool);
            impl SurfaceVisitor for NativeType {
                fn ty(&mut self, ty: &Type) {
                    self.0 |= ty.contains_noun();
                }
                fn call(&mut self, _: &str) {}
            }
            let mut visitor = NativeType(false);
            for item in &file.items {
                if self.is_item_cfg_active(&item.node) {
                    ast::surface::item(&item.node, &mut visitor);
                }
            }
            if visitor.0 {
                self.error("Noun requires the native nox target".into(), Span::dummy());
            }
        }
    }
}
