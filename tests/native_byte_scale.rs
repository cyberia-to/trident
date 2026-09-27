//! Sequential packed-word traversal and complete UTF-8 span checks.
#[allow(dead_code)]
#[path = "../examples/selfhost_data/model.rs"]
mod model;
#[path = "native_control/support.rs"]
mod support;
use nox::{artifact, sequential, NoTrace, Order, Outcome, Reduction};
use std::sync::OnceLock;
use trident::NATIVE_ARTIFACT_LIMITS as LIMITS;

fn fixture() -> &'static [u8] {
    static FIXTURE: OnceLock<Vec<u8>> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let mut options = trident::CompileOptions::default();
        // A measurement can select committed library inputs without changing
        // the shared working tree. Production builds never read this variable.
        if let Ok(revision) = std::env::var("TRIDENT_BYTE_SCALE_REVISION") {
            for (module, path) in [
                ("std.nox.tree", "lib/std/nox/tree.tri"),
                ("std.nox.bytes", "lib/std/nox/bytes.tri"),
                ("std.compiler.nox.utf8", "lib/std/compiler/nox/utf8.tri"),
            ] {
                let source = std::process::Command::new("git")
                    .args(["show", &format!("{revision}:{path}")])
                    .current_dir(env!("CARGO_MANIFEST_DIR"))
                    .output()
                    .unwrap();
                assert!(source.status.success(), "{revision}:{path}");
                options
                    .module_sources
                    .insert(module.into(), String::from_utf8(source.stdout).unwrap());
            }
        }
        trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_byte_scale.tri"),
            &options,
            trident::NativeArtifactProfile::RawNoun,
            LIMITS,
        )
        .unwrap_or_else(|errors| panic!("byte scale fixture: {errors:?}"))
        .bytes
    })
}

fn inspect(source: &[u8], mode: u64) -> [u64; 3] {
    // Preserve the existing source_capacity component's explicit limits.
    inspect_with_arena::<{ 1 << 24 }>(source, mode, 12_582_912, true)
}

fn inspect_with_arena<const N: usize>(
    source: &[u8],
    mode: u64,
    nodes: u32,
    report: bool,
) -> [u64; 3] {
    let mut arena = Reduction::<N>::try_new_boxed().unwrap();
    assert!(arena.limit_allocations(nodes));
    let root = artifact::decode(&mut arena, fixture(), LIMITS).unwrap();
    let mut cursor = arena.tail(root).unwrap();
    for _ in 0..3 {
        cursor = arena.tail(cursor).unwrap();
    }
    let formula = arena.head(cursor).unwrap();
    let source_noun = model::Bytes::from_slice(&mut arena, source, 65536)
        .unwrap()
        .encode(&mut arena)
        .unwrap();
    let mode_noun = model::atom(&mut arena, mode).unwrap();
    let input = model::pair(&mut arena, source_noun, mode_noun).unwrap();
    let execution = sequential::reduce(
        &mut arena,
        input,
        formula,
        1_000_000_000,
        sequential::Limits { max_frames: 65536 },
        &mut NoTrace,
    )
    .unwrap();
    let output = match execution.outcome {
        Outcome::Ok(value, remaining) => {
            if report {
                eprintln!(
                    "byte-scale mode={mode} bytes={} reductions={} nodes={} frames={}",
                    source.len(),
                    1_000_000_000 - remaining,
                    arena.count(),
                    execution.peak_frames
                );
            }
            value
        }
        other => panic!(
            "{other:?}; nodes={}; frames={}",
            arena.count(),
            execution.peak_frames
        ),
    };
    if mode == 0 {
        assert_eq!(arena.digest(output), arena.digest(source_noun));
        return [0, source.len() as u64, source.len() as u64];
    }
    fn atom<const N: usize>(arena: &Reduction<N>, value: Order) -> u64 {
        arena.atom_value(value).unwrap().as_u64()
    }
    let rest = arena.tail(output).unwrap();
    [
        atom(&arena, arena.head(output).unwrap()),
        atom(&arena, arena.head(rest).unwrap()),
        atom(&arena, arena.tail(rest).unwrap()),
    ]
}

