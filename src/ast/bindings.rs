//! Final callable declarations shared by exports, specialization and lowering.
use super::{File, FnDef, Item, StructDef};
use std::collections::{BTreeMap, BTreeSet};

impl File {
    /// Select each name's last active declaration, retaining the source order
    /// of surviving declarations. This does not remove earlier bodies from the
    /// source file or exempt them from the checker's validation.
    pub(crate) fn final_functions(&self, flags: &BTreeSet<String>) -> Vec<&FnDef> {
        let mut functions = BTreeMap::new();
        for (index, item) in self.items.iter().enumerate() {
            if let Item::Fn(function) = &item.node {
                if function
                    .cfg
                    .as_ref()
                    .is_none_or(|cfg| flags.contains(&cfg.node))
                {
                    functions.insert(&function.name.node, (index, function));
                }
            }
        }
        let mut functions: Vec<_> = functions.into_values().collect();
        functions.sort_unstable_by_key(|(index, _)| *index);
        functions
            .into_iter()
            .map(|(_, function)| function)
            .collect()
    }
}

impl File {
    pub(crate) fn final_structs(&self, flags: &BTreeSet<String>) -> Vec<&StructDef> {
        let mut definitions = BTreeMap::new();
        for (index, item) in self.items.iter().enumerate() {
            if let Item::Struct(s) = &item.node {
                if item.node.cfg_active(flags) {
                    definitions.insert(&s.name.node, (index, s));
                }
            }
        }
        let mut definitions: Vec<_> = definitions.into_values().collect();
        definitions.sort_unstable_by_key(|(index, _)| *index);
        definitions.into_iter().map(|(_, s)| s).collect()
    }
}

impl Item {
    pub(crate) fn cfg_active(&self, flags: &BTreeSet<String>) -> bool {
        let cfg = match self {
            Item::Fn(f) => &f.cfg,
            Item::Struct(s) => &s.cfg,
            Item::Const(c) => &c.cfg,
            Item::Event(e) => &e.cfg,
        };
        cfg.as_ref().is_none_or(|c| flags.contains(&c.node))
    }
}
