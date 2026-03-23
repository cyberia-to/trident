// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use super::*;
use crate::ast::ArraySize;
use crate::parse_source_silent;

#[test]
fn test_scaffold_with_requires_ensures() {
    let source = r#"program test

#[requires(amount > 0)]
#[ensures(balance == old_balance + amount)]
fn deposit(old_balance: Field, amount: Field) -> Field {
}
"#;
    let file = parse_source_silent(source, "test.tri").unwrap();
    let scaffold = generate_scaffold(&file);

    // Should contain the spec annotations
    assert!(scaffold.contains("#[requires(amount > 0)]"));
    assert!(scaffold.contains("#[ensures(balance == old_balance + amount)]"));

    // Should contain TODO comment
    assert!(scaffold.contains("// TODO: Implement deposit logic"));

    // Should contain spec comment block
    assert!(scaffold.contains("//   requires: amount > 0"));
    assert!(scaffold.contains("//   ensures: balance == old_balance + amount"));

    // Should contain a result binding
    assert!(scaffold.contains("let result: Field ="));

    // Should contain postcondition assertion
    assert!(scaffold.contains("assert(result == old_balance + amount)"));

    // Should return result
    assert!(scaffold.contains("    result\n"));
}

#[test]
fn test_scaffold_without_annotations() {
    let source = r#"program test

fn add(x: Field, y: Field) -> Field {
}
"#;
    let file = parse_source_silent(source, "test.tri").unwrap();
    let scaffold = generate_scaffold(&file);

    // Should have a TODO
    assert!(scaffold.contains("// TODO: Implement add logic"));

    // Should have a default return value
    assert!(scaffold.contains("let result: Field = 0"));

    // Should return result
    assert!(scaffold.contains("    result\n"));

    // Should NOT contain spec comment
    assert!(!scaffold.contains("// Specification:"));
}

#[test]
fn test_scaffold_void_function() {
    let source = r#"program test

#[requires(x > 0)]
#[ensures(true)]
fn validate(x: Field) {
}
"#;
    let file = parse_source_silent(source, "test.tri").unwrap();
    let scaffold = generate_scaffold(&file);

    // Should have TODO
    assert!(scaffold.contains("// TODO: Implement validate logic"));

    // Void function should assert requires
    assert!(scaffold.contains("assert(x > 0)"));

    // "true" in ensures should be skipped
    assert!(!scaffold.contains("assert(true)"));

    // Should NOT have result binding or return
    assert!(!scaffold.contains("let result"));
}

#[test]
fn test_default_value_field() {
    assert_eq!(default_value(&Type::Field), "0");
}

#[test]
fn test_default_value_bool() {
    assert_eq!(default_value(&Type::Bool), "false");
}

#[test]
fn test_default_value_u32() {
    assert_eq!(default_value(&Type::U32), "0");
}

#[test]
fn test_default_value_digest() {
    assert_eq!(default_value(&Type::Digest), "0");
}

#[test]
fn test_default_value_array() {
    let ty = Type::Array(Box::new(Type::Field), ArraySize::Literal(3));
    assert_eq!(default_value(&ty), "[0, 0, 0]");
}

#[test]
fn test_default_value_tuple() {
    let ty = Type::Tuple(vec![Type::Field, Type::Bool]);
    assert_eq!(default_value(&ty), "(0, false)");
}

#[test]
fn test_default_value_xfield() {
    assert_eq!(default_value(&Type::XField), "0");
}

#[test]
fn test_extract_variables_simple() {
    let vars = extract_variables("amount > 0");
    assert_eq!(vars, vec!["amount"]);
}

#[test]
fn test_extract_variables_expression() {
    let vars = extract_variables("balance == old_balance + amount");
    assert!(vars.contains(&"balance".to_string()));
    assert!(vars.contains(&"old_balance".to_string()));
    assert!(vars.contains(&"amount".to_string()));
}

#[test]
fn test_extract_variables_skips_keywords() {
    let vars = extract_variables("result == true");
    // "result" and "true" are keywords, so should be excluded
    assert!(vars.is_empty());
}

#[test]
fn test_extract_variables_with_underscore() {
    let vars = extract_variables("_private_var > 0");
    assert_eq!(vars, vec!["_private_var"]);
}

#[test]
fn test_scaffold_tuple_return() {
    let source = r#"program test

#[requires(amount > 0)]
#[ensures(new_sender == sub(sender_balance, amount))]
#[ensures(new_receiver == receiver_balance + amount)]
fn transfer(sender_balance: Field, receiver_balance: Field, amount: Field) -> (Field, Field) {
}
"#;
    let file = parse_source_silent(source, "test.tri").unwrap();
    let scaffold = generate_scaffold(&file);

    // Should contain annotations (parser spaces tokens inside attributes)
    assert!(scaffold.contains("#[requires(amount > 0)]"));
    assert!(scaffold.contains("#[ensures(new_sender == sub ( sender_balance , amount ))]"));
    assert!(scaffold.contains("#[ensures(new_receiver == receiver_balance + amount)]"));

    // Should have result binding with tuple type
    assert!(scaffold.contains("let result: (Field, Field) ="));

    // Should have postcondition assertions
    assert!(scaffold.contains("assert(result == sub ( sender_balance , amount ))"));
    assert!(scaffold.contains("assert(result == receiver_balance + amount)"));
}

#[test]
fn test_scaffold_result_keyword_in_ensures() {
    let source = r#"program test

#[requires(amount > 0)]
#[ensures(result == balance + amount)]
fn deposit(balance: Field, amount: Field) -> Field {
}
"#;
    let file = parse_source_silent(source, "test.tri").unwrap();
    let scaffold = generate_scaffold(&file);

    // When ensures uses "result", the synthesized expression comes from the RHS
    assert!(scaffold.contains("let result: Field = balance + amount"));

    // Assertion should use the clause as-is since it uses "result"
    assert!(scaffold.contains("assert(result == balance + amount)"));
}

#[test]
fn test_scaffold_multiple_requires() {
    let source = r#"program test

#[requires(amount > 0)]
#[requires(balance > amount)]
#[ensures(result == sub(balance, amount))]
fn withdraw(balance: Field, amount: Field) -> Field {
}
"#;
    let file = parse_source_silent(source, "test.tri").unwrap();
    let scaffold = generate_scaffold(&file);

    assert!(scaffold.contains("#[requires(amount > 0)]"));
    assert!(scaffold.contains("#[requires(balance > amount)]"));
    assert!(scaffold.contains("//   requires: amount > 0"));
    assert!(scaffold.contains("//   requires: balance > amount"));
    // Parser spaces tokens inside attributes: sub(balance, amount) -> sub ( balance , amount )
    assert!(scaffold.contains("let result: Field = sub ( balance , amount )"));
}

#[test]
fn test_scaffold_preserves_pub() {
    let source = r#"program test

#[requires(x > 0)]
pub fn check(x: Field) -> Field {
}
"#;
    let file = parse_source_silent(source, "test.tri").unwrap();
    let scaffold = generate_scaffold(&file);

    assert!(scaffold.contains("pub fn check"));
}

#[test]
fn test_scaffold_preserves_program_header() {
    let source = r#"program my_app

fn main() {
}
"#;
    let file = parse_source_silent(source, "test.tri").unwrap();
    let scaffold = generate_scaffold(&file);

    assert!(scaffold.starts_with("program my_app\n"));
}
