//! Final callable declarations shared by exports, specialization and lowering.
use super::{File, FnDef, Item};
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
