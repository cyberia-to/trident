// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use super::*;

#[test]
fn assembly_effect_counts_words_and_preserves_named_prefix() {
    let mut sm = StackManager::with_config(u32::MAX, 1000);
    sm.push_named("sentinel", 5);
    sm.push_temp(3);
    sm.push_temp(2);
    assert!(sm.can_pop_anonymous(5));
    assert!(!sm.can_pop_anonymous(6));
    sm.pop_anonymous(4);
    assert_eq!(sm.stack_depth(), 6);
    assert_eq!(sm.last().unwrap().width, 1);
    assert_eq!(sm.access_var("sentinel"), 1);
    sm.pop_anonymous(1);
    assert_eq!(sm.last().unwrap().name.as_deref(), Some("sentinel"));
    assert_eq!(sm.last().unwrap().width, 5);
    assert!(!sm.can_pop_anonymous(1));
    assert!(sm.drain_side_effects().is_empty());
}

#[test]
fn test_basic_push_pop() {
    let mut sm = StackManager::with_config(16, 1000);
    sm.push_named("a", 1);
    sm.push_named("b", 1);
    assert_eq!(sm.stack_depth(), 2);
    assert_eq!(sm.stack_len(), 2);
    assert_eq!(sm.access_var("b"), 0);
    assert_eq!(sm.access_var("a"), 1);
    sm.pop();
    assert_eq!(sm.stack_depth(), 1);
}

#[test]
fn test_no_spill_under_16() {
    let mut sm = StackManager::with_config(16, 1000);
    for i in 0..16 {
        sm.push_named(&format!("v{}", i), 1);
    }
    assert_eq!(sm.stack_depth(), 16);
    assert!(sm.drain_side_effects().is_empty());
}

#[test]
fn test_spill_at_17() {
    let mut sm = StackManager::with_config(16, 1000);
    // Push 16 variables
    for i in 0..16 {
        sm.push_named(&format!("v{}", i), 1);
    }
    // Access v15 to make it recently used
    sm.access_var("v15");

    // Push one more — should spill the LRU (v0)
    sm.push_named("v16", 1);
    let effects = sm.drain_side_effects();
    // Should have spill instructions
    assert!(!effects.is_empty(), "expected spill instructions");
    // v0 should be spilled
    assert!(sm.spilled.iter().any(|v| v.name.as_deref() == Some("v0")));
}

#[test]
fn test_reload_spilled_var() {
    let mut sm = StackManager::with_config(16, 1000);
    for i in 0..16 {
        sm.push_named(&format!("v{}", i), 1);
    }
    // Push one more to spill v0
    sm.push_named("v16", 1);
    sm.drain_side_effects(); // clear

    // Access v0 — should reload it
    let depth = sm.access_var("v0");
    let effects = sm.drain_side_effects();
    assert!(!effects.is_empty(), "expected reload instructions");
    assert_eq!(depth, 0); // reloaded to top
}

#[test]
fn test_temp_push() {
    let mut sm = StackManager::with_config(16, 1000);
    sm.push_temp(1);
    assert_eq!(sm.stack_depth(), 1);
    assert!(sm.last().unwrap().name.is_none());
}

#[test]
fn test_multi_width_spill() {
    let mut sm = StackManager::with_config(16, 1000);
    // Push a Digest (width 5) and fill up stack
    sm.push_named("digest", 5);
    for i in 0..11 {
        sm.push_named(&format!("v{}", i), 1);
    }
    assert_eq!(sm.stack_depth(), 16);

    // Push one more — should spill digest (LRU, earliest pushed)
    sm.push_named("extra", 1);
    let effects = sm.drain_side_effects();
    assert!(!effects.is_empty());
    // Digest with width 5 should have 5 write_mem instructions
    let write_count = effects
        .iter()
        .filter(|l| matches!(l, TIROp::WriteMem(1)))
        .count();
    assert_eq!(write_count, 5, "expected 5 write_mem for Digest spill");
}

