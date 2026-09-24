//! Named constructor validation preserves one initializer per declared field.
use super::TypeChecker;
use crate::ast::{Expr, ModulePath};
use crate::span::{Span, Spanned};
use crate::types::Ty;

impl TypeChecker {
    pub(super) fn check_struct_init(
        &mut self,
        path: &Spanned<ModulePath>,
        init_fields: &[(Spanned<String>, Spanned<Expr>)],
        span: Span,
    ) -> Ty {
        let struct_name = path.node.as_dotted();
        if let Some(sty) = self.structs.get(&struct_name).cloned() {
            let mut seen = std::collections::BTreeSet::new();
            for (name, _) in init_fields {
                if !seen.insert(name.node.as_str()) {
                    self.error(
                        format!("duplicate field '{}' in struct init", name.node),
                        name.span,
                    );
                }
            }
            // Check all required fields are provided
            for (def_name, def_ty, public) in &sty.fields {
                self.check_field_visibility(&sty, def_name, *public, span);
                if let Some((_name, val)) = init_fields.iter().find(|(n, _)| n.node == *def_name) {
                    let val_ty = self.check_expr(&val.node, val.span);
                    if val_ty != *def_ty {
                        self.error(
                            format!(
                                "field '{}': expected {} but got {}",
                                def_name,
                                def_ty.display(),
                                val_ty.display()
                            ),
                            val.span,
                        );
                    }
                } else {
                    self.error(format!("missing field '{}' in struct init", def_name), span);
                }
            }
            // Check for extra fields
            for (name, _) in init_fields {
                if !sty.fields.iter().any(|(n, _, _)| *n == name.node) {
                    self.error(
                        format!("unknown field '{}' in struct '{}'", name.node, struct_name),
                        name.span,
                    );
                }
            }
            Ty::Struct(sty)
        } else {
            self.error_with_help(
                format!("undefined struct '{}'", struct_name),
                span,
                "check the struct name spelling, or import the module that defines it".to_string(),
            );
            Ty::Field
        }
    }
}
