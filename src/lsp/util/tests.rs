// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use super::*;
use tower_lsp::lsp_types::Position;

// --- byte_offset_to_position ---

#[test]
fn test_byte_offset_first_line() {
    let src = "let x = 1\n";
    assert_eq!(byte_offset_to_position(src, 0), Position::new(0, 0));
    assert_eq!(byte_offset_to_position(src, 4), Position::new(0, 4));
}

#[test]
fn test_byte_offset_second_line() {
    let src = "let x = 1\nlet y = 2\n";
    assert_eq!(byte_offset_to_position(src, 10), Position::new(1, 0));
    assert_eq!(byte_offset_to_position(src, 14), Position::new(1, 4));
}

#[test]
fn test_byte_offset_clamps() {
    let src = "abc";
    let pos = byte_offset_to_position(src, 999);
    assert_eq!(pos, Position::new(0, 3));
}

// --- position_to_byte_offset ---

#[test]
fn test_position_to_offset_start() {
    let src = "let x = 1\nlet y = 2\n";
    assert_eq!(position_to_byte_offset(src, Position::new(0, 0)), Some(0));
    assert_eq!(position_to_byte_offset(src, Position::new(0, 4)), Some(4));
    assert_eq!(position_to_byte_offset(src, Position::new(1, 0)), Some(10));
    assert_eq!(position_to_byte_offset(src, Position::new(1, 4)), Some(14));
}

#[test]
fn test_position_to_offset_end_of_line() {
    let src = "abc\ndef\n";
    assert_eq!(position_to_byte_offset(src, Position::new(0, 3)), Some(3));
}

#[test]
fn test_position_to_offset_past_end() {
    let src = "abc";
    assert_eq!(position_to_byte_offset(src, Position::new(5, 0)), None);
}

// --- word_at_position ---

#[test]
fn test_word_simple() {
    let src = "let foo = bar\n";
    assert_eq!(word_at_position(src, Position::new(0, 4)), "foo");
    assert_eq!(word_at_position(src, Position::new(0, 10)), "bar");
}

#[test]
fn test_word_at_start() {
    let src = "hello world\n";
    assert_eq!(word_at_position(src, Position::new(0, 0)), "hello");
}

#[test]
fn test_word_qualified_after_dot() {
    let src = "let x = hash.tip5()\n";
    assert_eq!(word_at_position(src, Position::new(0, 14)), "hash.tip5");
}

#[test]
fn test_word_qualified_before_dot() {
    let src = "let x = hash.tip5()\n";
    assert_eq!(word_at_position(src, Position::new(0, 9)), "hash.tip5");
}

#[test]
fn test_word_on_boundary_picks_left() {
    let src = "let x = 1\n";
    assert_eq!(word_at_position(src, Position::new(0, 3)), "let");
}

#[test]
fn test_word_between_symbols_empty() {
    let src = "a = b\n";
    assert_eq!(word_at_position(src, Position::new(0, 2)), "");
}

// --- text_before_dot ---

#[test]
fn test_dot_completion_prefix() {
    let src = "hash.t";
    assert_eq!(
        text_before_dot(src, Position::new(0, 6)),
        Some("hash".to_string())
    );
}

#[test]
fn test_dot_completion_right_after_dot() {
    let src = "hash.";
    assert_eq!(
        text_before_dot(src, Position::new(0, 5)),
        Some("hash".to_string())
    );
}

#[test]
fn test_no_dot_prefix() {
    let src = "let x = 1";
    assert_eq!(text_before_dot(src, Position::new(0, 5)), None);
}

// --- span_to_range ---

#[test]
fn test_span_to_range_single_line() {
    let src = "let foo = 1\n";
    let span = crate::span::Span::new(0, 4, 7);
    let range = span_to_range(src, span);
    assert_eq!(range.start, Position::new(0, 4));
    assert_eq!(range.end, Position::new(0, 7));
}

#[test]
fn test_span_to_range_multi_line() {
    let src = "line1\nline2\nline3\n";
    let span = crate::span::Span::new(0, 6, 17);
    let range = span_to_range(src, span);
    assert_eq!(range.start, Position::new(1, 0));
    assert_eq!(range.end, Position::new(2, 5));
}

// --- to_lsp_diagnostic ---

#[test]
fn test_lsp_diagnostic_error() {
    let source = "let x: U32 = pub_read()\n";
    let diag = crate::diagnostic::Diagnostic::error(
        "type mismatch".to_string(),
        crate::span::Span::new(0, 13, 23),
    )
    .with_note("expected U32, found Field".to_string());

    let lsp_diag = to_lsp_diagnostic(&diag, source);
    assert_eq!(lsp_diag.severity, Some(DiagnosticSeverity::ERROR));
    assert!(lsp_diag.message.contains("type mismatch"));
    assert!(lsp_diag.message.contains("note: expected U32, found Field"));
    assert_eq!(lsp_diag.source, Some("trident".to_string()));
}

#[test]
fn test_lsp_diagnostic_warning_with_help() {
    let source = "as_u32(x)\n";
    let diag = crate::diagnostic::Diagnostic::warning(
        "redundant".to_string(),
        crate::span::Span::new(0, 0, 9),
    )
    .with_help("already proven".to_string());

    let lsp_diag = to_lsp_diagnostic(&diag, source);
    assert_eq!(lsp_diag.severity, Some(DiagnosticSeverity::WARNING));
    assert!(lsp_diag.message.contains("help: already proven"));
}

