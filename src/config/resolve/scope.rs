//! Explicit use bindings, shared by checking, lowering and editor navigation.
use super::{canonical_module_name, Diagnostic};
use crate::ast::File;
use crate::span::Span;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub(crate) struct Import {
    pub requested: String,
    pub owner: String,
    pub index: usize,
    pub span: Span,
}

impl Import {
    /// A use exposes the requested spelling, canonical spelling and basename.
    pub fn names(&self, member: &str) -> Vec<String> {
        let short = self.owner.rsplit('.').next().unwrap_or(&self.owner);
        [self.requested.as_str(), self.owner.as_str(), short]
            .into_iter()
            .map(|prefix| format!("{prefix}.{member}"))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Scope {
    pub imports: Vec<Import>,
}

/// Owners must be unique and dependencies supplied before their importers.
/// Repeated uses remain repeated: their order determines each symbol binding.
pub(crate) fn scopes(files: &[&File]) -> Result<Vec<Scope>, Vec<Diagnostic>> {
    let mut owners = BTreeMap::new();
    for (index, file) in files.iter().enumerate() {
        if owners.insert(file.name.node.as_str(), index).is_some() {
            return Err(vec![Diagnostic::error(
                format!("duplicate module '{}'", file.name.node),
                file.name.span,
            )]);
        }
    }
    files
        .iter()
        .enumerate()
        .map(|(caller, file)| {
            let mut scope = Scope::default();
            for usage in &file.uses {
                let requested = usage.node.as_dotted();
                let owner = canonical_module_name(&requested);
                let index = owners.get(owner.as_str()).copied().ok_or_else(|| {
                    vec![Diagnostic::error(
                        format!("module '{requested}' was not provided"),
                        usage.span,
                    )]
                })?;
                if files[index].kind != crate::ast::FileKind::Module {
                    return Err(vec![Diagnostic::error(
                        format!("import '{}' requires a module", requested),
                        usage.span,
                    )]);
                }
                if index >= caller {
                    return Err(vec![Diagnostic::error(
                        format!("dependency '{requested}' must precede '{}'", file.name.node),
                        usage.span,
                    )]);
                }
                scope.imports.push(Import {
                    requested,
                    owner,
                    index,
                    span: usage.span,
                });
            }
            Ok(scope)
        })
        .collect()
}

impl Scope {
    pub fn function_aliases(
        &self,
        files: &[&File],
        flags: &BTreeSet<String>,
    ) -> BTreeMap<String, String> {
        let mut aliases = BTreeMap::new();
        for import in &self.imports {
            for f in files[import.index]
                .final_functions(flags)
                .into_iter()
                .filter(|f| f.is_pub)
            {
                for name in import.names(&f.name.node) {
                    aliases.insert(name, format!("{}.{}", import.owner, f.name.node));
                }
            }
        }
        aliases
    }

    pub fn type_aliases(
        &self,
        file: &File,
        files: &[&File],
        flags: &BTreeSet<String>,
    ) -> BTreeMap<String, String> {
        let mut aliases = BTreeMap::new();
        for import in &self.imports {
            for s in files[import.index]
                .final_structs(flags)
                .into_iter()
                .filter(|s| s.is_pub)
            {
                for name in import.names(&s.name.node) {
                    aliases.insert(name, format!("{}.{}", import.owner, s.name.node));
                }
            }
        }
        for s in file.final_structs(flags) {
            aliases.insert(
                s.name.node.clone(),
                format!("{}.{}", file.name.node, s.name.node),
            );
        }
        aliases
    }
}
