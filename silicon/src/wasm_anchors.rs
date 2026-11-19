//! Regression anchors for the wasm backend (criba findings).

use super::*;

/// WebAssembly numeric opcodes (spec, section "Numeric Instructions"):
/// i64.eq=0x51 i64.ne=0x52 i64.lt_s=0x53 i64.lt_u=0x54 i64.gt_s=0x55
/// i64.gt_u=0x56 i64.le_s=0x57 i64.le_u=0x58 i64.ge_s=0x59 i64.ge_u=0x5A.
#[test]
fn opcode_i64_lt_u_matches_spec() {
    assert_eq!(OP_I64_LT_U, 0x54, "silicon-src-1.CL-LOG.1: i64.lt_u is 0x54 (0x53 is i64.lt_s)");
}

#[test]
fn opcode_i64_ge_u_matches_spec() {
    assert_eq!(OP_I64_GE_U, 0x5A, "silicon-src-1.CL-LOG.2: i64.ge_u is 0x5A (0x57 is i64.le_s)");
}

#[test]
fn opcode_i64_gt_u_matches_spec() {
    assert_eq!(OP_I64_GT_U, 0x56, "silicon-src-1.CL-LOG.3: i64.gt_u is 0x56 (0x55 is i64.gt_s)");
}
