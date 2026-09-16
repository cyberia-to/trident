// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use crate::syntax::span::Span;

/// Token type indices (must match mod.rs constants).
const TT_KEYWORD: u32 = 0;
const TT_VARIABLE: u32 = 3;
const TT_NUMBER: u32 = 6;
const TT_COMMENT: u32 = 7;
const TT_NAMESPACE: u32 = 9;

/// Modifier for builtin/stdlib items — distinguishes asm instructions
/// from language keywords visually.
const MOD_DEFAULT_LIBRARY: u32 = 1 << 3;

/// Expand an AsmBlock token into sub-tokens for instruction-level highlighting.
///
/// Returns `(span, token_type, modifiers)` tuples that replace the single
/// AsmBlock token in the semantic token stream.
pub(super) fn expand_asm_tokens(
    source: &str,
    block_span: Span,
    options: Option<&crate::CompileOptions>,
) -> Vec<(Span, u32, u32)> {
    let mut tokens = Vec::new();
    let mut explicit_target = None;
    let src = source.as_bytes();
    let start = block_span.start as usize;
    let end = block_span.end as usize;
    if start >= source.len() || end > source.len() {
        return tokens;
    }

    let region = &src[start..end];
    let mut pos = 0;

    // 1. `asm` keyword (always first 3 bytes)
    if region.len() >= 3 && &region[..3] == b"asm" {
        tokens.push((
            Span::new(block_span.file_id, block_span.start, block_span.start + 3),
            TT_KEYWORD,
            0,
        ));
        pos = 3;
    }

    // Skip whitespace
    while pos < region.len() && region[pos].is_ascii_whitespace() {
        pos += 1;
    }

    // 2. Optional annotation: `(target, +N)` or `(+N)` or `(target)`
    if pos < region.len() && region[pos] == b'(' {
        pos += 1; // skip '('
        skip_ws(region, &mut pos);

        // Check for identifier (target) vs effect number
        if pos < region.len() && region[pos].is_ascii_alphabetic() {
            let word_start = pos;
            while pos < region.len() && is_ident_char(region[pos]) {
                pos += 1;
            }
            explicit_target = std::str::from_utf8(&region[word_start..pos]).ok();
            tokens.push((
                span_at(block_span.file_id, start + word_start, start + pos),
                TT_NAMESPACE,
                0,
            ));
            skip_ws(region, &mut pos);

            // Optional `, +N`
            if pos < region.len() && region[pos] == b',' {
                pos += 1;
                skip_ws(region, &mut pos);
                scan_effect_token(region, &mut pos, block_span.file_id, start, &mut tokens);
            }
        } else {
            // Effect number directly
            scan_effect_token(region, &mut pos, block_span.file_id, start, &mut tokens);
        }

        // Skip to closing ')'
        while pos < region.len() && region[pos] != b')' {
            pos += 1;
        }
        if pos < region.len() {
            pos += 1; // skip ')'
        }
        skip_ws(region, &mut pos);
    }

    // 3. Body: everything between `{` and `}`
    if pos < region.len() && region[pos] == b'{' {
        pos += 1; // skip '{'

        // Find the matching closing brace (last byte before end)
        let body_end = if region.len() > 0 && region[region.len() - 1] == b'}' {
            region.len() - 1
        } else {
            region.len()
        };

        let explicit_options = explicit_target
            .filter(|target| options.map(|o| o.target_config.name.as_str()) != Some(*target))
            .and_then(|target| crate::CompileOptions::resolve(target, "debug").ok());
        let selected = if explicit_target.is_some()
            && explicit_options.is_none()
            && explicit_target != options.map(|o| o.target_config.name.as_str())
        {
            None
        } else {
            explicit_options.as_ref().or(options)
        };
        let instructions = selected
            .and_then(|o| o.target_package.as_ref())
            .map(|p| p.instructions.as_slice())
            .unwrap_or(&[]);

        // Tokenize the body
        tokenize_asm_body(
            region,
            pos,
            body_end,
            block_span.file_id,
            start,
            &mut tokens,
            instructions,
        );
    }

    tokens
}

/// Tokenize the asm body content between braces.
fn tokenize_asm_body(
    region: &[u8],
    mut pos: usize,
    end: usize,
    file_id: u16,
    abs_start: usize,
    tokens: &mut Vec<(Span, u32, u32)>,
    instructions: &[String],
) {
    while pos < end {
        // Skip whitespace
        if region[pos].is_ascii_whitespace() {
            pos += 1;
            continue;
        }

        // Line comment
        if pos + 1 < end && region[pos] == b'/' && region[pos + 1] == b'/' {
            let comment_start = pos;
            while pos < end && region[pos] != b'\n' {
                pos += 1;
            }
            tokens.push((
                span_at(file_id, abs_start + comment_start, abs_start + pos),
                TT_COMMENT,
                0,
            ));
            continue;
        }

        // Integer literal (possibly negative)
        if region[pos].is_ascii_digit()
            || (region[pos] == b'-' && pos + 1 < end && region[pos + 1].is_ascii_digit())
        {
            let num_start = pos;
            if region[pos] == b'-' {
                pos += 1;
            }
            while pos < end && region[pos].is_ascii_digit() {
                pos += 1;
            }
            tokens.push((
                span_at(file_id, abs_start + num_start, abs_start + pos),
                TT_NUMBER,
                0,
            ));
            continue;
        }

        // Identifier or instruction
        if region[pos].is_ascii_alphabetic() || region[pos] == b'_' {
            let word_start = pos;
            while pos < end && is_ident_char(region[pos]) {
                pos += 1;
            }
            let word = std::str::from_utf8(&region[word_start..pos]).unwrap_or("");
            let (tt, mods) = if instructions.iter().any(|instruction| instruction == word) {
                (TT_KEYWORD, MOD_DEFAULT_LIBRARY)
            } else {
                (TT_VARIABLE, 0)
            };
            tokens.push((
                span_at(file_id, abs_start + word_start, abs_start + pos),
                tt,
                mods,
            ));
            continue;
        }

        // Skip unknown characters
        pos += 1;
    }
}

fn scan_effect_token(
    region: &[u8],
    pos: &mut usize,
    file_id: u16,
    abs_start: usize,
    tokens: &mut Vec<(Span, u32, u32)>,
) {
    if *pos >= region.len() {
        return;
    }
    let eff_start = *pos;
    if region[*pos] == b'+' || region[*pos] == b'-' {
        *pos += 1;
    }
    while *pos < region.len() && region[*pos].is_ascii_digit() {
        *pos += 1;
    }
    if *pos > eff_start {
        tokens.push((
            Span::new(
                file_id,
                (abs_start + eff_start) as u32,
                (abs_start + *pos) as u32,
            ),
            TT_NUMBER,
            0,
        ));
    }
}

fn skip_ws(region: &[u8], pos: &mut usize) {
    while *pos < region.len() && region[*pos].is_ascii_whitespace() {
        *pos += 1;
    }
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn span_at(file_id: u16, abs_start: usize, abs_end: usize) -> Span {
    Span::new(file_id, abs_start as u32, abs_end as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supplied_instruction_set_controls_highlighting() {
        let mut tokens = Vec::new();
        let source = b"owned_op push";
        tokenize_asm_body(
            source,
            0,
            source.len(),
            0,
            0,
            &mut tokens,
            &["owned_op".into()],
        );
        assert_eq!(tokens[0].1, TT_KEYWORD);
        assert_eq!(tokens[0].2, MOD_DEFAULT_LIBRARY);
        assert_eq!(tokens[1].1, TT_VARIABLE);
        assert_eq!(tokens[1].2, 0);
    }
}
