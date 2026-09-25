// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! LSP intelligence: hover, completion, and signature help.

use std::path::PathBuf;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;

use super::builtins::{builtin_completions, builtin_hover, builtin_signature};
use super::util::{find_call_context, text_before_dot, word_at_position};
use super::{imports, TridentLsp};

impl TridentLsp {
    fn live_source(&self, uri: &Url) -> Option<String> {
        self.documents
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(uri)
            .map(|d| d.source.clone())
    }
    pub(super) async fn do_hover(&self, uri: &Url, pos: Position) -> Result<Option<Hover>> {
        let Some(source) = self.live_source(uri) else {
            return Ok(None);
        };
        let word = word_at_position(&source, pos);
        let symbols = imports::visible_symbols(&PathBuf::from(uri.path()), &source);
        let value = symbols
            .get(&word)
            .map(|s| format!("```trident\n{}\n```", s.detail))
            .or_else(|| super::project::editor_options(uri).and_then(|o| builtin_hover(&word, &o)));
        Ok(value.map(|value| Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: None,
        }))
    }
    pub(super) async fn do_completion(
        &self,
        uri: &Url,
        pos: Position,
    ) -> Result<Option<CompletionResponse>> {
        let Some(source) = self.live_source(uri) else {
            return Ok(None);
        };
        let options = super::project::editor_options(uri);
        let mut items = Vec::new();
        if let Some(prefix) = text_before_dot(&source, pos) {
            let symbols = imports::visible_symbols(&PathBuf::from(uri.path()), &source);
            for (name, symbol) in symbols.iter() {
                if let Some((module, member)) = name.rsplit_once('.') {
                    if module == prefix {
                        items.push(CompletionItem {
                            label: member.into(),
                            kind: Some(symbol.kind),
                            detail: Some(symbol.detail.clone()),
                            ..Default::default()
                        });
                    }
                }
            }
            return Ok(Some(CompletionResponse::Array(items)));
        }
        // General completions: keywords + builtins + imported module names
        let keywords = [
            "fn", "let", "mut", "const", "struct", "event", "if", "else", "for", "in", "bounded",
            "return", "use", "pub", "reveal", "seal", "true", "false",
        ];
        for kw in &keywords {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            });
        }

        let type_kws = ["Field", "XField", "Bool", "U32", "Digest", "Noun"];
        for ty in &type_kws {
            items.push(CompletionItem {
                label: ty.to_string(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                ..Default::default()
            });
        }

        for (name, detail) in options
            .as_ref()
            .map(builtin_completions)
            .unwrap_or_default()
        {
            items.push(CompletionItem {
                label: name,
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(detail),
                ..Default::default()
            });
        }

        if let Ok(file) = crate::parse_source_silent(&source, uri.path()) {
            for use_stmt in &file.uses {
                let short = use_stmt
                    .node
                    .0
                    .last()
                    .cloned()
                    .unwrap_or_else(|| use_stmt.node.as_dotted());
                items.push(CompletionItem {
                    label: short,
                    kind: Some(CompletionItemKind::MODULE),
                    detail: Some(format!("module {}", use_stmt.node.as_dotted())),
                    ..Default::default()
                });
            }
        }

        Ok(Some(CompletionResponse::Array(items)))
    }

    pub(super) async fn do_signature_help(
        &self,
        uri: &Url,
        pos: Position,
    ) -> Result<Option<SignatureHelp>> {
        let Some(source) = self.live_source(uri) else {
            return Ok(None);
        };
        let Some((name, active)) = find_call_context(&source, pos) else {
            return Ok(None);
        };
        let symbols = imports::visible_symbols(&PathBuf::from(uri.path()), &source);
        let signature = if let Some(symbol) = symbols.functions.get(&name) {
            symbol
                .parameters
                .as_ref()
                .map(|p| (symbol.detail.clone(), p.clone()))
        } else {
            super::project::editor_options(uri)
                .and_then(|o| builtin_signature(&name, &o))
                .map(|(params, ret)| {
                    let parameters: Vec<_> =
                        params.iter().map(|(n, t)| format!("{n}: {t}")).collect();
                    let suffix = if ret.is_empty() {
                        String::new()
                    } else {
                        format!(" -> {ret}")
                    };
                    (
                        format!("fn {name}({}){suffix}", parameters.join(", ")),
                        parameters,
                    )
                })
        };
        Ok(signature.map(|(label, parameters)| SignatureHelp {
            signatures: vec![SignatureInformation {
                label,
                documentation: None,
                parameters: Some(
                    parameters
                        .into_iter()
                        .map(|label| ParameterInformation {
                            label: ParameterLabel::Simple(label),
                            documentation: None,
                        })
                        .collect(),
                ),
                active_parameter: Some(active),
            }],
            active_signature: Some(0),
            active_parameter: Some(active),
        }))
    }
}
