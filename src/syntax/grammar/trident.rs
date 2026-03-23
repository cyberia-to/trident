// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use super::{dsl::*, Grammar, Node};

pub fn trident_grammar() -> Grammar {
    Grammar {
        name: "trident",
        word: "identifier",
        rules: rules(),
        extras: vec![pattern("\\s"), sym("line_comment")],
    }
}

fn rules() -> Vec<(&'static str, Node)> {
    vec![
        // ---- Source file ----
        (
            "source_file",
            seq(vec![
                sym("_file_header"),
                repeat(sym("use_declaration")),
                repeat(sym("io_declaration")),
                repeat(sym("_item")),
            ]),
        ),
        // ---- File header ----
        (
            "_file_header",
            choice(vec![sym("program_declaration"), sym("module_declaration")]),
        ),
        (
            "program_declaration",
            seq(vec![str_("program"), field("name", sym("identifier"))]),
        ),
        (
            "module_declaration",
            seq(vec![str_("module"), field("name", sym("module_path"))]),
        ),
        // ---- Use declarations ----
        (
            "use_declaration",
            seq(vec![str_("use"), sym("module_path")]),
        ),
        (
            "module_path",
            prec_left(
                0,
                seq(vec![
                    sym("identifier"),
                    repeat(seq(vec![str_("."), sym("identifier")])),
                ]),
            ),
        ),
        // ---- I/O declarations ----
        (
            "io_declaration",
            choice(vec![
                sym("pub_io_declaration"),
                sym("sec_io_declaration"),
                sym("sec_ram_declaration"),
            ]),
        ),
        (
            "pub_io_declaration",
            seq(vec![
                str_("pub"),
                field("kind", alias(sym("identifier"), "io_kind", true)),
                str_(":"),
                field("type", sym("_type")),
            ]),
        ),
        (
            "sec_io_declaration",
            prec(
                1,
                seq(vec![
                    str_("sec"),
                    field("kind", alias(sym("identifier"), "io_kind", true)),
                    str_(":"),
                    field("type", sym("_type")),
                ]),
            ),
        ),
        (
            "sec_ram_declaration",
            prec(
                2,
                seq(vec![
                    str_("sec"),
                    str_("ram"),
                    str_(":"),
                    str_("{"),
                    optional(comma_sep1("ram_entry")),
                    optional(str_(",")),
                    str_("}"),
                ]),
            ),
        ),
        (
            "ram_entry",
            seq(vec![
                field("address", sym("integer_literal")),
                str_(":"),
                field("type", sym("_type")),
            ]),
        ),
        // ---- Items ----
        (
            "_item",
            choice(vec![
                sym("const_definition"),
                sym("struct_definition"),
                sym("event_definition"),
                sym("function_definition"),
            ]),
        ),
        (
            "const_definition",
            seq(vec![
                optional(str_("pub")),
                str_("const"),
                field("name", sym("identifier")),
                str_(":"),
                field("type", sym("_type")),
                str_("="),
                field("value", sym("_expression")),
            ]),
        ),
        (
            "struct_definition",
            seq(vec![
                optional(str_("pub")),
                str_("struct"),
                field("name", sym("identifier")),
                str_("{"),
                optional(comma_sep1("struct_field")),
                optional(str_(",")),
                str_("}"),
            ]),
        ),
        (
            "struct_field",
            seq(vec![
                optional(str_("pub")),
                field("name", sym("identifier")),
                str_(":"),
                field("type", sym("_type")),
            ]),
        ),
        (
            "event_definition",
            seq(vec![
                str_("event"),
                field("name", sym("identifier")),
                str_("{"),
                optional(comma_sep1("event_field")),
                optional(str_(",")),
                str_("}"),
            ]),
        ),
        (
            "event_field",
            seq(vec![
                field("name", sym("identifier")),
                str_(":"),
                field("type", sym("_type")),
            ]),
        ),
        (
            "function_definition",
            seq(vec![
                optional(str_("pub")),
                optional(sym("attribute")),
                str_("fn"),
                field("name", sym("identifier")),
                str_("("),
                optional(comma_sep1("parameter")),
                optional(str_(",")),
                str_(")"),
                optional(seq(vec![str_("->"), field("return_type", sym("_type"))])),
                optional(field("body", sym("block"))),
            ]),
        ),
        (
            "attribute",
            seq(vec![
                str_("#"),
                str_("["),
                sym("identifier"),
                str_("("),
                sym("identifier"),
                str_(")"),
                str_("]"),
            ]),
        ),
        (
            "parameter",
            seq(vec![
                field("name", sym("identifier")),
                str_(":"),
                field("type", sym("_type")),
            ]),
        ),
        // ---- Types ----
        (
            "_type",
            choice(vec![
                sym("primitive_type"),
                sym("array_type"),
                sym("tuple_type"),
                sym("named_type"),
            ]),
        ),
        (
            "primitive_type",
            choice(vec![
                str_("Field"),
                str_("XField"),
                str_("Bool"),
                str_("U32"),
                str_("Digest"),
            ]),
        ),
        (
            "array_type",
            seq(vec![
                str_("["),
                field("element", sym("_type")),
                str_(";"),
                field("size", sym("integer_literal")),
                str_("]"),
            ]),
        ),
        (
            "tuple_type",
            seq(vec![
                str_("("),
                sym("_type"),
                repeat1(seq(vec![str_(","), sym("_type")])),
                optional(str_(",")),
                str_(")"),
            ]),
        ),
        ("named_type", sym("module_path")),
        // ---- Block ----
        (
            "block",
            seq(vec![
                str_("{"),
                repeat(sym("_statement")),
                optional(field("tail", sym("_expression"))),
                str_("}"),
            ]),
        ),
        // ---- Statements ----
        (
            "_statement",
            choice(vec![
                sym("let_statement"),
                sym("if_statement"),
                sym("for_statement"),
                sym("return_statement"),
                sym("match_statement"),
                sym("asm_block"),
                sym("reveal_statement"),
                sym("seal_statement"),
                sym("assignment_statement"),
                sym("expression_statement"),
            ]),
        ),
        (
            "let_statement",
            seq(vec![
                str_("let"),
                optional(str_("mut")),
                field("pattern", sym("_pattern")),
                optional(seq(vec![str_(":"), field("type", sym("_type"))])),
                str_("="),
                field("value", sym("_expression")),
            ]),
        ),
        (
            "_pattern",
            choice(vec![sym("identifier"), sym("tuple_pattern"), str_("_")]),
        ),
        (
            "tuple_pattern",
            seq(vec![
                str_("("),
                comma_sep1_inline(
                    choice(vec![sym("identifier"), str_("_")]),
                    choice(vec![sym("identifier"), str_("_")]),
                ),
                optional(str_(",")),
                str_(")"),
            ]),
        ),
        (
            "assignment_statement",
            prec(
                -1,
                seq(vec![
                    field("place", sym("_expression")),
                    str_("="),
                    field("value", sym("_expression")),
                ]),
            ),
        ),
        (
            "if_statement",
            seq(vec![
                str_("if"),
                field("condition", sym("_expression")),
                field("then", sym("block")),
                optional(seq(vec![
                    str_("else"),
                    field("else", choice(vec![sym("if_statement"), sym("block")])),
                ])),
            ]),
        ),
        (
            "for_statement",
            seq(vec![
                str_("for"),
                field("variable", choice(vec![sym("identifier"), str_("_")])),
                str_("in"),
                field("start", sym("_expression")),
                str_(".."),
                field("end", sym("_expression")),
                optional(seq(vec![
                    str_("bounded"),
                    field("bound", sym("integer_literal")),
                ])),
                field("body", sym("block")),
            ]),
        ),
        (
            "return_statement",
            prec_left(0, seq(vec![str_("return"), optional(sym("_expression"))])),
        ),
        (
            "match_statement",
            seq(vec![
                str_("match"),
                field("scrutinee", sym("_path_expr")),
                str_("{"),
                optional(comma_sep1("match_arm")),
                optional(str_(",")),
                str_("}"),
            ]),
        ),
        (
            "match_arm",
            seq(vec![
                field("pattern", sym("match_pattern")),
                str_("=>"),
                field("body", sym("block")),
            ]),
        ),
        (
            "match_pattern",
            choice(vec![
                sym("integer_literal"),
                sym("boolean_literal"),
                str_("_"),
            ]),
        ),
        (
            "asm_block",
            prec(
                20,
                seq(vec![
                    str_("asm"),
                    optional(sym("asm_annotation")),
                    str_("{"),
                    optional(sym("asm_body")),
                    str_("}"),
                ]),
            ),
        ),
        (
            "asm_annotation",
            seq(vec![
                str_("("),
                choice(vec![
                    seq(vec![
                        field("target", sym("identifier")),
                        str_(","),
                        field("effect", sym("asm_effect")),
                    ]),
                    field("target", sym("identifier")),
                    field("effect", sym("asm_effect")),
                ]),
                str_(")"),
            ]),
        ),
        ("asm_effect", pattern("[+-][0-9]+")),
        ("asm_body", repeat1(sym("asm_instruction"))),
        (
            "asm_instruction",
            choice(vec![
                sym("identifier"),
                sym("integer_literal"),
                sym("line_comment"),
            ]),
        ),
        (
            "reveal_statement",
            seq(vec![
                str_("reveal"),
                field("event", sym("identifier")),
                str_("{"),
                optional(comma_sep1("field_init")),
                optional(str_(",")),
                str_("}"),
            ]),
        ),
        (
            "seal_statement",
            seq(vec![
                str_("seal"),
                field("event", sym("identifier")),
                str_("{"),
                optional(comma_sep1("field_init")),
                optional(str_(",")),
                str_("}"),
            ]),
        ),
        (
            "field_init",
            seq(vec![
                field("name", sym("identifier")),
                optional(seq(vec![str_(":"), field("value", sym("_expression"))])),
            ]),
        ),
        ("expression_statement", prec(-2, sym("_expression"))),
        // ---- Expressions ----
        (
            "_expression",
            choice(vec![
                sym("integer_literal"),
                sym("boolean_literal"),
                sym("_path_expr"),
                sym("binary_expression"),
                sym("call_expression"),
                sym("index_expression"),
                sym("struct_init_expression"),
                sym("array_init_expression"),
                sym("tuple_expression"),
                sym("parenthesized_expression"),
            ]),
        ),
        ("integer_literal", pattern("[0-9]+")),
        ("boolean_literal", choice(vec![str_("true"), str_("false")])),
        ("_path_expr", sym("module_path")),
        // Binary operators
        (
            "binary_expression",
            choice(vec![
                prec_left(
                    12,
                    seq(vec![
                        field("left", sym("_expression")),
                        str_("/%"),
                        field("right", sym("_expression")),
                    ]),
                ),
                prec_left(
                    10,
                    seq(vec![
                        field("left", sym("_expression")),
                        str_("&"),
                        field("right", sym("_expression")),
                    ]),
                ),
                prec_left(
                    10,
                    seq(vec![
                        field("left", sym("_expression")),
                        str_("^"),
                        field("right", sym("_expression")),
                    ]),
                ),
                prec_left(
                    8,
                    seq(vec![
                        field("left", sym("_expression")),
                        str_("*"),
                        field("right", sym("_expression")),
                    ]),
                ),
                prec_left(
                    8,
                    seq(vec![
                        field("left", sym("_expression")),
                        str_("*."),
                        field("right", sym("_expression")),
                    ]),
                ),
                prec_left(
                    6,
                    seq(vec![
                        field("left", sym("_expression")),
                        str_("+"),
                        field("right", sym("_expression")),
                    ]),
                ),
                prec_left(
                    4,
                    seq(vec![
                        field("left", sym("_expression")),
                        str_("<"),
                        field("right", sym("_expression")),
                    ]),
                ),
                prec_left(
                    2,
                    seq(vec![
                        field("left", sym("_expression")),
                        str_("=="),
                        field("right", sym("_expression")),
                    ]),
                ),
            ]),
        ),
        (
            "call_expression",
            prec(
                15,
                seq(vec![
                    field("function", sym("module_path")),
                    str_("("),
                    optional(comma_sep1_inline(sym("_expression"), sym("_expression"))),
                    optional(str_(",")),
                    str_(")"),
                ]),
            ),
        ),
        (
            "index_expression",
            prec_left(
                16,
                seq(vec![
                    field("object", sym("_expression")),
                    str_("["),
                    field("index", sym("_expression")),
                    str_("]"),
                ]),
            ),
        ),
        (
            "struct_init_expression",
            prec(
                1,
                seq(vec![
                    field("name", sym("module_path")),
                    str_("{"),
                    optional(comma_sep1("field_init")),
                    optional(str_(",")),
                    str_("}"),
                ]),
            ),
        ),
        (
            "array_init_expression",
            seq(vec![
                str_("["),
                optional(seq(vec![
                    comma_sep1_inline(sym("_expression"), sym("_expression")),
                    optional(str_(",")),
                ])),
                str_("]"),
            ]),
        ),
        (
            "tuple_expression",
            seq(vec![
                str_("("),
                sym("_expression"),
                str_(","),
                optional(seq(vec![
                    comma_sep1_inline(sym("_expression"), sym("_expression")),
                    optional(str_(",")),
                ])),
                str_(")"),
            ]),
        ),
        (
            "parenthesized_expression",
            seq(vec![str_("("), sym("_expression"), str_(")")]),
        ),
        // ---- Terminals ----
        ("identifier", pattern("[a-zA-Z_][a-zA-Z0-9_]*")),
        ("line_comment", token(seq(vec![str_("//"), pattern(".*")]))),
    ]
}
