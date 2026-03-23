// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use super::*;

fn lines(s: &[&str]) -> Vec<String> {
    s.iter().map(|l| l.to_string()).collect()
}

#[test]
fn push_add() {
    let mut s = StackState::new(vec![]);
    s.execute(&lines(&["push 1", "push 2", "add"]));
    assert!(s.is_valid());
    assert_eq!(s.stack, vec![3]);
}

#[test]
fn dup_swap() {
    let mut s = StackState::new(vec![10, 20]);
    s.execute(&lines(&["dup 1", "swap 1"]));
    assert!(s.is_valid());
    // [10, 20] → dup 1 → [10, 20, 10] → swap 1 → [10, 10, 20]
    assert_eq!(s.stack, vec![10, 10, 20]);
}

#[test]
fn underflow_is_error() {
    let mut s = StackState::new(vec![]);
    s.execute(&lines(&["add"]));
    assert!(!s.is_valid());
}

#[test]
fn goldilocks_arithmetic() {
    let mut s = StackState::new(vec![]);
    // push p-1, push 1, add → (p-1) + 1 = p ≡ 0 mod p
    s.execute(&lines(&["push 18446744069414584320", "push 1", "add"]));
    assert!(s.is_valid());
    assert_eq!(s.stack, vec![0]); // (MODULUS - 1) + 1 = 0 mod p
}

#[test]
fn mul_field() {
    let mut s = StackState::new(vec![]);
    s.execute(&lines(&["push 3", "push 5", "mul"]));
    assert!(s.is_valid());
    assert_eq!(s.stack, vec![15]);
}

#[test]
fn split_instruction() {
    let mut s = StackState::new(vec![]);
    // 0x0000_0003_0000_0005 = 3 * 2^32 + 5
    let val = 3u64 * (1u64 << 32) + 5;
    s.stack.push(val);
    s.execute(&lines(&["split"]));
    assert!(s.is_valid());
    assert_eq!(s.stack, vec![3, 5]); // hi=3, lo=5
}

#[test]
fn eq_comparison() {
    let mut s = StackState::new(vec![42, 42]);
    s.execute(&lines(&["eq"]));
    assert!(s.is_valid());
    assert_eq!(s.stack, vec![1]);

    let mut s2 = StackState::new(vec![42, 43]);
    s2.execute(&lines(&["eq"]));
    assert!(s2.is_valid());
    assert_eq!(s2.stack, vec![0]);
}

#[test]
fn negative_push() {
    let mut s = StackState::new(vec![]);
    s.execute(&lines(&["push 5", "push -1", "add"]));
    assert!(s.is_valid());
    assert_eq!(s.stack, vec![4]);
}

#[test]
fn control_flow_is_error() {
    let mut s = StackState::new(vec![1]);
    s.execute(&lines(&["skiz"]));
    assert!(!s.is_valid());
}

#[test]
fn comments_and_labels_ignored() {
    let mut s = StackState::new(vec![]);
    s.execute(&lines(&["// comment", "__label:", "push 1", ""]));
    assert!(s.is_valid());
    assert_eq!(s.stack, vec![1]);
}

#[test]
fn verify_equivalent_same() {
    let baseline = lines(&["push 1", "push 2", "add"]);
    let candidate = lines(&["push 3"]); // same result, different path
    assert!(verify_equivalent(&baseline, &candidate, 42));
}

#[test]
fn verify_equivalent_different() {
    let baseline = lines(&["push 1", "push 2", "add"]);
    let candidate = lines(&["push 4"]); // different result
    assert!(!verify_equivalent(&baseline, &candidate, 42));
}

#[test]
fn verify_with_stack_input() {
    // Both should add TOS to second element
    let baseline = lines(&["dup 0", "dup 2", "add"]);
    let candidate = lines(&["dup 0", "dup 2", "add"]);
    assert!(verify_equivalent(&baseline, &candidate, 123));
}

#[test]
fn pow_instruction() {
    let mut s = StackState::new(vec![]);
    s.execute(&lines(&["push 2", "push 10", "pow"]));
    assert!(s.is_valid());
    assert_eq!(s.stack, vec![1024]); // 2^10
}

#[test]
fn pop_count_instruction() {
    let mut s = StackState::new(vec![0b1010_1010]);
    s.execute(&lines(&["pop_count"]));
    assert!(s.is_valid());
    assert_eq!(s.stack, vec![4]);
}

#[test]
fn sbox_pattern() {
    // x^5 via dup/mul chain (from poseidon baseline)
    let x = 7u64;
    let mut s = StackState::new(vec![x]);
    s.execute(&lines(&[
        "dup 0", "dup 0", "mul", // x, x^2
        "dup 0", "mul", // x, x^4
        "mul", // x^5
    ]));
    assert!(s.is_valid());
    // 7^5 = 16807
    assert_eq!(s.stack, vec![16807]);
}

#[test]
fn verify_rejects_when_baseline_errors() {
    // Baseline has control flow (can't simulate) — candidate must be rejected
    let baseline = lines(&["push 1", "call some_fn", "add"]);
    let candidate = lines(&["push 42"]);
    assert!(!verify_equivalent(&baseline, &candidate, 42));
}

#[test]
fn verify_rejects_when_candidate_errors() {
    let baseline = lines(&["push 1", "push 2", "add"]);
    let bad_candidate = lines(&["pop 100"]); // underflow
    assert!(!verify_equivalent(&baseline, &bad_candidate, 42));
}

#[test]
fn verify_rejects_both_error() {
    // Both error — conservative: reject (no free passes)
    let baseline = lines(&["call foo"]); // errors
    let candidate = lines(&["call bar"]); // also errors
    assert!(!verify_equivalent(&baseline, &candidate, 42));
}