#[test]
fn test_spill_all_named() {
    let mut sm = StackManager::with_config(16, 1000);
    sm.push_named("a", 1);
    sm.push_named("b", 1);
    sm.push_named("c", 1);
    sm.push_temp(1); // anonymous temp
    assert_eq!(sm.stack_depth(), 4);

    sm.spill_all_named();
    let effects = sm.drain_side_effects();
    // 3 named variables spilled → 3 write_mem instructions
    let write_count = effects
        .iter()
        .filter(|l| matches!(l, TIROp::WriteMem(1)))
        .count();
    assert_eq!(write_count, 3, "expected 3 write_mem for 3 named vars");

    // Only the anonymous temp should remain on stack
    assert_eq!(sm.stack_len(), 1, "only anonymous temp should remain");
    assert!(
        sm.last().unwrap().name.is_none(),
        "remaining entry should be anonymous"
    );
}

#[test]
fn test_spill_all_named_empty() {
    let mut sm = StackManager::with_config(16, 1000);
    sm.push_temp(1);
    sm.push_temp(1);
    sm.spill_all_named();
    let effects = sm.drain_side_effects();
    assert!(effects.is_empty(), "no named vars → no spill");
    assert_eq!(sm.stack_len(), 2);
}

#[test]
fn typed_spill_reload_preserves_multiword_order_and_temporaries() {
    use std::collections::BTreeMap;
    for width in 1..24 {
        for above in 0..24 {
            let mut manager = StackManager::with_config(u32::MAX, 1000);
            manager.push_named("saved", width);
            manager.push_temp(above);
            let mut stack: Vec<u64> = (0..width + above).map(u64::from).collect();
            let mut ram = BTreeMap::new();
            manager.spill_all_named();
            manager.access_var("saved");
            for op in manager.drain_side_effects() {
                match op {
                    TIROp::Push(v) => stack.push(v),
                    TIROp::Swap(d) => {
                        let top = stack.len() - 1;
                        stack.swap(top, top - d as usize);
                    }
                    TIROp::Pop(n) => stack.truncate(stack.len() - n as usize),
                    TIROp::WriteMem(1) => {
                        let addr = stack.pop().unwrap();
                        ram.insert(addr, stack.pop().unwrap());
                        stack.push(addr + 1);
                    }
                    TIROp::ReadMem(1) => {
                        let addr = stack.pop().unwrap();
                        stack.push(ram[&addr]);
                        stack.push(addr - 1);
                    }
                    _ => panic!("unexpected spill op: {op:?}"),
                }
            }
            let expected: Vec<_> = (width..width + above)
                .chain(0..width)
                .map(u64::from)
                .collect();
            assert_eq!(stack, expected, "width={width} above={above}");
        }
    }
}

#[test]
fn zero_width_bindings_survive_word_effects_and_spill_pressure() {
    let mut sm = StackManager::with_config(2, 1000);
    sm.push_named("empty", 0);
    sm.push_named("prefix", 1);
    sm.push_temp(1);
    sm.push_named("between", 0);
    sm.push_temp(0);
    assert!(sm.can_pop_anonymous(1));
    assert!(!sm.can_pop_anonymous(2));
    sm.pop_anonymous(1);
    assert_eq!(sm.find_var_depth_and_width("empty"), Some((1, 0)));
    assert_eq!(sm.find_var_depth_and_width("between"), Some((0, 0)));
    assert_eq!(sm.find_var_depth_and_width("prefix"), Some((0, 1)));
    sm.push_temp(2);
    assert_eq!(sm.spilled.len(), 1);
    assert_eq!(sm.spilled[0].name.as_deref(), Some("prefix"));
    assert_eq!(sm.find_var_depth_and_width("empty"), Some((2, 0)));
    assert_eq!(sm.find_var_depth_and_width("between"), Some((2, 0)));
}
