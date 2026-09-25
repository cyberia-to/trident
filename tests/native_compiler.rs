#[path = "native_compiler/support.rs"]
mod support;
use support::{Expr, Result};

#[test]
fn fixed_guest_compiles_fresh_arithmetic_to_exact_executable_formulas() {
    support::worker(|| {
        let c1 = support::compiler().to_vec(); // Build before constructing this corpus.
        use Expr::{Add as A, Mul as M, Number as N};
        let cases = vec![
            (
                "2+3*4".into(),
                A(Box::new(N(2)), Box::new(M(Box::new(N(3)), Box::new(N(4))))),
            ),
            (
                "(2+3)*4".into(),
                M(Box::new(A(Box::new(N(2)), Box::new(N(3)))), Box::new(N(4))),
            ),
            (
                "1+2+3".into(),
                A(Box::new(A(Box::new(N(1)), Box::new(N(2)))), Box::new(N(3))),
            ),
            (
                "2*3*4".into(),
                M(Box::new(M(Box::new(N(2)), Box::new(N(3)))), Box::new(N(4))),
            ),
        ];
        let mut expressions = cases;
        let mut seed = 0x31415926u64;
        for _ in 0..12 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let a = seed;
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let e = A(
                Box::new(N(a)),
                Box::new(M(Box::new(N(seed)), Box::new(N(seed >> 32)))),
            );
            expressions.push((e.text(), e));
        }
        for (text, expression) in expressions {
            let source = support::source(&text);
            match support::compile(&source) {
                Result::Program { bytes, value, .. } => {
                    assert_eq!(bytes, expression.artifact(), "{text}");
                    assert_eq!(value, expression.value(), "{text}");
                    assert_eq!(
                        value,
                        support::rust_value(std::str::from_utf8(&source).unwrap()),
                        "{text}"
                    );
                }
                other => panic!("{text}: {other:?}"),
            }
        }
        assert_eq!(c1, support::compiler());
    });
}

#[test]
fn decimal_range_and_leading_zeroes_match_native_seed_goldilocks() {
    support::worker(|| {
        for literal in [
            "0",
            "00000000000000000000000000000001",
            "4294967295",
            "4294967296",
            "18446744069414584320",
            "18446744069414584321",
            "18446744069414584322",
            "18446744073709551615",
        ] {
            let source = support::source(literal);
            let expected = (literal.parse::<u128>().unwrap() % 18446744069414584321) as u64;
            assert_eq!(
                support::value(support::compile(&source)),
                expected,
                "{literal}"
            );
            assert_eq!(
                support::rust_value(std::str::from_utf8(&source).unwrap()),
                expected
            );
        }
        for literal in [
            "18446744073709551616",
            "99999999999999999999",
            "100000000000000000000",
            "00018446744073709551616",
        ] {
            let source = support::source(literal);
            let error = support::error(&source, 1);
            assert_eq!(error.end - error.start, literal.len() as u32);
        }
    });
}

#[path = "native_compiler/diagnostics.rs"]
mod diagnostics;

#[path = "native_compiler/bounds.rs"]
mod bounds;

#[path = "native_compiler/locals.rs"]
mod locals;

#[path = "native_compiler/codegen.rs"]
mod codegen;

#[path = "native_compiler/control.rs"]
mod control;

#[path = "native_compiler/signatures.rs"]
mod signatures;

#[path = "native_compiler/call_expressions.rs"]
mod call_expressions;

#[path = "native_compiler/function_check.rs"]
mod function_check;

#[path = "native_compiler/function_plan.rs"]
mod function_plan;

#[path = "native_compiler/functions.rs"]
mod functions;

#[path = "native_compiler/function_bounds.rs"]
mod function_bounds;

#[path = "native_compiler/scalars.rs"]
mod scalars;

#[path = "native_compiler/loops.rs"]
mod loops;

#[path = "native_compiler/nouns.rs"]
mod nouns;

#[path = "native_compiler/types.rs"]
mod types;

#[path = "native_compiler/digests.rs"]
mod digests;

#[path = "native_compiler/tuple_errors.rs"]
mod tuple_errors;
#[path = "native_compiler/tuples.rs"]
mod tuples;

#[path = "native_compiler/type_syntax.rs"]
mod type_syntax;

#[path = "native_compiler/nominal.rs"]
mod nominal;

#[path = "native_compiler/record_errors.rs"]
mod record_errors;
#[path = "native_compiler/records.rs"]
mod records;

#[path = "native_compiler/record_bounds.rs"]
mod record_bounds;

#[path = "native_compiler/record_write_bounds.rs"]
mod record_write_bounds;
#[path = "native_compiler/record_writes.rs"]
mod record_writes;

#[path = "native_compiler/record_edit_component.rs"]
mod record_edit_component;

#[path = "native_compiler/array_errors.rs"]
mod array_errors;
#[path = "native_compiler/arrays.rs"]
mod arrays;

#[path = "native_compiler/array_order.rs"]
mod array_order;

#[path = "native_compiler/array_bounds.rs"]
mod array_bounds;
