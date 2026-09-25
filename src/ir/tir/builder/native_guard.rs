//! Reject variable native trees before any stack width is computed.
use super::TIRBuilder;
use crate::ast::{self, surface::SurfaceVisitor, File, Item, Type};
use crate::diagnostic::Diagnostic;
use std::collections::BTreeSet;

impl TIRBuilder {
    pub(super) fn check_fixed_layout(&self, file: &File) -> Result<(), Vec<Diagnostic>> {
        let locals: BTreeSet<_> = file
            .items
            .iter()
            .filter_map(|i| match &i.node {
                Item::Fn(f) if self.is_item_cfg_active(&i.node) => Some(f.name.node.as_str()),
                _ => None,
            })
            .collect();
        let mut guard = Guard {
            builder: self,
            locals,
            rejected: false,
        };
        ast::surface::declarations(file, &mut guard);
        for item in &file.items {
            if !self.is_item_cfg_active(&item.node) {
                continue;
            }
            ast::surface::item(&item.node, &mut guard);
            if let Item::Fn(f) = &item.node {
                if let Some(i) = &f.intrinsic {
                    guard.intrinsic(ast::intrinsic_name(&i.node));
                }
            }
            if guard.rejected {
                return Err(vec![Diagnostic::error(
                    "Noun requires native nox tree lowering; shared TIR has no Noun layout".into(),
                    item.span,
                )]);
            }
        }
        if guard.rejected {
            return Err(vec![Diagnostic::error(
                "Noun has no shared TIR layout".into(),
                file.name.span,
            )]);
        }
        Ok(())
    }

    fn contains_noun(&self, ty: &Type, seen: &mut BTreeSet<String>) -> bool {
        match ty {
            Type::Noun => true,
            Type::Array(t, _) => self.contains_noun(t, seen),
            Type::Tuple(ts) => ts.iter().any(|t| self.contains_noun(t, seen)),
            Type::Named(path) => {
                let name = path.as_dotted();
                if !seen.insert(name.clone()) {
                    return false;
                }
                self.struct_types.get(&name).is_some_and(|s| {
                    s.fields
                        .iter()
                        .any(|f| self.contains_noun(&f.ty.node, seen))
                })
            }
            _ => false,
        }
    }
}

struct Guard<'a> {
    builder: &'a TIRBuilder,
    locals: BTreeSet<&'a str>,
    rejected: bool,
}
impl Guard<'_> {
    fn intrinsic(&mut self, name: &str) {
        // An external owner's fixed-word ABI takes precedence over builtins.
        if !self.builder.target_intrinsics.contains_key(name) {
            self.rejected |= matches!(
                name,
                "nox_noun_atom"
                    | "nox_noun_pair"
                    | "nox_noun_head"
                    | "nox_noun_tail"
                    | "nox_noun_as_field"
                    | "nox_noun_eq"
                    | "nox_noun_identity"
            );
        }
    }
}
impl SurfaceVisitor for Guard<'_> {
    fn ty(&mut self, ty: &Type) {
        self.rejected |= self.builder.contains_noun(ty, &mut BTreeSet::new());
    }
    fn call(&mut self, name: &str) {
        if let Some(ty) = self
            .builder
            .fn_return_types
            .get(&self.builder.qualified_function(name))
        {
            self.ty(ty);
        }
        if let Some(intrinsic) = self.builder.intrinsic_map.get(name) {
            self.intrinsic(intrinsic);
        } else if !self.locals.contains(name)
            && !self.builder.fn_return_widths.contains_key(name)
            && !self.builder.generic_fn_defs.contains_key(name)
        {
            self.intrinsic(name);
        }
    }
}