fn expected_utf8(source: &[u8]) -> [u64; 3] {
    match std::str::from_utf8(source) {
        Ok(_) => [0, source.len() as u64, source.len() as u64],
        Err(error) => {
            let start = error.valid_up_to();
            let end = if !(0xc2..=0xf4).contains(&source[start]) {
                start + 1
            } else {
                // Rust counts the valid prefix of an invalid sequence. Guest
                // diagnostics also cover the byte that invalidates it.
                error.error_len().map_or(source.len(), |n| start + n + 1)
            };
            [1, start as u64, end as u64]
        }
    }
}

fn compile(body: &str) -> Vec<u8> {
    let source = format!("program byte_cursor use vm.nox.noun use std.nox.tree use std.nox.bytes fn main(input:Noun)->Noun{{{body}}}");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("main.tri");
    std::fs::write(&path, source).unwrap();
    trident::compile_native_artifact_project(
        &path,
        &Default::default(),
        trident::NativeArtifactProfile::RawNoun,
        LIMITS,
    )
    .unwrap()
    .bytes
}

fn run<const N: usize>(
    arena: &mut Reduction<N>,
    program: &[u8],
    input: Order,
) -> Result<Order, String> {
    let root = artifact::decode(arena, program, LIMITS).unwrap();
    let mut cursor = arena.tail(root).unwrap();
    for _ in 0..3 {
        cursor = arena.tail(cursor).unwrap();
    }
    let formula = arena.head(cursor).unwrap();
    let execution = sequential::reduce(
        arena,
        input,
        formula,
        20_000_000,
        sequential::Limits { max_frames: 65536 },
        &mut NoTrace,
    )
    .unwrap();
    match execution.outcome {
        Outcome::Ok(value, _) => Ok(value),
        other => Err(format!("{other:?}")),
    }
}

#[test]
fn suffix_cursors_preserve_opaque_leaves_across_every_u32_tree_height() {
    support::worker(|| {
        let program = compile(
            "let root=noun.head(input) let args=noun.tail(input)
             let length=as_u32(noun.as_field(noun.head(args)))
             let start=as_u32(noun.as_field(noun.tail(args)))
             let mut c=tree.cursor_at(root,length,start)
             assert_eq(as_field(tree.remaining(c))+as_field(start),as_field(length))
             let mut pos=start
             for step in 0..8 {
                 if pos==length { return noun.atom(as_field(pos)) }
                 let old=c
                 let (value,next)=tree.read(c)
                 c=next
                 let (same,unused)=tree.read(old)
                 assert(noun.eq(value,same))
                 assert_eq(noun.as_field(noun.head(value)),88)
                 assert_eq(noun.as_field(noun.tail(value)),99)
                 pos=as_u32(as_field(pos)+1)
                 assert_eq(as_field(tree.remaining(c))+as_field(pos),as_field(length))
             }
             noun.atom(as_field(pos))",
        );
        let mut lengths = std::collections::BTreeSet::from([0, 1, 2, 3, u32::MAX]);
        for bit in 1..32 {
            let power = 1u32 << bit;
            lengths.extend([power - 1, power, power + 1]);
        }
        for length in lengths {
            for start in std::collections::BTreeSet::from([
                0,
                length / 2,
                length.saturating_sub(2),
                length.saturating_sub(1),
                length,
            ]) {
                let mut arena = Reduction::<{ 1 << 18 }>::try_new_boxed().unwrap();
                let left = model::atom(&mut arena, 88).unwrap();
                let right = model::atom(&mut arena, 99).unwrap();
                let mut root = model::pair(&mut arena, left, right).unwrap();
                for _ in 0..model::height(length) {
                    root = model::pair(&mut arena, root, root).unwrap();
                }
                // The directly constructed shared tree has opaque pair leaves.
                // Suffix reads stop before padding, so no admission is claimed.
                let length_noun = model::atom(&mut arena, u64::from(length)).unwrap();
                let start_noun = model::atom(&mut arena, u64::from(start)).unwrap();
                let args = model::pair(&mut arena, length_noun, start_noun).unwrap();
                let input = model::pair(&mut arena, root, args).unwrap();
                let output = run(&mut arena, &program, input)
                    .unwrap_or_else(|error| panic!("length={length}, start={start}: {error}"));
                assert_eq!(
                    arena.atom_value(output).unwrap().as_u64(),
                    u64::from(start) + u64::from((length - start).min(8)),
                );
            }
        }
    });
}

