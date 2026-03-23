// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use super::*;
use crate::sym;

fn parse_and_encode(source: &str, mode: QueryMode) -> String {
    let file = crate::parse_source_silent(source, "test.tri").unwrap();
    let system = sym::analyze(&file);
    encode_system(&system, mode)
}

#[test]
fn test_safety_check_encoding() {
    let smt = parse_and_encode(
        "program test\nfn main() {\n    assert(true)\n}\n",
        QueryMode::SafetyCheck,
    );
    assert!(smt.contains("(set-logic QF_BV)"));
    assert!(smt.contains("GOLDILOCKS_P"));
    assert!(smt.contains("(check-sat)"));
}

#[test]
fn test_witness_existence_encoding() {
    let smt = parse_and_encode(
        "program test\nfn main() {\n    let x: Field = divine()\n    assert_eq(x, 42)\n}\n",
        QueryMode::WitnessExistence,
    );
    assert!(smt.contains("Witness existence"));
    assert!(smt.contains("(check-sat)"));
}

#[test]
fn test_variable_declarations() {
    let smt = parse_and_encode(
        "program test\nfn main() {\n    let x: Field = pub_read()\n    pub_write(x)\n}\n",
        QueryMode::SafetyCheck,
    );
    assert!(smt.contains("declare-fun"));
    assert!(smt.contains("BitVec 128"));
}

#[test]
fn test_field_arithmetic_encoding() {
    let smt = parse_and_encode(
        "program test\nfn main() {\n    let x: Field = pub_read()\n    let y: Field = pub_read()\n    assert_eq(x + y, y + x)\n}\n",
        QueryMode::SafetyCheck,
    );
    assert!(smt.contains("bvadd"));
    assert!(smt.contains("field_mod"));
}

#[test]
fn test_sanitize_name() {
    assert_eq!(sanitize_smt_name("x"), "x");
    assert_eq!(sanitize_smt_name("x_0"), "x_0");
    assert_eq!(sanitize_smt_name("std.hash"), "std_hash");
    assert_eq!(sanitize_smt_name("0start"), "v_0start");
}

#[test]
fn test_range_u32_encoding() {
    let smt = parse_and_encode(
        "program test\nfn main() {\n    let x: Field = pub_read()\n    let y: U32 = as_u32(x)\n}\n",
        QueryMode::SafetyCheck,
    );
    assert!(smt.contains("bvule"));
}

#[test]
fn test_empty_constraints() {
    let smt = parse_and_encode(
        "program test\nfn main() {\n    let x: Field = pub_read()\n}\n",
        QueryMode::SafetyCheck,
    );
    assert!(smt.contains("No constraints"));
}
