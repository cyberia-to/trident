//! Lexer chunk boundaries preserve tokens while bounding trivia scan frames.
#[allow(dead_code)]
#[path = "../examples/selfhost_data/model.rs"]
mod model;
#[path = "native_control/support.rs"]
mod support;

use nox::{artifact, sequential, Outcome, Reduction};
use std::sync::OnceLock;
use trident::NATIVE_ARTIFACT_LIMITS as LIMITS;

fn fixtures() -> &'static [Vec<u8>; 2] {
    static FIXTURES: OnceLock<[Vec<u8>; 2]> = OnceLock::new();
    FIXTURES.get_or_init(|| {
        [true, false].map(|before| {
            let mut options = trident::CompileOptions::default();
            if before {
                options.module_sources.insert(
                    "std.compiler.nox.lexer".into(),
                    include_str!("fixtures/native_lexer_frames_before.tri").into(),
                );
            }
            trident::compile_native_artifact_project(
                &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/native_lexer_frames.tri"),
                &options,
                trident::NativeArtifactProfile::RawNoun,
                LIMITS,
            )
            .unwrap()
            .bytes
        })
    })
}

fn inspect<const N: usize>(source: &[u8], start: u64, before: bool, nodes: u32) -> [u64; 4] {
    let mut arena = Reduction::<N>::try_new_boxed().unwrap();
    assert!(arena.limit_allocations(nodes));
    let root = artifact::decode(&mut arena, &fixtures()[usize::from(!before)], LIMITS).unwrap();
    let mut fields = arena.tail(root).unwrap();
    for _ in 0..3 {
        fields = arena.tail(fields).unwrap();
    }
    let formula = arena.head(fields).unwrap();
    let content = model::Bytes::from_slice(&mut arena, source, 65536)
        .unwrap()
        .encode(&mut arena)
        .unwrap();
    let start = model::atom(&mut arena, start).unwrap();
    let input = model::pair(&mut arena, content, start).unwrap();
    let execution = sequential::reduce_cached(
        &mut arena,
        input,
        formula,
        1_000_000_000,
        sequential::Limits { max_frames: 65536 },
    )
    .unwrap();
    let mut result = match execution.outcome {
        Outcome::Ok(value, remaining) => {
            if source.len() == 65536 {
                eprintln!(
                    "lexer bytes={} reductions={} nodes={} frames={}",
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
    let mut words = [0; 4];
    for word in &mut words[..3] {
        *word = arena
            .atom_value(arena.head(result).unwrap())
            .unwrap()
            .as_u64();
        result = arena.tail(result).unwrap();
    }
    words[3] = arena.atom_value(result).unwrap().as_u64();
    words
}

#[test]
fn chunk_boundaries_preserve_every_byte_class_and_exact_token_spans() {
    support::worker(|| {
        let compare = |source: &[u8], start| {
            assert_eq!(
                inspect::<{ 1 << 18 }>(source, start, false, 196608),
                inspect::<{ 1 << 18 }>(source, start, true, 196608),
                "source={source:?}, start={start}"
            );
        };
        for byte in 0..=255 {
            compare(&[byte, b'x', b'=', b'7', b';'], 0);
        }
        for offset in [0, 1, 254, 255, 256, 257, 511, 512] {
            for token in [
                "",
                "(",
                ")",
                "name",
                "fn",
                "0007",
                "18446744073709551616",
                "+",
                "*",
                "{",
                "}",
                "->",
                "-",
                "/%",
                "/",
                "[",
                "]",
                ";",
                "#",
                ",",
                "<",
                "&",
                "..",
                ".",
                ":",
                "==",
                "=",
                "?",
                "@",
                "//comment",
                "//comment\n7",
                "//comment\r7",
                "//\n//second\nname",
            ] {
                let source = format!("{}{token}", " ".repeat(offset));
                compare(source.as_bytes(), 0);
            }
            let source = format!("{}//comment\n7", " ".repeat(offset));
            for start in [offset, offset + 1, source.len()] {
                compare(source.as_bytes(), start as u64);
            }
        }
        // A newline at the chunk's last byte must carry comment=false into
        // the next chunk, where the following token begins immediately.
        for newline in [255, 511] {
            let source = format!("//{}\n7", "x".repeat(newline - 2));
            compare(source.as_bytes(), 0);
        }
    });
}

#[test]
fn full_source_trivia_reaches_eof_and_final_token_with_existing_frame_limit() {
    support::worker(|| {
        // Same component gas/node/frame limits as native_compiler/source_capacity.
        let mut source = vec![b' '; 65536];
        assert_eq!(
            inspect::<{ 1 << 24 }>(&source, 0, false, 12_582_912),
            [0, 65536, 65536, 0]
        );
        source[0] = b'/';
        source[1] = b'/';
        source[65534] = b'\n';
        source[65535] = b'7';
        assert_eq!(
            inspect::<{ 1 << 24 }>(&source, 0, false, 12_582_912),
            [2, 65535, 65536, 7]
        );
        source[65534] = b' ';
        assert_eq!(
            inspect::<{ 1 << 24 }>(&source, 0, false, 12_582_912),
            [0, 65536, 65536, 0]
        );
    });
}
