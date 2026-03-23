// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Builtin function metadata for LSP hover, signature help, and completions.

/// Hover info for builtin functions.
pub fn builtin_hover(name: &str) -> Option<String> {
    let info = match name {
        "pub_read" => "```trident\nfn pub_read() -> Field\n```\nRead one field element from public input.",
        "pub_read2" => "```trident\nfn pub_read2() -> (Field, Field)\n```\nRead 2 field elements from public input.",
        "pub_read3" => "```trident\nfn pub_read3() -> (Field, Field, Field)\n```\nRead 3 field elements from public input.",
        "pub_read4" => "```trident\nfn pub_read4() -> (Field, Field, Field, Field)\n```\nRead 4 field elements from public input.",
        "pub_read5" => "```trident\nfn pub_read5() -> Digest\n```\nRead 5 field elements (Digest) from public input.",
        "pub_write" => "```trident\nfn pub_write(v: Field)\n```\nWrite one field element to public output.",
        "pub_write2" => "```trident\nfn pub_write2(a: Field, b: Field)\n```\nWrite 2 field elements to public output.",
        "pub_write3" => "```trident\nfn pub_write3(a: Field, b: Field, c: Field)\n```\nWrite 3 field elements to public output.",
        "pub_write4" => "```trident\nfn pub_write4(a: Field, b: Field, c: Field, d: Field)\n```\nWrite 4 field elements to public output.",
        "pub_write5" => "```trident\nfn pub_write5(a: Field, b: Field, c: Field, d: Field, e: Field)\n```\nWrite 5 field elements to public output.",
        "divine" => "```trident\nfn divine() -> Field\n```\nRead one non-deterministic field element (secret witness).",
        "divine3" => "```trident\nfn divine3() -> (Field, Field, Field)\n```\nRead 3 non-deterministic field elements.",
        "divine5" => "```trident\nfn divine5() -> Digest\n```\nRead 5 non-deterministic field elements (Digest).",
        "assert" => "```trident\nfn assert(cond: Bool)\n```\nAbort execution if condition is false.",
        "assert_eq" => "```trident\nfn assert_eq(a: Field, b: Field)\n```\nAbort execution if a != b.",
        "assert_digest_eq" => "```trident\nfn assert_digest_eq(a: Digest, b: Digest)\n```\nAbort execution if digests are not equal.",
        "hash" => "```trident\nfn hash(x0..x9: Field) -> Digest\n```\nTip5 hash of 10 field elements.",
        "sponge_init" => "```trident\nfn sponge_init()\n```\nInitialize the Tip5 sponge state.",
        "sponge_absorb" => "```trident\nfn sponge_absorb(x0..x9: Field)\n```\nAbsorb 10 field elements into the sponge.",
        "sponge_squeeze" => "```trident\nfn sponge_squeeze() -> [Field; 10]\n```\nSqueeze 10 field elements from the sponge.",
        "split" => "```trident\nfn split(a: Field) -> (U32, U32)\n```\nSplit field element into (hi, lo) u32 limbs.",
        "log2" => "```trident\nfn log2(a: U32) -> U32\n```\nFloor of log base 2.",
        "pow" => "```trident\nfn pow(base: U32, exp: U32) -> U32\n```\nInteger exponentiation.",
        "popcount" => "```trident\nfn popcount(a: U32) -> U32\n```\nCount set bits.",
        "as_u32" => "```trident\nfn as_u32(a: Field) -> U32\n```\nRange-check and convert field to u32.",
        "as_field" => "```trident\nfn as_field(a: U32) -> Field\n```\nConvert u32 to field element.",
        "field_add" => "```trident\nfn field_add(a: Field, b: Field) -> Field\n```\nField addition.",
        "field_mul" => "```trident\nfn field_mul(a: Field, b: Field) -> Field\n```\nField multiplication.",
        "inv" => "```trident\nfn inv(a: Field) -> Field\n```\nField multiplicative inverse.",
        "neg" => "```trident\nfn neg(a: Field) -> Field\n```\nField negation.",
        "sub" => "```trident\nfn sub(a: Field, b: Field) -> Field\n```\nField subtraction.",
        "ram_read" => "```trident\nfn ram_read(addr: Field) -> Field\n```\nRead one field element from RAM.",
        "ram_write" => "```trident\nfn ram_write(addr: Field, val: Field)\n```\nWrite one field element to RAM.",
        "ram_read_block" => "```trident\nfn ram_read_block(addr: Field) -> Digest\n```\nRead 5 consecutive field elements from RAM.",
        "ram_write_block" => "```trident\nfn ram_write_block(addr: Field, d: Digest)\n```\nWrite 5 consecutive field elements to RAM.",
        "merkle_step" => "```trident\nfn merkle_step(idx: U32, d0..d4: Field) -> (U32, Digest)\n```\nOne step of Merkle tree authentication.",
        "xfield" => "```trident\nfn xfield(a: Field, b: Field, c: Field) -> XField\n```\nConstruct extension field element.",
        "xinvert" => "```trident\nfn xinvert(a: XField) -> XField\n```\nExtension field multiplicative inverse.",
        _ => return None,
    };
    Some(info.to_string())
}

