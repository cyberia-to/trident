use super::run_profile;

#[test]
fn terminal_comparisons_use_canonical_field_literals() {
    let source = "program residues
const ZERO:Field=18446744069414584321
fn main()->Field {if ZERO==0 {18446744073709551615} else {7}}";
    for profile in ["debug", "release"] {
        assert_eq!(run_profile(source, &[], profile), 4294967294);
    }
}

#[test]
fn terminal_conditionals_return_the_selected_value() {
    let source = "program terminal
fn main(x: Field) -> Field {
    if x == 0 { 3 } else { 4 }
}";
    for profile in ["debug", "release"] {
        assert_eq!(run_profile(source, &[0], profile), 3);
        assert_eq!(run_profile(source, &[1], profile), 4);
    }
}

#[test]
fn helper_terminal_branches_preserve_locals_and_aggregate_results() {
    let source = "program helper
struct Pair { a: Field, b: Field }
fn choose(x: Field) -> Pair {
    let a = 7
    if x == 0 {
        let a = 11
        if x == 0 { Pair { b: 19, a: a } } else { Pair { a: 13, b: 23 } }
    } else {
        if x == 1 { return Pair { a: a, b: 29 } }
        Pair { a: 31, b: 37 }
    }
}
fn main(x: Field) -> Field {
    let p = choose(x)
    p.a * 100 + p.b
}";
    for profile in ["debug", "release"] {
        for (input, expected) in [(0, 1119), (1, 729), (2, 3137)] {
            assert_eq!(run_profile(source, &[input], profile), expected);
        }
    }
}

#[test]
fn intermediate_branch_and_loop_tails_do_not_return_from_function() {
    let source = "program intermediate
fn main(x: Field) -> Field {
    let mut total: Field = 7
    if x == 0 { total = 11\n 99 } else { 98 }
    for i in 0..2 { total = total + 1\n 97 }
    total
}";
    for profile in ["debug", "release"] {
        assert_eq!(run_profile(source, &[0], profile), 13);
        assert_eq!(run_profile(source, &[1], profile), 9);
    }
}
