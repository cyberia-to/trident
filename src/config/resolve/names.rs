//! Canonical owners shared by discovery and direct import scopes.
pub(crate) fn canonical_module_name(name: &str) -> String {
    if let Some(owner) = super::legacy_stdlib_fallback(name) {
        return owner.into();
    }
    if ["std.", "vm.", "os."]
        .iter()
        .any(|prefix| name.starts_with(prefix))
    {
        return name.into();
    }
    if let Some(rest) = name.strip_prefix("ext.") {
        if rest.contains('.') {
            return format!("os.{rest}");
        }
    }
    if let Some((os, module)) = name.split_once(".ext.") {
        return format!("os.{os}.{module}");
    }
    name.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_owners_are_fixed_points_of_legacy_resolution() {
        for (source, owner) in [
            ("std.convert", "vm.core.convert"),
            ("ext.neptune.ext.kernel", "os.neptune.ext.kernel"),
            ("neptune.ext.kernel", "os.neptune.kernel"),
            ("a.ext.b.ext.c", "os.a.b.ext.c"),
            ("std.library.ext.helper", "std.library.ext.helper"),
            ("vm.ext.helper", "vm.ext.helper"),
        ] {
            assert_eq!(canonical_module_name(source), owner);
            assert_eq!(canonical_module_name(&canonical_module_name(source)), owner);
        }
    }
}