#[test]
fn generate_test_stack_deterministic() {
    let a = generate_test_stack(42, 8);
    let b = generate_test_stack(42, 8);
    assert_eq!(a, b);
    // Different seed → different stack
    let c = generate_test_stack(43, 8);
    assert_ne!(a, c);
}

#[test]
fn generate_test_stack_in_range() {
    use crate::field::goldilocks::MODULUS;
    let stack = generate_test_stack(99, 100);
    for val in &stack {
        assert!(*val < MODULUS, "value {} >= MODULUS", val);
    }
}

// --- Side-channel verification tests ---

#[test]
fn write_io_removal_caught() {
    // Baseline writes TOS; candidate just pops — same stack, different I/O
    let baseline = lines(&["write_io 1"]);
    let candidate = lines(&["pop 1"]);
    assert!(!verify_equivalent(&baseline, &candidate, 77));
}

#[test]
fn write_io_equivalent_accepted() {
    // Both write the same value
    let baseline = lines(&["write_io 1"]);
    let candidate = lines(&["write_io 1"]);
    assert!(verify_equivalent(&baseline, &candidate, 77));
}

#[test]
fn assert_removal_caught() {
    // Baseline asserts TOS==1; candidate just pops — different assert_log
    let baseline = lines(&["push 1", "assert"]);
    let candidate = lines(&["push 1", "pop 1"]);
    assert!(!verify_equivalent(&baseline, &candidate, 88));
}

#[test]
fn assert_equivalent_accepted() {
    let baseline = lines(&["push 1", "assert"]);
    let candidate = lines(&["push 1", "assert"]);
    assert!(verify_equivalent(&baseline, &candidate, 88));
}

#[test]
fn divine_replacement_caught() {
    // Baseline uses divine 1; candidate uses push 0 — same stack (both push 0)
    // but divine_log differs
    let baseline = lines(&["divine 1"]);
    let candidate = lines(&["push 0"]);
    assert!(!verify_equivalent(&baseline, &candidate, 99));
}

#[test]
fn divine_equivalent_accepted() {
    let baseline = lines(&["divine 1"]);
    let candidate = lines(&["divine 1"]);
    assert!(verify_equivalent(&baseline, &candidate, 99));
}

#[test]
fn halt_removal_caught() {
    // Baseline halts before push 99; candidate executes push 99
    let baseline = lines(&["halt", "push 99"]);
    let candidate = lines(&["push 99"]);
    assert!(!verify_equivalent(&baseline, &candidate, 55));
}

#[test]
fn halt_equivalent_accepted() {
    let baseline = lines(&["push 1", "halt"]);
    let candidate = lines(&["push 1", "halt"]);
    assert!(verify_equivalent(&baseline, &candidate, 55));
}

#[test]
fn split_now_verifiable() {
    // split should work now — same input, same deterministic output
    let baseline = lines(&["split"]);
    let candidate = lines(&["split"]);
    assert!(verify_equivalent(&baseline, &candidate, 42));
}

#[test]
fn split_wrong_replacement_caught() {
    // Candidate fakes split with wrong stack effect
    let baseline = lines(&["split"]);
    let candidate = lines(&["dup 0"]); // wrong: pushes copy instead of hi/lo
    assert!(!verify_equivalent(&baseline, &candidate, 42));
}

#[test]
fn assert_vector_removal_caught() {
    // Baseline: assert_vector (compare top 5 with next 5, pop 5)
    // Candidate: pop 5 (same stack effect, no assertion)
    let baseline = lines(&[
        "push 1",
        "push 2",
        "push 3",
        "push 4",
        "push 5",
        "push 1",
        "push 2",
        "push 3",
        "push 4",
        "push 5",
        "assert_vector",
    ]);
    let candidate = lines(&[
        "push 1", "push 2", "push 3", "push 4", "push 5", "push 1", "push 2", "push 3",
        "push 4", "push 5", "pop 5",
    ]);
    assert!(!verify_equivalent(&baseline, &candidate, 66));
}

#[test]
fn score_candidate_crash_returns_zero() {
    let baseline = lines(&["push 1", "push 2", "add"]);
    // Pop everything (16 initial + 0 pushed), then pop again → underflow
    let candidate = lines(&["pop 5", "pop 5", "pop 5", "pop 5", "pop 1"]);
    assert_eq!(score_candidate(&baseline, &candidate, 42), 0);
}

#[test]
fn score_candidate_no_crash_wrong_depth() {
    let baseline = lines(&["push 1", "push 2", "add"]); // depth +1
    let candidate = lines(&["push 1", "push 2"]); // depth +2
    let score = score_candidate(&baseline, &candidate, 42);
    assert_eq!(score, 100); // doesn't crash, but wrong depth
}

#[test]
fn score_candidate_right_depth_wrong_values() {
    let baseline = lines(&["push 1"]); // pushes 1
    let candidate = lines(&["push 2"]); // pushes 2, same depth
    let score = score_candidate(&baseline, &candidate, 42);
    assert!(
        score >= 200,
        "right depth should score >= 200, got {}",
        score
    );
}

#[test]
fn score_candidate_identical_scores_900() {
    let baseline = lines(&["push 1", "push 2", "add"]);
    let candidate = lines(&["push 1", "push 2", "add"]);
    let score = score_candidate(&baseline, &candidate, 42);
    assert_eq!(score, 900);
}

#[test]
fn score_candidate_nop_equivalent_scores_900() {
    let baseline = lines(&["push 5"]);
    let candidate = lines(&["push 5", "nop"]);
    let score = score_candidate(&baseline, &candidate, 42);
    assert_eq!(score, 900);
}