// --- format_fn_signature ---

#[test]
fn test_format_fn_signature_no_params() {
    let f = crate::ast::FnDef {
        is_pub: false,
        is_test: false,
        is_pure: false,
        cfg: None,
        intrinsic: None,
        requires: vec![],
        ensures: vec![],
        name: crate::span::Spanned::dummy("main".to_string()),
        type_params: vec![],
        params: vec![],
        return_ty: None,
        body: None,
    };
    assert_eq!(format_fn_signature(&f), "fn main()");
}

#[test]
fn test_format_fn_signature_with_return() {
    let f = crate::ast::FnDef {
        is_pub: true,
        is_test: false,
        is_pure: false,
        cfg: None,
        intrinsic: None,
        requires: vec![],
        ensures: vec![],
        name: crate::span::Spanned::dummy("add".to_string()),
        type_params: vec![],
        params: vec![
            crate::ast::Param {
                name: crate::span::Spanned::dummy("a".to_string()),
                ty: crate::span::Spanned::dummy(crate::ast::Type::Field),
            },
            crate::ast::Param {
                name: crate::span::Spanned::dummy("b".to_string()),
                ty: crate::span::Spanned::dummy(crate::ast::Type::Field),
            },
        ],
        return_ty: Some(crate::span::Spanned::dummy(crate::ast::Type::Field)),
        body: None,
    };
    assert_eq!(
        format_fn_signature(&f),
        "fn add(a: Field, b: Field) -> Field"
    );
}

// --- format_ast_type ---

#[test]
fn test_format_ast_types() {
    assert_eq!(format_ast_type(&crate::ast::Type::Field), "Field");
    assert_eq!(format_ast_type(&crate::ast::Type::XField), "XField");
    assert_eq!(format_ast_type(&crate::ast::Type::Bool), "Bool");
    assert_eq!(format_ast_type(&crate::ast::Type::U32), "U32");
    assert_eq!(format_ast_type(&crate::ast::Type::Digest), "Digest");
    assert_eq!(
        format_ast_type(&crate::ast::Type::Array(
            Box::new(crate::ast::Type::Field),
            crate::ast::ArraySize::Literal(5)
        )),
        "[Field; 5]"
    );
    assert_eq!(
        format_ast_type(&crate::ast::Type::Tuple(vec![
            crate::ast::Type::Field,
            crate::ast::Type::U32
        ])),
        "(Field, U32)"
    );
}

// --- is_ident_char ---

#[test]
fn test_ident_chars() {
    assert!(is_ident_char(b'a'));
    assert!(is_ident_char(b'Z'));
    assert!(is_ident_char(b'0'));
    assert!(is_ident_char(b'_'));
    assert!(!is_ident_char(b'.'));
    assert!(!is_ident_char(b' '));
    assert!(!is_ident_char(b'('));
}

// --- find_call_context ---

#[test]
fn test_find_call_context_simple() {
    let src = "pub_write(x, y)";
    let ctx = find_call_context(src, Position::new(0, 12));
    assert_eq!(ctx, Some(("pub_write".to_string(), 1)));
}

#[test]
fn test_find_call_context_first_param() {
    let src = "pub_write(x)";
    let ctx = find_call_context(src, Position::new(0, 10));
    assert_eq!(ctx, Some(("pub_write".to_string(), 0)));
}

#[test]
fn test_find_call_context_no_paren() {
    let src = "let x = 1";
    let ctx = find_call_context(src, Position::new(0, 5));
    assert_eq!(ctx, None);
}

#[test]
fn test_find_call_context_nested() {
    let src = "split(field_add(a, b))";
    let ctx = find_call_context(src, Position::new(0, 19));
    assert_eq!(ctx, Some(("field_add".to_string(), 1)));
}

#[test]
fn test_find_call_context_qualified_name() {
    let src = "math.add(x, y, z)";
    let ctx = find_call_context(src, Position::new(0, 15));
    assert_eq!(ctx, Some(("math.add".to_string(), 2)));
}

#[test]
fn test_find_call_context_right_after_open_paren() {
    let src = "assert(";
    let ctx = find_call_context(src, Position::new(0, 7));
    assert_eq!(ctx, Some(("assert".to_string(), 0)));
}

#[test]
fn test_find_call_context_space_before_paren() {
    let src = "foo (a, b)";
    let ctx = find_call_context(src, Position::new(0, 8));
    assert_eq!(ctx, Some(("foo".to_string(), 1)));
}

// --- format_cost_inline ---

#[test]
fn test_format_cost_inline_zero() {
    let cost = crate::cost::TableCost::from_slice(&[0, 0, 0, 0, 0, 0]);
    let s = format_cost_inline(&cost);
    assert!(s.contains("cc=0"), "should contain cc=0, got: {}", s);
    assert!(
        s.contains("dominant:"),
        "should contain dominant label, got: {}",
        s
    );
}

#[test]
fn test_format_cost_inline_hash_dominant() {
    let cost = crate::cost::TableCost::from_slice(&[1, 6, 0, 1, 0, 0]);
    let s = format_cost_inline(&cost);
    assert!(s.contains("cc=1"), "should contain cc=1, got: {}", s);
    assert!(s.contains("hash=6"), "should contain hash=6, got: {}", s);
    assert!(
        s.contains("dominant: hash"),
        "dominant should be hash, got: {}",
        s
    );
    assert!(
        !s.contains("u32="),
        "zero u32 should be omitted, got: {}",
        s
    );
}
