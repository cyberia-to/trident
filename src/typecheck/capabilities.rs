//! Infer target requirements through local and imported function calls.
use super::{ModuleExports, TypeChecker};
use crate::ast::{File, Item};
use crate::diagnostic::Diagnostic;
use std::collections::{BTreeMap, BTreeSet};

impl TypeChecker {
    pub(super) fn validate_intrinsic(
        &mut self,
        function: &crate::ast::FnDef,
        params: &[(String, crate::types::Ty)],
        return_ty: &crate::types::Ty,
    ) {
        let Some(intrinsic) = &function.intrinsic else {
            return;
        };
        let name = crate::ast::intrinsic_name(&intrinsic.node);
        match self.intrinsic_signatures.get(name) {
            None => self.error(
                format!("unknown intrinsic '{name}' for the selected target ABI"),
                intrinsic.span,
            ),
            Some(expected)
                if &expected.return_ty != return_ty
                    || expected
                        .params
                        .iter()
                        .map(|(_, ty)| ty)
                        .ne(params.iter().map(|(_, ty)| ty)) =>
            {
                self.error(
                    format!(
                        "intrinsic '{name}' declaration does not match the selected target ABI"
                    ),
                    intrinsic.span,
                )
            }
            _ => {}
        }
    }

    /// Editor tools expose the owner's callable builtin surface.
    pub(crate) fn available_functions(&self) -> impl Iterator<Item = (&String, &super::FnSig)> {
        self.functions
            .iter()
            .filter(|(name, _)| self.available_intrinsics.contains(*name))
    }

    pub(super) fn infer_requirements(&self, file: &File) -> BTreeMap<String, BTreeSet<String>> {
        let mut requirements = self.imported_requirements.clone();
        for name in self.intrinsic_signatures.keys() {
            requirements.insert(name.clone(), BTreeSet::from([name.clone()]));
        }
        let mut calls = BTreeMap::new();
        for item in &file.items {
            if !self.is_item_cfg_active(&item.node) {
                continue;
            }
            if let Item::Fn(function) = &item.node {
                let name = function.name.node.clone();
                // A local declaration shadows the identically named builtin.
                let direct = function
                    .intrinsic
                    .as_ref()
                    .map(|i| BTreeSet::from([crate::ast::intrinsic_name(&i.node).to_string()]))
                    .unwrap_or_default();
                requirements.insert(name.clone(), direct);
                let mut callees = Vec::new();
                if let Some(body) = &function.body {
                    Self::collect_calls_block(&body.node, &mut callees);
                }
                calls.insert(name, callees);
            }
        }
        // Monotone finite sets: local SCCs terminate without recursive Rust calls.
        loop {
            let mut changed = false;
            for (name, callees) in &calls {
                let mut extra = BTreeSet::new();
                for callee in callees {
                    if let Some(required) = requirements.get(callee) {
                        extra.extend(required.iter().cloned());
                    }
                    if self.target_config.architecture != crate::target::Arch::Stack {
                        if callee.starts_with("@operator:") {
                            extra.insert(callee.clone());
                        }
                        if let Some(tag) = callee.strip_prefix("@asm:") {
                            if tag.is_empty() || tag == self.target_config.name {
                                extra.insert(callee.clone());
                            }
                        }
                    }
                }
                let current = requirements
                    .get_mut(name)
                    .expect("local function initialized");
                let before = current.len();
                current.extend(extra);
                changed |= current.len() != before;
            }
            if !changed {
                break;
            }
        }
        calls
            .keys()
            .map(|name| (name.clone(), requirements[name].clone()))
            .collect()
    }
}

impl ModuleExports {
    /// Validate the actual entry selected by lowering; imported libraries retain
    /// requirements without claiming that every export is callable on this target.
    pub(crate) fn check_entry_requirements(
        &self,
        file: &File,
        options: &crate::CompileOptions,
    ) -> Result<(), Vec<Diagnostic>> {
        let available = options.checker().available_intrinsics;
        match entry_errors(
            file,
            &self.function_requirements,
            &available,
            &options.cfg_flags,
            &options.target_config.name,
        ) {
            Some(errors) => Err(errors),
            None => Ok(()),
        }
    }
}

