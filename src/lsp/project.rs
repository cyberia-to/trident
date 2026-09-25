// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Project-level helpers: symbol index, exports, function costs.

use std::path::Path;

use tower_lsp::lsp_types::*;

use crate::ast::Item;
use crate::resolve::resolve_modules_with_sources;

use super::document::DocumentData;
use super::util::{format_fn_signature, span_to_range};
use super::TridentLsp;

/// Resolve editor modules through the same target package as compilation.
pub(super) fn project_modules(
    file_path: &Path,
) -> Result<
    (crate::CompileOptions, Vec<crate::resolve::ModuleInfo>),
    Vec<crate::diagnostic::Diagnostic>,
> {
    let (entry, options) = crate::api::options_for_project(file_path)?;
    let modules =
        resolve_modules_with_sources(&entry, options.dep_dirs.clone(), options.library_sources())?;
    Ok((options, modules))
}

impl TridentLsp {
    /// Collect workspace symbols from all open documents, filtered by query.
    pub(super) fn workspace_symbols(
        &self,
        query: &str,
        docs: &std::collections::BTreeMap<Url, DocumentData>,
    ) -> Vec<SymbolInformation> {
        let query_lower = query.to_lowercase();
        let mut symbols = Vec::new();

        for (uri, doc) in docs.iter() {
            let file = match crate::parse_source_silent(&doc.source, uri.path()) {
                Ok(f) => f,
                Err(_) => continue,
            };

            for item in &file.items {
                let (name, kind, name_span) = match &item.node {
                    Item::Fn(f) => (f.name.node.clone(), SymbolKind::FUNCTION, f.name.span),
                    Item::Struct(s) => (s.name.node.clone(), SymbolKind::STRUCT, s.name.span),
                    Item::Const(c) => (c.name.node.clone(), SymbolKind::CONSTANT, c.name.span),
                    Item::Event(e) => (e.name.node.clone(), SymbolKind::EVENT, e.name.span),
                };

                if !query_lower.is_empty() && !name.to_lowercase().contains(&query_lower) {
                    continue;
                }

                #[allow(deprecated)]
                symbols.push(SymbolInformation {
                    name,
                    kind,
                    tags: None,
                    deprecated: None,
                    location: Location {
                        uri: uri.clone(),
                        range: span_to_range(&doc.source, name_span),
                    },
                    container_name: None,
                });
            }
        }

        symbols
    }

    /// Build document symbols for a single file.
    pub(super) fn document_symbols(
        &self,
        source: &str,
        file: &crate::ast::File,
    ) -> Vec<DocumentSymbol> {
        let mut symbols = Vec::new();
        for item in &file.items {
            let (name, kind, detail) = match &item.node {
                Item::Fn(f) => {
                    let sig = format_fn_signature(f);
                    (f.name.node.clone(), SymbolKind::FUNCTION, Some(sig))
                }
                Item::Struct(s) => (s.name.node.clone(), SymbolKind::STRUCT, None),
                Item::Const(c) => (c.name.node.clone(), SymbolKind::CONSTANT, None),
                Item::Event(e) => (e.name.node.clone(), SymbolKind::EVENT, None),
            };

            let range = span_to_range(source, item.span);
            let selection_range = match &item.node {
                Item::Fn(f) => span_to_range(source, f.name.span),
                Item::Struct(s) => span_to_range(source, s.name.span),
                Item::Const(c) => span_to_range(source, c.name.span),
                Item::Event(e) => span_to_range(source, e.name.span),
            };

            #[allow(deprecated)]
            symbols.push(DocumentSymbol {
                name,
                detail,
                kind,
                tags: None,
                deprecated: None,
                range,
                selection_range,
                children: None,
            });
        }
        symbols
    }
}

/// Unavailable targets yield no potentially misleading builtin information.
pub(super) fn editor_options(uri: &Url) -> Option<crate::CompileOptions> {
    let path = uri.to_file_path().ok()?;
    crate::api::options_for_project(&path)
        .ok()
        .map(|(_, options)| options)
}
