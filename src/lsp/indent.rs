// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use tower_lsp::lsp_types::*;

use crate::syntax::lexeme::Lexeme;
use crate::syntax::span::Spanned;

use super::util::{byte_offset_to_position, position_to_byte_offset};

const INDENT: &str = "    ";

/// Compute text edits for on-type formatting (auto-indent).
///
/// Trigger characters:
/// - `\n` — insert correct indentation on new line
/// - `}` — outdent closing brace to match its opening brace
pub(super) fn on_type_formatting(
    source: &str,
    tokens: &[Spanned<Lexeme>],
    position: Position,
    ch: &str,
) -> Option<Vec<TextEdit>> {
    match ch {
        "\n" => indent_new_line(source, tokens, position),
        "}" => outdent_closing_brace(source, tokens, position),
        _ => None,
    }
}

/// After pressing Enter, insert the correct indentation on the new line.
fn indent_new_line(
    source: &str,
    tokens: &[Spanned<Lexeme>],
    position: Position,
) -> Option<Vec<TextEdit>> {
    let offset = position_to_byte_offset(source, position)?;
    let depth = brace_depth_at(tokens, offset);
    if depth == 0 {
        return None;
    }

    let indent = INDENT.repeat(depth as usize);

    // Replace any existing whitespace at the start of the current line
    let line_start = line_start_offset(source, offset);
    let existing_ws_end = source[line_start..]
        .find(|c: char| !c.is_ascii_whitespace() || c == '\n')
        .map(|i| line_start + i)
        .unwrap_or(offset);

    Some(vec![TextEdit {
        range: Range::new(
            byte_offset_to_position(source, line_start),
            byte_offset_to_position(source, existing_ws_end),
        ),
        new_text: indent,
    }])
}

/// After typing `}`, outdent it to match the opening brace's indentation.
fn outdent_closing_brace(
    source: &str,
    tokens: &[Spanned<Lexeme>],
    position: Position,
) -> Option<Vec<TextEdit>> {
    let offset = position_to_byte_offset(source, position)?;

    // The `}` was just typed, so depth at this point already accounts for it.
    // We want the depth *after* the `}`, which is the depth *before* its
    // matching `{`. That's one less than the depth before the `}`.
    let depth_before = brace_depth_at(tokens, offset.saturating_sub(1));
    let target_depth = depth_before.saturating_sub(1);

    let indent = INDENT.repeat(target_depth as usize);

    let line_start = line_start_offset(source, offset);

    // Only adjust if the line contains only whitespace before the `}`
    let before_brace = &source[line_start..offset.saturating_sub(1)];
    if !before_brace.chars().all(|c| c.is_ascii_whitespace()) {
        return None;
    }

    let brace_end = offset;
    Some(vec![TextEdit {
        range: Range::new(
            byte_offset_to_position(source, line_start),
            byte_offset_to_position(source, brace_end.saturating_sub(1)),
        ),
        new_text: indent,
    }])
}

/// Count the nesting depth at a given byte offset by scanning tokens.
/// Each `LBrace` / `LParen` / `LBracket` increases depth;
/// each `RBrace` / `RParen` / `RBracket` decreases depth.
/// `AsmBlock` tokens are opaque — their internal braces are consumed
/// by the lexer and do not affect the count.
fn brace_depth_at(tokens: &[Spanned<Lexeme>], offset: usize) -> u32 {
    let mut depth: i32 = 0;
    for tok in tokens {
        if tok.span.start as usize >= offset {
            break;
        }
        match &tok.node {
            Lexeme::LBrace | Lexeme::LParen | Lexeme::LBracket => depth += 1,
            Lexeme::RBrace | Lexeme::RParen | Lexeme::RBracket => {
                depth = (depth - 1).max(0);
            }
            _ => {}
        }
    }
    depth.max(0) as u32
}

/// Find the byte offset of the start of the line containing `offset`.
fn line_start_offset(source: &str, offset: usize) -> usize {
    source[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::lexer::Lexer;

    fn tokens(source: &str) -> Vec<Spanned<Lexeme>> {
        Lexer::new(source, 0).tokenize().0
    }

    #[test]
    fn newline_after_open_brace_indents() {
        let src = "program test\nfn main() {\n";
        let toks = tokens(src);
        // Cursor at start of the empty line after `{`
        let pos = Position::new(2, 0);
        let edits = on_type_formatting(src, &toks, pos, "\n");
        assert!(edits.is_some());
        let edits = edits.unwrap();
        assert_eq!(edits[0].new_text, "    ");
    }

    #[test]
    fn nested_blocks_accumulate_indent() {
        let src = "program test\nfn main() {\n    if true {\n";
        let toks = tokens(src);
        let pos = Position::new(3, 0);
        let edits = on_type_formatting(src, &toks, pos, "\n");
        assert!(edits.is_some());
        assert_eq!(edits.unwrap()[0].new_text, "        ");
    }

    #[test]
    fn top_level_no_indent() {
        let src = "program test\n";
        let toks = tokens(src);
        let pos = Position::new(1, 0);
        let edits = on_type_formatting(src, &toks, pos, "\n");
        assert!(edits.is_none());
    }

    #[test]
    fn closing_brace_outdents() {
        let src = "program test\nfn main() {\n        }";
        let toks = tokens(src);
        // Cursor after the `}` on line 2
        let pos = Position::new(2, 9);
        let edits = on_type_formatting(src, &toks, pos, "}");
        assert!(edits.is_some());
        assert_eq!(edits.unwrap()[0].new_text, "");
    }

    #[test]
    fn brace_depth_ignores_asm_blocks() {
        let src = "program test\nfn main() {\n    asm { push 1 }\n";
        let toks = tokens(src);
        // After the asm block, depth should still be 1 (fn body)
        let pos = Position::new(3, 0);
        let edits = on_type_formatting(src, &toks, pos, "\n");
        assert!(edits.is_some());
        assert_eq!(edits.unwrap()[0].new_text, "    ");
    }
}
