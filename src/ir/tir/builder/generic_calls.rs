//! Validate exact generic bindings before emitting any function body.
use super::TIRBuilder;
use crate::ast::{self, surface::SurfaceVisitor, Expr, File, Item, Type};
use crate::diagnostic::Diagnostic;
use std::collections::BTreeMap;

impl TIRBuilder {
    pub(super) fn check_generic_calls(&self, file: &File) -> Result<(), Vec<Diagnostic>> {
        let declarations: BTreeMap<_, _> = file
            .final_functions(&self.cfg_flags)
            .into_iter()
            .map(|f| (f.name.node.as_str(), f))
            .collect();
        let mut errors = Vec::new();
        for item in &file.items {
            let Item::Fn(function) = &item.node else {
                continue;
            };
            if !declarations
                .get(function.name.node.as_str())
                .is_some_and(|last| std::ptr::eq(*last, function))
            {
                continue;
            }
            let emitted = if function.type_params.is_empty() {
                !function.is_test
            } else {
                self.mono_instances
                    .iter()
                    .any(|i| i.name == function.name.node)
            };
            if emitted {
                ast::surface::item(
                    &item.node,
                    &mut Guard {
                        builder: self,
                        function: &function.name.node,
                        generic_body: !function.type_params.is_empty(),
                        errors: &mut errors,
                    },
                );
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

struct Guard<'a> {
    builder: &'a TIRBuilder,
    function: &'a str,
    generic_body: bool,
    errors: &'a mut Vec<Diagnostic>,
}
impl SurfaceVisitor for Guard<'_> {
    fn ty(&mut self, _: &Type) {}
    fn call(&mut self, _: &str) {}
    fn expression(&mut self, expr: &Expr) {
        let Expr::Call { path, .. } = expr else {
            return;
        };
        let name = path.node.as_dotted();
        let Some(definition) = self.builder.generic_fn_defs.get(&name) else {
            return;
        };
        let resolution = self.builder.call_resolutions.get(&(
            self.function.to_string(),
            path.span.start,
            path.span.end,
        ));
        let valid = !self.generic_body
            && resolution.is_some_and(|instance| {
                instance.name == name
                    && instance.size_args.len() == definition.type_params.len()
                    && self.builder.mono_instances.contains(instance)
            });
        if !valid {
            self.errors.push(Diagnostic::error(
                format!("generic call '{}' requires its checked source-site resolution and emitted instance", name),
                path.span,
            ).with_help("use build_tir or build_tir_modules to specialize nested generic bodies".into()));
        }
    }
}
