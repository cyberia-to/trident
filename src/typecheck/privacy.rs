//! Field layout stays complete across imports; access is checked at each use.
use super::TypeChecker;
use crate::span::Span;
use crate::types::StructTy;

impl TypeChecker {
    pub(super) fn check_field_visibility(
        &mut self,
        structure: &StructTy,
        field: &str,
        public: bool,
        span: Span,
    ) {
        if !public && structure.module != self.current_module {
            self.error(
                format!(
                    "field '{}.{}.{}' is private to module '{}'",
                    structure.module, structure.name, field, structure.module
                ),
                span,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Item, MatchPattern, Stmt};

    #[test]
    fn struct_patterns_check_visibility_even_when_the_field_is_ignored() {
        let owner = crate::parse_source_silent(
            "module vault\npub struct Seal { value: Field }",
            "vault.tri",
        )
        .unwrap();
        let exports = TypeChecker::new().check_file(&owner).unwrap();
        for pattern in ["value", "value: 0", "value: _"] {
            let source = format!(
                "module caller\nfn inspect(s: vault.Seal) {{ match s {{ Seal {{ {pattern} }} => {{}} _ => {{}} }} }}"
            );
            let mut file = crate::parse_source_silent(&source, "caller.tri").unwrap();
            // Qualified pattern spelling is not yet in the parser's surface.
            // Exercise its semantic representation directly so access checks
            // remain correct when that syntax becomes available.
            let Item::Fn(function) = &mut file.items[0].node else {
                panic!()
            };
            let Stmt::Match { arms, .. } = &mut function.body.as_mut().unwrap().node.stmts[0].node
            else {
                panic!()
            };
            let MatchPattern::Struct { name, .. } = &mut arms[0].pattern.node else {
                panic!()
            };
            name.node = "vault.Seal".into();
            let mut checker = TypeChecker::new();
            checker.import_module(&exports);
            let errors = checker.check_file(&file).unwrap_err();
            assert!(errors
                .iter()
                .any(|e| e.message.contains("vault.Seal.value' is private")));
        }
        // The defining module may destructure its own private fields.
        let local = crate::parse_source_silent(
            "module vault\nstruct Seal { value: Field }\nfn read(s: Seal) -> Field { match s { Seal { value } => { return value } } }",
            "vault.tri",
        ).unwrap();
        TypeChecker::new().check_file(&local).unwrap();
    }
}
