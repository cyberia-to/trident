use super::support::{self, Result};

#[test]
fn malformed_and_unsupported_sources_fail_with_bound_diagnostics() {
    support::worker(|| {
        for text in [
            "",
            "program",
            "program sample fn main() -> Field { 1+ }",
            "program sample fn main() -> Field { (1 }",
            "program sample fn main() -> Field { 1) }",
            "program sample fn main() -> Field { 1 } 2",
            "program sample fn main() -> Field { 1",
        ] {
            support::error(text.as_bytes(), 2);
        }
        for text in [
            "module sample",
            "program sample use dependency fn main() -> Field { 1 }",
            "program sample fn main() -> Field { 1; }",
            "#[pure] program sample fn main() -> Field { 1 }",
        ] {
            support::error(text.as_bytes(), 6);
        }
        for text in [
            "program different fn main() -> Field { 1 }",
            "program sample fn other() -> Field { 1 }",
        ] {
            support::error(text.as_bytes(), 3);
        }
        let source = support::source("unbound");
        let error = support::error(&source, 5);
        assert_eq!(
            &source[error.start as usize..error.end as usize],
            b"unbound"
        );
        for expression in ["1!2", "1|2", "1/2", "1%2", "1-2"] {
            support::error(&support::source(expression), 1);
        }
        for expression in ["1^2", "1/%2"] {
            support::error(&support::source(expression), 6);
        }
        for expression in ["1&2", "1<2"] {
            support::error(&support::source(expression), 5);
        }
        for word in [
            "program", "module", "use", "fn", "pub", "sec", "let", "mut", "const", "struct", "if",
            "else", "for", "in", "bounded", "return", "true", "false", "event", "reveal", "seal",
            "match", "Field", "XField", "Bool", "U32", "Noun", "Digest", "_", "asm",
        ] {
            let source = format!("program {word} fn main() -> Field {{ 1 }}");
            assert!(
                matches!(support::compile(source.as_bytes()), Result::Errors(_)),
                "{word}"
            );
        }
        for name in ["asm_x", "_x", "programmer", "field", "sample_123"] {
            let source = format!("program {name} fn main() -> Field {{ 7 }}");
            let mut module = support::module(source.as_bytes());
            module.path = name.into();
            assert_eq!(
                support::value(support::compile_package(
                    &[module],
                    name,
                    "main",
                    support::options(),
                    support::CAPS
                )),
                7
            );
        }
    });
}

#[test]
fn exact_whitespace_comments_and_utf8_preserve_source_spans() {
    support::worker(|| {
        for source in [
            b"program\tsample\r\nfn\x0cmain() -> Field { 2+3*4 }".as_slice(),
            "// ж😀\r still comment\nprogram sample fn main() -> Field { 2+3*4 }// конец"
                .as_bytes(),
            b"program sample fn main() -> Field { 2// +100\r still comment\n+3*4 }",
        ] {
            assert_eq!(support::value(support::compile(source)), 14);
        }
        for byte in [0, 11, 127] {
            let mut source = b"program ".to_vec();
            source.push(byte);
            source.extend_from_slice(b"sample fn main() -> Field { 1 }");
            let error = support::error(&source, 1);
            assert_eq!((error.start, error.end), (8, 9));
        }
        for invalid in [
            &b"\x80"[..],
            b"\xc0\xaf",
            b"\xc1\xbf",
            b"\xe0\x9f\x80",
            b"\xed\xa0\x80",
            b"\xf0\x8f\xbf\xbf",
            b"\xf4\x90\x80\x80",
            b"\xf5\x80\x80\x80",
            b"\xff",
            b"\xe2",
            b"\xf0\x90",
        ] {
            let mut source = b"//".to_vec();
            source.extend_from_slice(invalid);
            let error = support::error(&source, 1);
            assert_eq!(error.start, 2);
            assert!(error.end > 2);
        }
        for bytes in [
            &b"\xc2\x80"[..],
            b"\xdf\xbf",
            b"\xe0\xa0\x80",
            b"\xed\x9f\xbf",
            b"\xef\xbf\xbf",
            b"\xf0\x90\x80\x80",
            b"\xf4\x8f\xbf\xbf",
        ] {
            let mut source = b"//".to_vec();
            source.extend_from_slice(bytes);
            source.extend_from_slice(b"\nprogram sample fn main() -> Field { 1 }");
            assert_eq!(support::value(support::compile(&source)), 1);
        }
        // UTF-8 validation runs over the whole selected source before syntax.
        let error = support::error(b"nonsense //\xff", 1);
        assert_eq!((error.start, error.end), (11, 12));
        support::error("program sample fn main() -> Field { ж }".as_bytes(), 1);
        // A continuation sequence crosses the 64-byte helper boundary.
        let mut source = b"//".to_vec();
        source.extend_from_slice(&[b'a'; 61]);
        source.extend_from_slice("😀\nprogram sample fn main() -> Field { 3 }".as_bytes());
        assert_eq!(support::value(support::compile(&source)), 3);
    });
}

#[test]
fn entry_selection_ignores_unused_source_and_rejects_unsupported_requests() {
    support::worker(|| {
        let good = support::module(&support::source("7"));
        let mut unused = support::module(b"\xffgarbage\0");
        unused.path = "aaa".into();
        assert_eq!(
            support::value(support::compile_package(
                &[unused.clone(), good.clone()],
                "sample",
                "main",
                support::options(),
                support::CAPS
            )),
            7
        );
        match support::compile_package(
            &[unused, good.clone()],
            "aaa",
            "main",
            support::options(),
            support::CAPS,
        ) {
            Result::Errors(errors) => {
                assert_eq!(errors[0].module, 0);
                assert_eq!(errors[0].code, 1);
            }
            other => panic!("{other:?}"),
        }
        for (function, profiles) in [("other", 0), ("main", 1)] {
            let mut options = support::options();
            options.input = profiles;
            options.output = profiles;
            match support::compile_package(
                std::slice::from_ref(&good),
                "sample",
                function,
                options,
                support::CAPS,
            ) {
                Result::Errors(errors) => assert_eq!(errors[0].code, 6),
                other => panic!("{other:?}"),
            }
        }
        let mut options = support::options();
        options.cfg = vec!["debug".into(), "release".into()];
        assert_eq!(
            support::value(support::compile_package(
                &[good],
                "sample",
                "main",
                options,
                support::CAPS
            )),
            7
        );
    });
}
