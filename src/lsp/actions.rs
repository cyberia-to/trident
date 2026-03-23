// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! LSP code actions: quick fixes triggered by diagnostics.

use std::collections::BTreeMap;

use tower_lsp::lsp_types::*;

use super::util::byte_offset_to_position;

/// Build code actions for diagnostics in the given range.
pub(super) fn code_actions(
    source: &str,
    diagnostics: &[Diagnostic],
    uri: &Url,
) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();
    for diag in diagnostics {
        let msg = first_line(&diag.message);
        if msg.starts_with("unused import '") {
            if let Some(a) = remove_unused_import(diag, uri) {
                actions.push(CodeActionOrCommand::CodeAction(a));
            }
        } else if msg == "cannot assign to immutable variable" {
            if let Some(a) = add_mut_keyword(source, diag, uri) {
                actions.push(CodeActionOrCommand::CodeAction(a));
            }
        } else if msg.starts_with("hint[H0003]: as_u32(") {
            if let Some(a) = remove_redundant_as_u32(diag, uri) {
                actions.push(CodeActionOrCommand::CodeAction(a));
            }
        } else if msg.starts_with("missing field '") {
            if let Some(a) = insert_missing_field(source, diag, uri) {
                actions.push(CodeActionOrCommand::CodeAction(a));
            }
        }
    }
    actions
}

fn first_line(msg: &str) -> &str {
    msg.split('\n').next().unwrap_or(msg)
}

/// Delete the entire line containing the unused import.
fn remove_unused_import(diag: &Diagnostic, uri: &Url) -> Option<CodeAction> {
    let import_name = extract_quoted(&diag.message)?;
    let line = diag.range.start.line;

    // Find line boundaries to delete entire line including newline
    let line_start = Position::new(line, 0);
    let line_end = Position::new(line + 1, 0);

    let edit = TextEdit {
        range: Range::new(line_start, line_end),
        new_text: String::new(),
    };

    Some(make_quickfix(
        format!("Remove unused import '{}'", import_name),
        uri,
        vec![edit],
        diag,
    ))
}

/// Insert `mut ` into the `let` declaration of the variable.
fn add_mut_keyword(source: &str, diag: &Diagnostic, uri: &Url) -> Option<CodeAction> {
    // The diagnostic span is on the assignment site (variable name).
    // We need to find the original `let varname` declaration.
    let assign_start = position_to_byte(source, diag.range.start)?;
    let assign_end = position_to_byte(source, diag.range.end)?;
    let var_name = source.get(assign_start..assign_end)?;

    // Scan backwards for `let <varname>` (without `mut`)
    let let_pattern = format!("let {}", var_name);
    let let_mut_pattern = format!("let mut {}", var_name);

    // Search from beginning of source for the pattern
    if let Some(pos) = source.find(&let_pattern) {
        // Make sure it's not already `let mut`
        if source[pos..].starts_with(&let_mut_pattern) {
            return None;
        }
        let insert_offset = pos + 4; // after "let "
        let insert_pos = byte_offset_to_position(source, insert_offset);

        let edit = TextEdit {
            range: Range::new(insert_pos, insert_pos),
            new_text: "mut ".to_string(),
        };

        return Some(make_quickfix(
            format!("Add `mut` to declaration of `{}`", var_name),
            uri,
            vec![edit],
            diag,
        ));
    }

    None
}

/// Replace `as_u32(x)` with just `x`.
fn remove_redundant_as_u32(diag: &Diagnostic, uri: &Url) -> Option<CodeAction> {
    let msg = first_line(&diag.message);
    // Extract variable name from "hint[H0003]: as_u32(X) is redundant..."
    let start = msg.find("as_u32(")? + 7;
    let end = msg[start..].find(')')? + start;
    let var_name = &msg[start..end];

    let edit = TextEdit {
        range: diag.range,
        new_text: var_name.to_string(),
    };

    Some(make_quickfix(
        format!("Remove redundant `as_u32({})`", var_name),
        uri,
        vec![edit],
        diag,
    ))
}

/// Add missing field with a zero default before the closing `}`.
fn insert_missing_field(source: &str, diag: &Diagnostic, uri: &Url) -> Option<CodeAction> {
    let field_name = extract_quoted(&diag.message)?;

    // Find closing `}` at the end of the diagnostic range
    let end_offset = position_to_byte(source, diag.range.end)?;
    // Search backwards from end for `}`
    let brace_offset = source[..end_offset].rfind('}')?;

    // Determine indentation from the line above the `}`
    let line_before_brace = source[..brace_offset]
        .rfind('\n')
        .map(|i| i + 1)
        .unwrap_or(0);
    let brace_line = &source[line_before_brace..brace_offset];
    let indent = &brace_line[..brace_line.len() - brace_line.trim_start().len()];
    // Field indent is one level deeper than the brace
    let field_indent = format!("{}    ", indent);

    let insert_pos = byte_offset_to_position(source, brace_offset);
    let edit = TextEdit {
        range: Range::new(insert_pos, insert_pos),
        new_text: format!("{}{}: 0,\n", field_indent, field_name),
    };

    Some(make_quickfix(
        format!("Add missing field `{}`", field_name),
        uri,
        vec![edit],
        diag,
    ))
}

