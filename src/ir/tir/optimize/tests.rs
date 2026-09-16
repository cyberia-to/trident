// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use super::*;

#[test]
fn merge_consecutive_hints() {
    let ops = vec![
        TIROp::Hint(1),
        TIROp::Hint(1),
        TIROp::Hint(1),
        TIROp::Add,
        TIROp::Hint(1),
        TIROp::Hint(1),
    ];
    let result = optimize(ops);
    assert_eq!(result.len(), 3);
    assert!(matches!(result[0], TIROp::Hint(3)));
    assert!(matches!(result[1], TIROp::Add));
    assert!(matches!(result[2], TIROp::Hint(2)));
}

#[test]
fn merge_consecutive_pops() {
    let ops = vec![TIROp::Pop(2), TIROp::Pop(3), TIROp::Pop(1)];
    let result = optimize(ops);
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0], TIROp::Pop(6)));
}

#[test]
fn eliminate_swap_zero() {
    let ops = vec![TIROp::Push(1), TIROp::Swap(0), TIROp::Add];
    let result = optimize(ops);
    assert_eq!(result.len(), 2);
    assert!(matches!(result[0], TIROp::Push(1)));
    assert!(matches!(result[1], TIROp::Add));
}

#[test]
fn eliminate_spill_reload_pair() {
    let addr = 1 << 30;
    let ops = vec![
        TIROp::Push(42),
        TIROp::Push(addr),
        TIROp::Swap(1),
        TIROp::WriteMem(1),
        TIROp::Pop(1),
        TIROp::Add,
        TIROp::Push(addr),
        TIROp::ReadMem(1),
        TIROp::Pop(1),
    ];
    let result = optimize(ops);
    assert_eq!(result.len(), 2);
    assert!(matches!(result[0], TIROp::Push(42)));
    assert!(matches!(result[1], TIROp::Add));
}

#[test]
fn eliminate_dead_store() {
    let addr = 1 << 30;
    let ops = vec![
        TIROp::Push(42),
        TIROp::Push(addr),
        TIROp::Swap(1),
        TIROp::WriteMem(1),
        TIROp::Pop(1),
        TIROp::Add,
    ];
    let result = optimize(ops);
    assert!(result.iter().any(|op| matches!(op, TIROp::Pop(_))));
    assert!(!result.iter().any(|op| matches!(op, TIROp::WriteMem(_))));
}

#[test]
fn no_eliminate_when_multiple_reads() {
    let addr = 1 << 30;
    let ops = vec![
        TIROp::Push(addr),
        TIROp::Swap(1),
        TIROp::WriteMem(1),
        TIROp::Pop(1),
        TIROp::Push(addr),
        TIROp::ReadMem(1),
        TIROp::Pop(1),
        TIROp::Push(addr),
        TIROp::ReadMem(1),
        TIROp::Pop(1),
    ];
    let result = optimize(ops);
    assert_eq!(result.len(), 10);
}

#[test]
fn eliminate_dup0_pop1_nop() {
    let ops = vec![TIROp::Push(42), TIROp::Dup(0), TIROp::Pop(1), TIROp::Add];
    let result = optimize(ops);
    assert_eq!(result.len(), 2);
    assert!(matches!(result[0], TIROp::Push(42)));
    assert!(matches!(result[1], TIROp::Add));
}

#[test]
fn eliminate_dup0_swap1_pop1_nop() {
    let ops = vec![
        TIROp::Push(42),
        TIROp::Dup(0),
        TIROp::Swap(1),
        TIROp::Pop(1),
        TIROp::Add,
    ];
    let result = optimize(ops);
    assert_eq!(result.len(), 2);
    assert!(matches!(result[0], TIROp::Push(42)));
    assert!(matches!(result[1], TIROp::Add));
}

#[test]
fn eliminate_double_swap() {
    let ops = vec![TIROp::Push(1), TIROp::Swap(15), TIROp::Swap(15), TIROp::Add];
    let result = optimize(ops);
    assert_eq!(result.len(), 2);
    assert!(matches!(result[0], TIROp::Push(1)));
    assert!(matches!(result[1], TIROp::Add));
}

#[test]
fn eliminate_many_double_swaps() {
    let mut ops = Vec::new();
    for _ in 0..12 {
        ops.push(TIROp::Swap(15));
    }
    ops.push(TIROp::Return);
    let result = optimize(ops);
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0], TIROp::Return));
}

#[test]
fn collapse_epilogue_swap_pop_chain() {
    let ops = vec![
        TIROp::Swap(5),
        TIROp::Pop(1),
        TIROp::Swap(4),
        TIROp::Pop(1),
        TIROp::Swap(3),
        TIROp::Pop(1),
        TIROp::Return,
    ];
    let input = (0..20).collect::<Vec<_>>();
    assert_eq!(
        stack_effect(&optimize(ops.clone()), input.clone()),
        stack_effect(&ops, input)
    );
}