#[test]
fn byte_suffix_cursors_exclude_padding_and_match_every_packed_lane() {
    support::worker(|| {
        let program = compile(
            "let b=bytes.from_noun(input,as_u32(1024),as_u32(10000))
             for offset in 0..1025 {
                 if offset==bytes.len(b) {
                     assert_eq(as_field(tree.remaining(bytes.words_from(b,offset))),0)
                     return input
                 }
                 let cursor=bytes.words_from(b,offset)
                 let (value,next)=tree.read(cursor)
                 let word=as_u32(noun.as_field(value))
                 assert_eq(as_field(word),as_field(bytes.word_at(b,offset)))
                 assert_eq(as_field(bytes.word_byte(word,offset & as_u32(3))),as_field(bytes.get(b,offset)))
             }
             input",
        );
        for length in [0, 1, 2, 3, 4, 5, 31, 32, 33, 129, 513] {
            let mut arena = Reduction::<{ 1 << 20 }>::try_new_boxed().unwrap();
            let source: Vec<_> = (0..length).map(|i| (i * 37 + 11) as u8).collect();
            let input = model::Bytes::from_slice(&mut arena, &source, 1024)
                .unwrap()
                .encode(&mut arena)
                .unwrap();
            let output = run(&mut arena, &program, input).unwrap();
            assert_eq!(arena.digest(output), arena.digest(input));
        }
        for body in [
            "let c=tree.cursor(input,as_u32(0)) let (value,next)=tree.read(c) value",
            "let c=tree.cursor_at(input,as_u32(0),as_u32(1)) input",
            "let c=bytes.words_from(bytes.empty(),as_u32(1)) input",
            "noun.atom(as_field(bytes.word_at(bytes.empty(),as_u32(0))))",
            "noun.atom(as_field(bytes.word_byte(as_u32(0),as_u32(4))))",
        ] {
            let mut arena = Reduction::<{ 1 << 18 }>::try_new_boxed().unwrap();
            let input = model::atom(&mut arena, 0).unwrap();
            assert!(run(&mut arena, &compile(body), input).is_err(), "{body}");
        }
    });
}

