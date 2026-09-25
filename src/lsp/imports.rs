//! One exact spelling map for editor hover, completion and navigation.
use super::util::span_to_range;
use crate::ast::display::{format_ast_type, format_const_value, format_fn_signature};
use crate::ast::Item;
use crate::resolve::{self, scope};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use tower_lsp::lsp_types::*;

#[derive(Clone)]
pub(super) struct Symbol {
    pub location: Location,
    pub detail: String,
    pub kind: CompletionItemKind,
    pub parameters: Option<Vec<String>>,
}
#[derive(Default)]
pub(super) struct Symbols {
    pub functions: BTreeMap<String, Symbol>,
    pub types: BTreeMap<String, Symbol>,
    pub constants: BTreeMap<String, Symbol>,
}
impl Symbols {
    pub fn get(&self, name: &str) -> Option<&Symbol> {
        self.functions
            .get(name)
            .or_else(|| self.types.get(name))
            .or_else(|| self.constants.get(name))
    }
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Symbol)> {
        self.functions
            .iter()
            .chain(&self.types)
            .chain(&self.constants)
    }
}

pub(super) fn visible_symbols(path: &Path, source: &str) -> Symbols {
    resolve_symbols(path, source).unwrap_or_default()
}
fn resolve_symbols(path: &Path, source: &str) -> Option<Symbols> {
    let (entry, options) = crate::api::options_for_project(path).ok()?;
    let (tokens, _, lexical_errors) = crate::Lexer::new(source, 0).tokenize();
    let (partial, boundary) = crate::Parser::new_with_source(tokens, source)
        .parse_editor_file()
        .ok()?;
    if lexical_errors
        .iter()
        .any(|e| (e.span.start as usize) < boundary)
    {
        return None;
    }
    // Discovery is strict; only intelligence uses the recovered body afterward.
    let discovery_source = &source[..boundary];
    let discover = |entry: &Path| {
        resolve::resolve_modules_with_overlay(
            entry,
            options.dep_dirs.clone(),
            options.library_sources(),
            path,
            discovery_source,
        )
        .ok()
    };
    let mut modules = discover(&entry)?;
    let canonical = path.canonicalize().unwrap_or_else(|_| path.into());
    let target = |m: &resolve::ModuleInfo| {
        m.file_path
            .canonicalize()
            .unwrap_or_else(|_| m.file_path.clone())
            == canonical
    };
    if !modules.iter().any(target) {
        modules = discover(path)?;
    }
    let index = modules.iter().position(target)?;
    modules[index].file = partial;
    modules[index].source = source.into();
    let files: Vec<_> = modules.iter().map(|m| &m.file).collect();
    let scopes = scope::scopes(&files).ok()?;
    let mut symbols = Symbols::default();
    for import in &scopes[index].imports {
        add(
            &mut symbols,
            &modules[import.index],
            &options.cfg_flags,
            Some(import),
        );
    }
    add(&mut symbols, &modules[index], &options.cfg_flags, None);
    Some(symbols)
}
fn add(
    out: &mut Symbols,
    module: &resolve::ModuleInfo,
    flags: &BTreeSet<String>,
    import: Option<&scope::Import>,
) {
    let Ok(uri) = Url::from_file_path(&module.file_path) else {
        return;
    };
    let mut functions = BTreeMap::new();
    let mut types = BTreeMap::new();
    let mut constants = BTreeMap::new();
    for item in &module.file.items {
        if !item.node.cfg_active(flags) {
            continue;
        }
        let (name, public, detail, kind, parameters, map) = match &item.node {
            Item::Fn(f) => (
                &f.name,
                f.is_pub,
                format_fn_signature(f),
                CompletionItemKind::FUNCTION,
                Some(
                    f.params
                        .iter()
                        .map(|p| format!("{}: {}", p.name.node, format_ast_type(&p.ty.node)))
                        .collect(),
                ),
                &mut functions,
            ),
            Item::Struct(s) => (
                &s.name,
                s.is_pub,
                format!(
                    "struct {} {{ {} }}",
                    s.name.node,
                    s.fields
                        .iter()
                        .filter(|f| import.is_none() || f.is_pub)
                        .map(|f| format!("{}: {}", f.name.node, format_ast_type(&f.ty.node)))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                CompletionItemKind::STRUCT,
                None,
                &mut types,
            ),
            Item::Const(c) => (
                &c.name,
                c.is_pub,
                format!(
                    "const {}: {} = {}",
                    c.name.node,
                    format_ast_type(&c.ty.node),
                    format_const_value(&c.value.node)
                ),
                CompletionItemKind::CONSTANT,
                None,
                &mut constants,
            ),
            Item::Event(_) => continue,
        };
        map.insert(
            name.node.clone(),
            (
                public,
                Symbol {
                    location: Location {
                        uri: uri.clone(),
                        range: span_to_range(&module.source, name.span),
                    },
                    detail,
                    kind,
                    parameters,
                },
            ),
        );
    }
    for (finals, output) in [
        (functions, &mut out.functions),
        (types, &mut out.types),
        (constants, &mut out.constants),
    ] {
        for (name, (public, symbol)) in finals {
            if let Some(import) = import {
                if public {
                    for name in import.names(&name) {
                        output.insert(name, symbol.clone());
                    }
                }
            } else {
                output.insert(name, symbol);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn write(root: &Path, path: &str, source: &str) {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, source).unwrap();
    }
    #[test]
    fn editor_bindings_follow_direct_use_order_and_live_overlay() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a/common.tri", "module a.common pub fn retained()->Field{7} pub fn same()->Field{7} pub fn hidden()->Field{7} fn hidden()->Field{9} #[cfg(absent)] pub fn disabled(){} pub struct S{pub x:Field} struct S{pub x:Field}");
        write(
            dir.path(),
            "b/common.tri",
            "module b.common pub fn same(x:Field)->Field{x}",
        );
        write(
            dir.path(),
            "facade.tri",
            "module facade use a.common pub fn forwarded()->Field{common.retained()}",
        );
        let path = dir.path().join("entry.tri");
        write(
            dir.path(),
            "entry.tri",
            "program app use facade fn main()->Field{7}",
        );
        let hidden = visible_symbols(&path, "program app use facade fn main()->Field{7}");
        for name in [
            "retained",
            "common.retained",
            "a.common.retained",
            "common.hidden",
            "common.disabled",
            "common.S",
        ] {
            assert!(hidden.get(name).is_none(), "{name}");
        }
        assert!(hidden.get("facade.forwarded").is_some());
        let later = visible_symbols(
            &path,
            "program app use a.common use b.common fn main()->Field{7}",
        );
        assert!(later.functions["common.same"]
            .location
            .uri
            .path()
            .ends_with("b/common.tri"));
        assert!(later.functions.contains_key("common.retained"));
        for name in ["common.hidden", "common.disabled", "common.S"] {
            assert!(later.get(name).is_none(), "{name}");
        }
        let repeated = visible_symbols(
            &path,
            "program app use a.common use b.common use a.common fn main()->Field{7}",
        );
        assert!(repeated.functions["common.same"]
            .location
            .uri
            .path()
            .ends_with("a/common.tri"));
        assert_eq!(
            repeated.functions["common.same"]
                .parameters
                .as_ref()
                .unwrap()
                .len(),
            0
        );
        assert_eq!(
            later.functions["common.same"]
                .parameters
                .as_ref()
                .unwrap()
                .len(),
            1
        );
        assert!(repeated.get("main").is_some());
        assert!(repeated.get("app.main").is_none());
        for suffix in ["common.", "common.same("] {
            let source = format!("program app use b.common fn main(){{{suffix}");
            let incomplete = visible_symbols(&path, &source);
            assert!(incomplete.functions["common.same"]
                .location
                .uri
                .path()
                .ends_with("b/common.tri"));
            assert!(!incomplete.functions.contains_key("common.retained"));
        }
        assert!(visible_symbols(&path, "program app use b. fn main(){}")
            .iter()
            .next()
            .is_none());
    }
}