/// Build a quickfix CodeAction.
fn make_quickfix(title: String, uri: &Url, edits: Vec<TextEdit>, diag: &Diagnostic) -> CodeAction {
    let mut changes = BTreeMap::new();
    changes.insert(uri.clone(), edits);
    // Convert BTreeMap to HashMap for WorkspaceEdit
    let changes: std::collections::HashMap<Url, Vec<TextEdit>> = changes.into_iter().collect();

    CodeAction {
        title,
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diag.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }),
        is_preferred: Some(true),
        ..Default::default()
    }
}

/// Extract text between first pair of single quotes.
fn extract_quoted(msg: &str) -> Option<String> {
    let start = msg.find('\'')?;
    let end = msg[start + 1..].find('\'')?;
    Some(msg[start + 1..start + 1 + end].to_string())
}

/// Convert an LSP Position to a byte offset.
fn position_to_byte(source: &str, pos: Position) -> Option<usize> {
    super::util::position_to_byte_offset(source, pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_uri() -> Url {
        Url::parse("file:///test.tri").unwrap()
    }

    fn make_diag(msg: &str, start: (u32, u32), end: (u32, u32)) -> Diagnostic {
        Diagnostic {
            range: Range::new(Position::new(start.0, start.1), Position::new(end.0, end.1)),
            severity: Some(DiagnosticSeverity::WARNING),
            source: Some("trident".to_string()),
            message: msg.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn remove_unused_import_action() {
        let source = "program test\nuse std.hash\nfn main() {}\n";
        let diag = make_diag("unused import 'std.hash'", (1, 4), (1, 12));
        let actions = code_actions(source, &[diag], &test_uri());
        assert_eq!(actions.len(), 1);
        let action = match &actions[0] {
            CodeActionOrCommand::CodeAction(a) => a,
            _ => panic!("expected CodeAction"),
        };
        assert!(action.title.contains("Remove unused import"));
        let edit = action.edit.as_ref().unwrap();
        let changes = edit.changes.as_ref().unwrap();
        let edits = &changes[&test_uri()];
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "");
        // Should delete line 1 entirely
        assert_eq!(edits[0].range.start.line, 1);
        assert_eq!(edits[0].range.end.line, 2);
    }

    #[test]
    fn add_mut_keyword_action() {
        let source = "program test\nfn main() {\n  let x: Field = 1\n  x = 2\n}\n";
        // Diagnostic on the assignment `x` at line 3, col 2-3
        let diag = make_diag(
            "cannot assign to immutable variable\nhelp: declare the variable with `let mut` to make it mutable",
            (3, 2),
            (3, 3),
        );
        let actions = code_actions(source, &[diag], &test_uri());
        assert_eq!(actions.len(), 1);
        let action = match &actions[0] {
            CodeActionOrCommand::CodeAction(a) => a,
            _ => panic!("expected CodeAction"),
        };
        assert!(action.title.contains("mut"));
        let edit = action.edit.as_ref().unwrap();
        let edits = &edit.changes.as_ref().unwrap()[&test_uri()];
        assert_eq!(edits[0].new_text, "mut ");
    }

    #[test]
    fn remove_redundant_as_u32_action() {
        let source =
            "program test\nfn main() {\n  let a = as_u32(pub_read())\n  let b = as_u32(a)\n}\n";
        // Diagnostic on `as_u32(a)` at line 3
        let diag = make_diag(
            "hint[H0003]: as_u32(a) is redundant — value is already proven U32",
            (3, 10),
            (3, 20),
        );
        let actions = code_actions(source, &[diag], &test_uri());
        assert_eq!(actions.len(), 1);
        let action = match &actions[0] {
            CodeActionOrCommand::CodeAction(a) => a,
            _ => panic!("expected CodeAction"),
        };
        assert!(action.title.contains("Remove redundant"));
        let edit = action.edit.as_ref().unwrap();
        let edits = &edit.changes.as_ref().unwrap()[&test_uri()];
        assert_eq!(edits[0].new_text, "a");
    }

    #[test]
    fn insert_missing_field_action() {
        let source = "program test\nstruct Foo {\n  x: Field,\n  y: Field,\n}\nfn main() {\n  let f = Foo { x: 1 }\n}\n";
        // Diagnostic on `Foo { x: 1 }` — span covers the struct init
        let diag = make_diag("missing field 'y' in struct init", (6, 10), (6, 22));
        let actions = code_actions(source, &[diag], &test_uri());
        assert_eq!(actions.len(), 1);
        let action = match &actions[0] {
            CodeActionOrCommand::CodeAction(a) => a,
            _ => panic!("expected CodeAction"),
        };
        assert!(action.title.contains("Add missing field `y`"));
    }
}