#[test]
fn fused_admission_checks_every_word_and_partial_block_padding() {
    support::worker(|| {
        let program = compile("bytes.to_noun(bytes.from_noun(input,as_u32(1024),as_u32(10000)))");
        for length in [29, 31, 32, 33, 61, 63, 64, 65] {
            let count = (length + 3) / 4;
            for index in 0..count {
                for pair_leaf in [false, true] {
                    let mut arena = Reduction::<{ 1 << 18 }>::try_new_boxed().unwrap();
                    let zero = model::atom(&mut arena, 0).unwrap();
                    let bad = if pair_leaf {
                        model::pair(&mut arena, zero, zero).unwrap()
                    } else {
                        model::atom(&mut arena, 1 << 32).unwrap()
                    };
                    let mut words = vec![zero; count];
                    words[index] = bad;
                    let encoded = model::Seq::from_values(&mut arena, &words, 1024)
                        .unwrap()
                        .encode(&mut arena)
                        .unwrap();
                    let root = arena.tail(arena.tail(encoded).unwrap()).unwrap();
                    let length_noun = model::atom(&mut arena, length as u64).unwrap();
                    let body = model::pair(&mut arena, length_noun, root).unwrap();
                    let tag = model::atom(&mut arena, model::BYTES).unwrap();
                    let input = model::pair(&mut arena, tag, body).unwrap();
                    assert!(
                        run(&mut arena, &program, input).is_err(),
                        "length={length}, word={index}, pair={pair_leaf}"
                    );
                }
            }
        }
        for length in [1usize, 2, 3, 17, 29, 30, 31, 33, 61, 62, 63, 65] {
            let count = length.div_ceil(4);
            let capacity = count.next_power_of_two();
            let high_byte = (count - 1, 1u64 << ((length % 4) * 8));
            // Unused byte lanes and whole padding leaves must both be zero.
            for (index, value) in
                std::iter::once(high_byte).chain((count..capacity).map(|i| (i, 1)))
            {
                let mut arena = Reduction::<{ 1 << 18 }>::try_new_boxed().unwrap();
                let zero = model::atom(&mut arena, 0).unwrap();
                let bad = model::atom(&mut arena, value).unwrap();
                let mut words = vec![zero; capacity];
                words[index] = bad;
                let encoded = model::Seq::from_values(&mut arena, &words, 1024)
                    .unwrap()
                    .encode(&mut arena)
                    .unwrap();
                let root = arena.tail(arena.tail(encoded).unwrap()).unwrap();
                let length_noun = model::atom(&mut arena, length as u64).unwrap();
                let body = model::pair(&mut arena, length_noun, root).unwrap();
                let tag = model::atom(&mut arena, model::BYTES).unwrap();
                let input = model::pair(&mut arena, tag, body).unwrap();
                assert!(
                    run(&mut arena, &program, input).is_err(),
                    "padding length={length}, word={index}, value={value}"
                );
            }
        }
    });
}

#[test]
fn packed_utf8_preserves_every_byte_lane_and_exact_error_spans() {
    support::worker(|| {
        let inspect = |source: &[u8]| {
            assert_eq!(
                inspect_with_arena::<{ 1 << 18 }>(source, 1, 196608, false),
                expected_utf8(source),
                "{source:?}"
            );
        };
        inspect(&[]);
        for lane in 0..4 {
            for byte in 0..=255 {
                let mut source = vec![b' '; lane];
                source.push(byte);
                source.extend_from_slice(b"ASCII");
                inspect(&source);
            }
        }
        let sequences: &[&[u8]] = &[
            &[0xc2, 0x80],
            &[0xdf, 0xbf],
            &[0xe0, 0xa0, 0x80],
            &[0xed, 0x9f, 0xbf],
            &[0xee, 0x80, 0x80],
            &[0xef, 0xbf, 0xbf],
            &[0xf0, 0x90, 0x80, 0x80],
            &[0xf4, 0x8f, 0xbf, 0xbf],
            &[0xe0, 0x9f, 0xbf],
            &[0xed, 0xa0, 0x80],
            &[0xf0, 0x8f, 0xbf, 0xbf],
            &[0xf4, 0x90, 0x80, 0x80],
            &[0xe1, 0x80, b'A'],
            &[0xf1, 0x80, 0x80, b'A'],
        ];
        // Cross packed words, word chunks, and final partial words.
        for offset in [
            0, 1, 2, 3, 30, 31, 32, 62, 63, 64, 254, 255, 256, 2047, 2048,
        ] {
            for sequence in sequences {
                for length in 1..=sequence.len() {
                    let mut source = vec![b' '; offset];
                    source.extend_from_slice(&sequence[..length]);
                    inspect(&source);
                    source.extend_from_slice(b"ASCII");
                    inspect(&source);
                }
            }
        }
    });
}

#[test]
fn packed_source_scale_preserves_complete_admission_and_utf8() {
    support::worker(|| {
        for length in [128, 4096, 16384, 65536] {
            let source = vec![b' '; length];
            for mode in [0, 1] {
                assert_eq!(inspect(&source, mode), [0, length as u64, length as u64]);
            }
        }
    });
}

#[test]
fn utf8_preserves_the_final_error_span_at_the_source_ceiling() {
    support::worker(|| {
        let mut source = vec![b' '; 65536];
        source[65535] = 0xc2;
        assert_eq!(inspect(&source, 1), [1, 65535, 65536]);
    });
}