#[test]
fn collapse_constant_depth_swap1_pop1_chain() {
    for count in 1..=128 {
        let ops: Vec<_> = (0..count)
            .flat_map(|_| [TIROp::Swap(1), TIROp::Pop(1)])
            .collect();
        let input: Vec<_> = (0..count + 8).map(u64::from).collect();
        let result = optimize(ops.clone());
        assert_eq!(
            stack_effect(&result, input.clone()),
            stack_effect(&ops, input)
        );
        assert_eq!(result.len(), 2);
        assert!(matches!(result[0], TIROp::Swap(n) if n == count));
        assert!(matches!(result[1], TIROp::Pop(n) if n == count));
    }
}

#[test]
fn cleanup_rewrites_preserve_survivors_across_depth_patterns() {
    for depth in 1..40 {
        for count in 1..=depth {
            for decreasing in [false, true] {
                let ops: Vec<_> = (0..count)
                    .flat_map(|i| {
                        [
                            TIROp::Swap(if decreasing { depth - i } else { depth }),
                            TIROp::Pop(1),
                        ]
                    })
                    .collect();
                let input: Vec<_> = (0..100).collect();
                assert_eq!(
                    stack_effect(&optimize(ops.clone()), input.clone()),
                    stack_effect(&ops, input)
                );
            }
        }
    }
}

#[test]
fn hint_batches_remain_abstract() {
    assert!(matches!(
        optimize(vec![TIROp::Hint(17), TIROp::Hint(19)]).as_slice(),
        [TIROp::Hint(36)]
    ));
}

fn stack_effect(ops: &[TIROp], mut stack: Vec<u64>) -> Vec<u64> {
    for op in ops {
        match op {
            TIROp::Swap(d) => {
                let top = stack.len() - 1;
                stack.swap(top, top - *d as usize);
            }
            TIROp::Dup(d) => stack.push(stack[stack.len() - 1 - *d as usize]),
            TIROp::Pop(n) => {
                stack.truncate(stack.len() - *n as usize);
            }
            TIROp::Return => break,
            _ => panic!("unexpected operation"),
        }
    }
    stack
}

#[test]
fn multiword_return_cleanup_preserves_values_after_optimization() {
    for width in [3, 5, 8] {
        for dead in [4, 5, 16] {
            let ops: Vec<_> = (0..dead)
                .flat_map(|_| [TIROp::Swap(width), TIROp::Pop(1)])
                .collect();
            let input: Vec<u64> = (1..=40).collect();
            assert_eq!(
                stack_effect(&optimize(ops.clone()), input.clone()),
                stack_effect(&ops, input)
            );
        }
    }
}

#[test]
fn block_copy_cleanup_preserves_operand_values() {
    let ops = vec![TIROp::Dup(1), TIROp::Dup(1), TIROp::Swap(2), TIROp::Pop(2)];
    assert_eq!(stack_effect(&optimize(ops), vec![1, 2]), vec![1, 2]);
}

#[test]
fn optimize_nested_bodies() {
    let ops = vec![TIROp::IfElse {
        then_body: vec![TIROp::Hint(1), TIROp::Hint(1)],
        else_body: vec![TIROp::Pop(0)],
    }];
    let result = optimize(ops);
    if let TIROp::IfElse {
        then_body,
        else_body,
    } = &result[0]
    {
        assert_eq!(then_body.len(), 1);
        assert!(matches!(then_body[0], TIROp::Hint(2)));
        assert!(else_body.is_empty());
    } else {
        panic!("expected IfElse");
    }
}

#[test]
fn lowering_errors_survive_peephole_and_nested_optimization() {
    let error = TIROp::Comment("ERROR: unresolved variable 'missing'".into());
    let leaf = vec![TIROp::Dup(0), TIROp::Pop(1), error.clone(), TIROp::Pop(0)];
    let ops = vec![
        error.clone(),
        TIROp::IfElse {
            then_body: leaf.clone(),
            else_body: leaf.clone(),
        },
        TIROp::IfOnly {
            then_body: leaf.clone(),
        },
        TIROp::Loop {
            label: "loop".into(),
            body: leaf.clone(),
        },
        TIROp::ProofBlock {
            program_hash: "proof".into(),
            body: leaf,
        },
    ];
    let expected = vec![
        error.clone(),
        TIROp::IfElse {
            then_body: vec![error.clone()],
            else_body: vec![error.clone()],
        },
        TIROp::IfOnly {
            then_body: vec![error.clone()],
        },
        TIROp::Loop {
            label: "loop".into(),
            body: vec![error.clone()],
        },
        TIROp::ProofBlock {
            program_hash: "proof".into(),
            body: vec![error],
        },
    ];
    assert_eq!(
        format!("{:?}", super::optimize(ops)),
        format!("{:?}", expected)
    );
}