/// Return the parameter list and return type for a builtin function.
pub fn builtin_signature(name: &str) -> Option<(Vec<(&'static str, &'static str)>, &'static str)> {
    let sig: (Vec<(&str, &str)>, &str) = match name {
        "pub_read" => (vec![], "Field"),
        "pub_read2" => (vec![], "(Field, Field)"),
        "pub_read3" => (vec![], "(Field, Field, Field)"),
        "pub_read4" => (vec![], "(Field, Field, Field, Field)"),
        "pub_read5" => (vec![], "Digest"),
        "pub_write" => (vec![("v", "Field")], ""),
        "pub_write2" => (vec![("a", "Field"), ("b", "Field")], ""),
        "pub_write3" => (vec![("a", "Field"), ("b", "Field"), ("c", "Field")], ""),
        "pub_write4" => (
            vec![
                ("a", "Field"),
                ("b", "Field"),
                ("c", "Field"),
                ("d", "Field"),
            ],
            "",
        ),
        "pub_write5" => (
            vec![
                ("a", "Field"),
                ("b", "Field"),
                ("c", "Field"),
                ("d", "Field"),
                ("e", "Field"),
            ],
            "",
        ),
        "divine" => (vec![], "Field"),
        "divine3" => (vec![], "(Field, Field, Field)"),
        "divine5" => (vec![], "Digest"),
        "assert" => (vec![("cond", "Bool")], ""),
        "assert_eq" => (vec![("a", "Field"), ("b", "Field")], ""),
        "assert_digest_eq" => (vec![("a", "Digest"), ("b", "Digest")], ""),
        "hash" => (
            vec![
                ("x0", "Field"),
                ("x1", "Field"),
                ("x2", "Field"),
                ("x3", "Field"),
                ("x4", "Field"),
                ("x5", "Field"),
                ("x6", "Field"),
                ("x7", "Field"),
                ("x8", "Field"),
                ("x9", "Field"),
            ],
            "Digest",
        ),
        "sponge_init" => (vec![], ""),
        "sponge_absorb" => (
            vec![
                ("x0", "Field"),
                ("x1", "Field"),
                ("x2", "Field"),
                ("x3", "Field"),
                ("x4", "Field"),
                ("x5", "Field"),
                ("x6", "Field"),
                ("x7", "Field"),
                ("x8", "Field"),
                ("x9", "Field"),
            ],
            "",
        ),
        "sponge_squeeze" => (vec![], "[Field; 10]"),
        "split" => (vec![("a", "Field")], "(U32, U32)"),
        "log2" => (vec![("a", "U32")], "U32"),
        "pow" => (vec![("base", "U32"), ("exp", "U32")], "U32"),
        "popcount" => (vec![("a", "U32")], "U32"),
        "as_u32" => (vec![("a", "Field")], "U32"),
        "as_field" => (vec![("a", "U32")], "Field"),
        "field_add" => (vec![("a", "Field"), ("b", "Field")], "Field"),
        "field_mul" => (vec![("a", "Field"), ("b", "Field")], "Field"),
        "inv" => (vec![("a", "Field")], "Field"),
        "neg" => (vec![("a", "Field")], "Field"),
        "sub" => (vec![("a", "Field"), ("b", "Field")], "Field"),
        "ram_read" => (vec![("addr", "Field")], "Field"),
        "ram_write" => (vec![("addr", "Field"), ("val", "Field")], ""),
        "ram_read_block" => (vec![("addr", "Field")], "Digest"),
        "ram_write_block" => (vec![("addr", "Field"), ("d", "Digest")], ""),
        "merkle_step" => (
            vec![
                ("idx", "U32"),
                ("d0", "Field"),
                ("d1", "Field"),
                ("d2", "Field"),
                ("d3", "Field"),
                ("d4", "Field"),
            ],
            "(U32, Digest)",
        ),
        "xfield" => (
            vec![("a", "Field"), ("b", "Field"), ("c", "Field")],
            "XField",
        ),
        "xinvert" => (vec![("a", "XField")], "XField"),
        _ => return None,
    };
    Some(sig)
}

