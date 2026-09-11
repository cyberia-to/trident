//! Version-matched language libraries and target declarations travel with the compiler.

include!(concat!(env!("OUT_DIR"), "/resources.rs"));

pub(crate) fn get(path: &str) -> Option<&'static str> {
    FILES
        .iter()
        .find(|(name, _)| *name == path)
        .map(|(_, source)| *source)
}

pub(crate) fn module(name: &str) -> Option<&'static str> {
    get(&format!("{}.tri", name.replace('.', "/")))
}
