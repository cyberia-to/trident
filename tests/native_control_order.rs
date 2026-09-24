//! Distinguishable failures establish evaluation order without host services.
#[path = "native_control/support.rs"]
mod support;
use support::*;
use trident::RAW_ARTIFACT_LIMITS as LIMITS;

fn failure(source: String) -> String {
    worker(move || {
        run(
            &compile(&source).bytes,
            0,
            1_000_000,
            65536,
            LIMITS.max_nodes,
        )
        .unwrap_err()
    })
}

#[test]
fn user_call_arguments_stop_at_the_first_failure_in_source_order() {
    let axis = "nox_noun_head(nox_noun_atom(0))";
    let inverse = "nox_noun_atom(inv(0))";
    for (first, second, expected) in [(axis, inverse, "AxisError"), (inverse, axis, "InvZero")] {
        let source = format!("program p\nfn f(a: Noun,b: Noun) -> Noun {{ a }}\nfn main(input: Noun) -> Noun {{ f({first},{second}) }}");
        let error = failure(source);
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn nested_assignment_checks_indices_before_its_right_hand_side() {
    let bad_index = "as_u32(nox_noun_as_field(nox_noun_head(nox_noun_atom(0))))";
    let source = program(&format!(
        "let mut a = [[1,2],[3,4]]\na[0][{bad_index}] = inv(0)\ninput"
    ));
    assert!(failure(source).contains("AxisError"));
    // The first out-of-range index fails before the later expression is run.
    let source = program(&format!(
        "let mut a = [[1,2],[3,4]]\na[2][{bad_index}] = 7\ninput"
    ));
    assert!(failure(source).contains("InvZero"));
    let source = program("let mut a = [1]\na[0] = 8\nnox_noun_atom(a[0])");
    assert_eq!(evaluate(&source, 0), 8);
}

#[test]
fn a_same_named_inner_index_does_not_capture_its_end_expression() {
    let source = program("let mut n: Field = 0\nfor i in 0..3 { for i in 0..i bounded 0 { n = n * 10 + as_field(i) + 1 } }\nnox_noun_atom(n)");
    assert_eq!(evaluate(&source, 0), 112);
}

#[test]
fn tuple_assignment_materializes_the_rhs_before_any_target_is_replaced() {
    let source = program(
        "let mut a: Field = 3\nlet mut b: Field = 7\n(a,b) = (b,a)\nnox_noun_atom(a * 10 + b)",
    );
    assert_eq!(evaluate(&source, 0), 73);
    let source = program("let mut a: Field = 0\n(a,a) = (2,3)\nnox_noun_atom(a)");
    assert_eq!(evaluate(&source, 0), 3);
}

#[derive(Default)]
struct Inversions(Vec<u64>);
impl nox::trace::Tracer for Inversions {
    fn record(&mut self, row: nox::trace::TraceRow) {
        // inv emits 64 rows. Its initial row records operand and step zero.
        if row.col(0) == 8 && row.col(12) == 0 {
            self.0.push(row.col(4));
        }
    }
}

#[test]
fn calls_conversions_indices_and_rhs_evaluate_each_argument_once() {
    worker(|| {
        let sources = [
            ("program p\nfn f(a: Field,b: Field) -> Field { a+b }\nfn main(input: Noun) -> Noun { nox_noun_atom(f(inv(2),inv(3))) }".to_string(), vec![2,3]),
            (program("nox_noun_atom(as_field(as_u32(inv(1))))"),vec![1]),
            (program("let a = [7,8]\nnox_noun_atom(a[as_u32(inv(1))])"),vec![1]),
            (program("let mut a = [7,8]\na[as_u32(inv(1))] = inv(2)\nnox_noun_atom(a[1])"),vec![1,2]),
        ];
        for (source, expected) in sources {
            let mut trace = Inversions::default();
            run_traced(
                &compile(&source).bytes,
                0,
                1_000_000,
                65536,
                LIMITS.max_nodes,
                &mut trace,
            )
            .unwrap();
            assert_eq!(trace.0, expected);
        }
    });
}

#[test]
fn every_dynamic_candidate_evaluates_the_guard_even_after_it_was_false() {
    worker(|| {
        let source = program("let mut count: Field = 0\nfor i in 0..as_u32(inv(1)) bounded 4 { count = count+1 }\nnox_noun_atom(count)");
        let mut trace = Inversions::default();
        let result = run_traced(
            &compile(&source).bytes,
            0,
            1_000_000,
            65536,
            LIMITS.max_nodes,
            &mut trace,
        )
        .unwrap();
        assert_eq!(result.0, 1);
        assert_eq!(trace.0, vec![1; 4]);
    });
}