pub(super) fn entry_errors(
    file: &File,
    requirements: &BTreeMap<String, BTreeSet<String>>,
    available: &BTreeSet<String>,
    flags: &BTreeSet<String>,
    target: &str,
) -> Option<Vec<Diagnostic>> {
    let functions = || {
        file.items.iter().filter_map(|item| match &item.node {
            Item::Fn(function)
                if function
                    .cfg
                    .as_ref()
                    .is_none_or(|cfg| flags.contains(&cfg.node)) =>
            {
                Some(function)
            }
            _ => None,
        })
    };
    let entry = functions()
        .find(|f| f.name.node == "main")
        .or_else(|| functions().find(|f| f.is_pub && f.body.is_some()))?;
    let errors: Vec<_> = requirements
        .get(&entry.name.node)?
        .difference(available)
        .map(|name| {
            Diagnostic::error(
                format!(
                    "entry '{}' requires intrinsic '{}' which target '{}' does not provide",
                    entry.name.node, name, target
                ),
                entry.name.span,
            )
        })
        .collect();
    (!errors.is_empty()).then_some(errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checked(source: &str) -> Result<ModuleExports, Vec<Diagnostic>> {
        let file = crate::parse_source_silent(source, "test.tri").unwrap();
        TypeChecker::new().check_file(&file)
    }

    #[test]
    fn unused_requirements_are_reported_and_reachable_calls_rejected() {
        let module = checked("module library\npub fn portable() -> Field { 7 }\nfn inner() -> Field { ram_read(0) }\npub fn memory() -> Field { inner() }\n").unwrap();
        assert!(module.function_requirements["portable"].is_empty());
        assert!(module.function_requirements["memory"].contains("ram_read"));
        for (called, accepted) in [("portable", true), ("memory", false)] {
            let file = crate::parse_source_silent(
                &format!("program test\nuse library\nfn main() -> Field {{ library.{called}() }}"),
                "test.tri",
            )
            .unwrap();
            let mut checker = TypeChecker::new();
            checker.import_module(&module);
            assert_eq!(checker.check_file(&file).is_ok(), accepted);
        }
    }

    #[test]
    fn requirements_follow_multiple_module_aliases() {
        let leaf =
            checked("module vm.leaf\n#[intrinsic(ram_read)]\npub fn read(addr: Field) -> Field\n")
                .unwrap();
        let wrapper = crate::parse_source_silent(
            "module std.wrapper\nuse vm.leaf\npub fn indirect() -> Field { leaf.read(0) }",
            "wrapper.tri",
        )
        .unwrap();
        let mut checker = TypeChecker::new();
        checker.import_module(&leaf);
        let wrapper = checker.check_file(&wrapper).unwrap();
        assert!(wrapper.function_requirements["indirect"].contains("ram_read"));
        for alias in ["wrapper", "std.wrapper"] {
            let file = crate::parse_source_silent(
                &format!("program entry\nfn main() -> Field {{ {alias}.indirect() }}"),
                "entry.tri",
            )
            .unwrap();
            let mut checker = TypeChecker::new();
            checker.import_module(&wrapper);
            assert!(checker
                .check_file(&file)
                .unwrap_err()
                .iter()
                .any(|e| e.message.contains("ram_read")));
        }
    }

    #[test]
    fn index_and_assignment_index_calls_cannot_hide_requirements() {
        for body in [
            "let a: [Field; 2] = [1, 2]\nlet x = a[as_u32(ram_read(0))]",
            "let mut a: [Field; 2] = [1, 2]\na[as_u32(ram_read(0))] = 7",
        ] {
            let errors = checked(&format!("program indexed\nfn main() {{ {body} }}")).unwrap_err();
            assert!(
                errors
                    .iter()
                    .any(|e| e.message.contains("requires intrinsic 'ram_read'")),
                "{errors:?}"
            );
        }
    }

    #[test]
    fn language_operators_and_assembly_require_actual_lowering() {
        for body in [
            "let a: U32 = 7\nlet b: U32 = 2\nlet pair = a /% b",
            "asm { push 0 }",
        ] {
            let errors =
                checked(&format!("program operation\nfn main() {{ {body} }}")).unwrap_err();
            assert!(
                errors
                    .iter()
                    .any(|e| e.message.contains("requires intrinsic")),
                "{errors:?}"
            );
        }
    }

    #[test]
    fn local_function_shadowing_a_builtin_does_not_inherit_its_requirement() {
        assert!(checked("program shadow\nfn ram_read(x: Field) -> Field { x + 1 }\nfn main() -> Field { ram_read(4) }").is_ok());
    }

    #[test]
    fn selected_entry_in_a_module_is_checked_at_compilation_boundary() {
        let file = crate::parse_source_silent(
            "module entry\npub fn first() -> Field { ram_read(0) }",
            "entry.tri",
        )
        .unwrap();
        let exports = TypeChecker::new().check_file(&file).unwrap();
        assert!(exports
            .check_entry_requirements(&file, &crate::CompileOptions::default())
            .is_err());
    }

    #[test]
    fn xfield_destructuring_uses_selected_width() {
        let file = crate::parse_source_silent("program extension\nfn main() -> Field { let value = xfield(1, 2, 3)\nlet (a, b, c) = value\na + b + c }", "extension.tri").unwrap();
        assert!(
            TypeChecker::with_target(crate::target::TerrainConfig::triton())
                .with_intrinsics(&crate::target::TerrainConfig::test_intrinsics())
                .check_file(&file)
                .is_ok()
        );
        let file = crate::parse_source_silent(
            "program extension\nfn main() { let value = xfield(1, 2, 3)\nlet (a, b) = value }",
            "extension.tri",
        )
        .unwrap();
        assert!(
            TypeChecker::with_target(crate::target::TerrainConfig::triton())
                .with_intrinsics(&crate::target::TerrainConfig::test_intrinsics())
                .check_file(&file)
                .unwrap_err()
                .iter()
                .any(|e| e.message.contains("exactly 3"))
        );
    }
}
