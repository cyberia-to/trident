use super::{atom, compile, empty_tree, equal, pair, run, support, Arena};
use std::collections::BTreeSet;

#[test]
fn indexed_reads_preserve_pair_leaves_at_every_u32_height_and_reject_the_end() {
    support::worker(|| {
        let program = compile("let args=noun.tail(input) tree.get(noun.head(input),as_u32(noun.as_field(noun.head(args))),as_u32(noun.as_field(noun.tail(args))))");
        let mut lengths: BTreeSet<u32> = (1..=9).collect();
        for bit in 1..32 {
            let power = 1u32 << bit;
            lengths.extend([power - 1, power, power + 1]);
        }
        lengths.insert(u32::MAX);
        for length in lengths {
            let height = 32 - (length - 1).leading_zeros();
            let positions: BTreeSet<_> = [0, (length - 1).min(1), length / 2, length - 1]
                .into_iter()
                .collect();
            for index in positions {
                let mut arena = Arena::new();
                let marker = atom(&mut arena, 77).unwrap();
                let at = atom(&mut arena, u64::from(index)).unwrap();
                let expected = pair(&mut arena, marker, at).unwrap();
                let mut root = expected;
                // Independent bottom-up construction; index bits select where
                // the complete pair leaf belongs in the canonical zero tree.
                for level in 0..height {
                    let empty = empty_tree(&mut arena, level as usize);
                    root = if index & (1u32 << level) == 0 {
                        pair(&mut arena, root, empty).unwrap()
                    } else {
                        pair(&mut arena, empty, root).unwrap()
                    };
                }
                let count = atom(&mut arena, u64::from(length)).unwrap();
                let args = pair(&mut arena, count, at).unwrap();
                let input = pair(&mut arena, root, args).unwrap();
                let actual = run(&mut arena, &program, input)
                    .unwrap_or_else(|error| panic!("length={length}, index={index}: {error}"));
                equal(&arena, actual, expected);
            }
            let mut arena = Arena::new();
            let root = empty_tree(&mut arena, height as usize);
            let count = atom(&mut arena, u64::from(length)).unwrap();
            let args = pair(&mut arena, count, count).unwrap();
            let input = pair(&mut arena, root, args).unwrap();
            assert!(run(&mut arena, &program, input).is_err(), "length={length}");
        }
    });
}