/// Completion items for all builtin functions.
pub fn builtin_completions() -> Vec<(String, String)> {
    vec![
        ("pub_read".into(), "() -> Field".into()),
        ("pub_read2".into(), "() -> (Field, Field)".into()),
        ("pub_read3".into(), "() -> (Field, Field, Field)".into()),
        (
            "pub_read4".into(),
            "() -> (Field, Field, Field, Field)".into(),
        ),
        ("pub_read5".into(), "() -> Digest".into()),
        ("pub_write".into(), "(v: Field)".into()),
        ("pub_write2".into(), "(a: Field, b: Field)".into()),
        ("pub_write3".into(), "(a: Field, b: Field, c: Field)".into()),
        (
            "pub_write4".into(),
            "(a: Field, b: Field, c: Field, d: Field)".into(),
        ),
        ("pub_write5".into(), "(a..e: Field)".into()),
        ("divine".into(), "() -> Field".into()),
        ("divine3".into(), "() -> (Field, Field, Field)".into()),
        ("divine5".into(), "() -> Digest".into()),
        ("assert".into(), "(cond: Bool)".into()),
        ("assert_eq".into(), "(a: Field, b: Field)".into()),
        ("assert_digest_eq".into(), "(a: Digest, b: Digest)".into()),
        ("hash".into(), "(x0..x9: Field) -> Digest".into()),
        ("sponge_init".into(), "()".into()),
        ("sponge_absorb".into(), "(x0..x9: Field)".into()),
        ("sponge_squeeze".into(), "() -> [Field; 10]".into()),
        ("split".into(), "(a: Field) -> (U32, U32)".into()),
        ("log2".into(), "(a: U32) -> U32".into()),
        ("pow".into(), "(base: U32, exp: U32) -> U32".into()),
        ("popcount".into(), "(a: U32) -> U32".into()),
        ("as_u32".into(), "(a: Field) -> U32".into()),
        ("as_field".into(), "(a: U32) -> Field".into()),
        ("field_add".into(), "(a: Field, b: Field) -> Field".into()),
        ("field_mul".into(), "(a: Field, b: Field) -> Field".into()),
        ("inv".into(), "(a: Field) -> Field".into()),
        ("neg".into(), "(a: Field) -> Field".into()),
        ("sub".into(), "(a: Field, b: Field) -> Field".into()),
        ("ram_read".into(), "(addr: Field) -> Field".into()),
        ("ram_write".into(), "(addr: Field, val: Field)".into()),
        ("ram_read_block".into(), "(addr: Field) -> Digest".into()),
        ("ram_write_block".into(), "(addr: Field, d: Digest)".into()),
        (
            "merkle_step".into(),
            "(idx: U32, d0..d4: Field) -> (U32, Digest)".into(),
        ),
        (
            "xfield".into(),
            "(a: Field, b: Field, c: Field) -> XField".into(),
        ),
        ("xinvert".into(), "(a: XField) -> XField".into()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_hover_known() {
        assert!(builtin_hover("pub_read").is_some());
        assert!(builtin_hover("hash").is_some());
        assert!(builtin_hover("split").is_some());
        assert!(builtin_hover("merkle_step").is_some());
    }

    #[test]
    fn test_builtin_hover_unknown() {
        assert!(builtin_hover("nonexistent").is_none());
        assert!(builtin_hover("my_function").is_none());
    }

    #[test]
    fn test_builtin_hover_contains_signature() {
        let info = builtin_hover("pub_read").unwrap();
        assert!(info.contains("fn pub_read()"));
        assert!(info.contains("-> Field"));
    }

    #[test]
    fn test_builtin_completions_count() {
        let completions = builtin_completions();
        assert!(
            completions.len() >= 30,
            "expected many builtins, got {}",
            completions.len()
        );
        let names: Vec<&str> = completions.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"pub_read"));
        assert!(names.contains(&"hash"));
        assert!(names.contains(&"split"));
        assert!(names.contains(&"ram_read"));
        assert!(names.contains(&"xinvert"));
    }

    #[test]
    fn test_builtin_signature_known() {
        let (params, ret) = builtin_signature("pub_write").unwrap();
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], ("v", "Field"));
        assert_eq!(ret, "");
    }

    #[test]
    fn test_builtin_signature_with_return() {
        let (params, ret) = builtin_signature("split").unwrap();
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], ("a", "Field"));
        assert_eq!(ret, "(U32, U32)");
    }

    #[test]
    fn test_builtin_signature_no_params() {
        let (params, ret) = builtin_signature("pub_read").unwrap();
        assert_eq!(params.len(), 0);
        assert_eq!(ret, "Field");
    }

    #[test]
    fn test_builtin_signature_unknown() {
        assert!(builtin_signature("nonexistent").is_none());
    }

    #[test]
    fn test_builtin_signature_multi_params() {
        let (params, ret) = builtin_signature("pow").unwrap();
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], ("base", "U32"));
        assert_eq!(params[1], ("exp", "U32"));
        assert_eq!(ret, "U32");
    }

    #[test]
    fn test_builtin_hover_includes_cost() {
        use super::super::util::format_cost_inline;
        let mut info = builtin_hover("hash").unwrap();
        let cost = crate::cost::cost_builtin("triton", "hash");
        info = format!("{}\n\n**Cost:** {}", info, format_cost_inline(&cost));
        assert!(
            info.contains("hash=6"),
            "hash hover should include hash=6 cost, got: {}",
            info
        );
        assert!(
            info.contains("**Cost:**"),
            "hover should include Cost header, got: {}",
            info
        );
    }

    #[test]
    fn test_builtin_hover_pub_read_cost() {
        use super::super::util::format_cost_inline;
        let mut info = builtin_hover("pub_read").unwrap();
        let cost = crate::cost::cost_builtin("triton", "pub_read");
        info = format!("{}\n\n**Cost:** {}", info, format_cost_inline(&cost));
        assert!(
            info.contains("cc=1"),
            "pub_read hover should show cc=1, got: {}",
            info
        );
    }
}
