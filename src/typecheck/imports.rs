//! Checked exports enter only the importing file's explicit lexical scope.
use super::*;
use crate::resolve::scope::{Import, Scope};

impl TypeChecker {
    pub(crate) fn import_scope(
        &mut self,
        scope: &Scope,
        exports: &[ModuleExports],
    ) -> Result<(), Vec<Diagnostic>> {
        for import in &scope.imports {
            let exports = exports.get(import.index).ok_or_else(|| {
                vec![Diagnostic::error(
                    format!("dependency '{}' has no checked exports", import.requested),
                    import.span,
                )]
            })?;
            self.import_as(exports, import);
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn import_module(&mut self, exports: &ModuleExports) {
        self.import_as(
            exports,
            &Import {
                requested: exports.module_name.clone(),
                owner: exports.module_name.clone(),
                index: 0,
                span: Span::dummy(),
            },
        );
    }

    fn import_as(&mut self, exports: &ModuleExports, import: &Import) {
        for (name, params, return_ty) in &exports.functions {
            let sig = FnSig {
                intrinsic: exports.direct_intrinsics.get(name).cloned(),
                params: params.clone(),
                return_ty: return_ty.clone(),
            };
            for visible in import.names(name) {
                self.imported_requirements.insert(
                    visible.clone(),
                    exports
                        .function_requirements
                        .get(name)
                        .cloned()
                        .unwrap_or_default(),
                );
                self.generic_fns.remove(&visible);
                self.functions.insert(visible, sig.clone());
            }
        }
        for (name, definition) in &exports.generic_functions {
            let mut definition = definition.clone();
            definition.canonical_name = Some(format!("{}.{}", exports.module_name, name));
            for visible in import.names(name) {
                self.imported_requirements.insert(
                    visible.clone(),
                    exports
                        .function_requirements
                        .get(name)
                        .cloned()
                        .unwrap_or_default(),
                );
                self.functions.remove(&visible);
                self.generic_fns.insert(visible, definition.clone());
            }
        }
        exports
            .resolved_constants
            .import_as(&mut self.constant_bindings, import);
        for (name, ty, value) in &exports.constants {
            for visible in import.names(name) {
                self.constant_types.insert(visible.clone(), ty.clone());
                self.constants.insert(visible, *value);
            }
        }
        for sty in &exports.structs {
            for visible in import.names(&sty.name) {
                self.structs.insert(visible, sty.clone());
            }
        }
    }
}
