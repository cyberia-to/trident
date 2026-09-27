use super::support::{self, schema, Compilation};
use nox::{artifact, sequential, NoTrace, Outcome, Reduction};
use std::sync::OnceLock;
use trident::NATIVE_ARTIFACT_LIMITS as LIMITS;

fn fixture() -> &'static [u8] {
    static FIXTURE: OnceLock<Vec<u8>> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_source_bounds.tri"),
            &Default::default(),
            trident::NativeArtifactProfile::RawNoun,
            LIMITS,
        )
        .unwrap_or_else(|errors| panic!("source bounds fixture: {errors:?}"))
        .bytes
    })
}

// Component measurements use explicit limits above Joy's supported job tier.
// Complete JOB1 tests below retain 3145728 nodes and 100M reductions.
fn inspect(source: &[u8], mode: u64, offset: u64) -> [u64; 4] {
    let mut arena = Reduction::<{ 1 << 24 }>::try_new_boxed().unwrap();
    assert!(arena.limit_allocations(12_582_912));
    let root = artifact::decode(&mut arena, fixture(), LIMITS).unwrap();
    let mut cursor = arena.tail(root).unwrap();
    for _ in 0..3 {
        cursor = arena.tail(cursor).unwrap();
    }
    let formula = arena.head(cursor).unwrap();
    let content = schema::bytes(&mut arena, source).unwrap();
    let mode = schema::atom(&mut arena, mode).unwrap();
    let offset = schema::atom(&mut arena, offset).unwrap();
    let args = schema::pair(&mut arena, mode, offset).unwrap();
    let input = schema::pair(&mut arena, content, args).unwrap();
    let execution = sequential::reduce(
        &mut arena,
        input,
        formula,
        1_000_000_000,
        sequential::Limits { max_frames: 65536 },
        &mut NoTrace,
    )
    .unwrap();
    let mut result = match execution.outcome {
        Outcome::Ok(value, remaining) => {
            eprintln!(
                "source component: bytes={}, reductions={}, nodes={}, frames={}",
                source.len(),
                1_000_000_000 - remaining,
                arena.count(),
                execution.peak_frames
            );
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
fn whitespace_classification_preserves_every_byte_and_rejects_wide_values() {
    support::worker(|| {
        let expected: Vec<u8> = (0..=255)
            .map(|value| u8::from(matches!(value, 9 | 10 | 12 | 13 | 32)))
            .collect();
        for value in [256, 65536, u32::MAX as u64] {
            assert_eq!(inspect(&expected, 4, value), [0, value, 256, 0]);
        }
    });
}

#[test]
fn source_byte_ceiling_is_independent_of_internal_id_sentinels() {
    support::worker(|| {
        for length in [4096, 4097, 65536, 65537] {
            let mut source = vec![0; length];
            source[0] = 255;
            let mut caps = support::CAPS;
            caps[0] = 131072;
            caps[4] = 2_000_000;
            caps[9] = 3145728;
            match support::try_compile_only_package(
                &[support::module(&source)],
                "sample",
                "main",
                support::options(),
                caps,
            ) {
                Ok(Compilation::Errors(errors)) => {
                    assert_eq!(errors[0].code, if length <= 65536 { 1 } else { 7 });
                    assert_eq!(errors[0].module, 0);
                }
                Err(error) if length == 65536 => {
                    // Admission capacity is separate from lifetime execution memory.
                    assert!(error.contains("Error(Unavailable)"), "{error}");
                    assert!(error.contains("nodes=3145728"), "{error}");
                    eprintln!("exact 65536-byte JOB1 runtime boundary: {error}");
                }
                other => panic!("{length}: {other:?}"),
            }
        }
    });
}

#[test]
fn lexer_preserves_complete_tokens_and_offsets_beyond_the_old_source_ceiling() {
    support::worker(|| {
        let identifier = format!("{}(", "x".repeat(4097));
        assert_eq!(inspect(identifier.as_bytes(), 0, 0), [1, 0, 4097, 0]);
        let decimal = format!("{}7;", "0".repeat(4096));
        assert_eq!(inspect(decimal.as_bytes(), 0, 0), [2, 0, 4097, 7]);
        let overflow = format!("{}18446744073709551616", "0".repeat(4096));
        assert_eq!(inspect(overflow.as_bytes(), 0, 0), [14, 0, 4116, 0]);
        let comment = format!("//{}\n7", "x".repeat(4096));
        assert_eq!(inspect(comment.as_bytes(), 0, 0), [2, 4099, 4100, 7]);
        let mut end = vec![b' '; 65536];
        end[65535] = b'7';
        assert_eq!(inspect(&end, 0, 65535), [2, 65535, 65536, 7]);
    });
}

#[test]
fn utf8_validation_reaches_the_end_of_the_admitted_source() {
    support::worker(|| {
        let mut source = vec![b' '; 65536];
        assert_eq!(inspect(&source, 1, 0), [0, 65536, 65536, 0]);
        source[65535] = 0xc2;
        assert_eq!(inspect(&source, 1, 0), [1, 65535, 65536, 0]);
        source.truncate(4096);
        source.extend_from_slice(&[0xe0, 0x80, 0x80]);
        assert_eq!(inspect(&source, 1, 0), [1, 4096, 4098, 0]);
    });
}

#[test]
fn long_names_and_line_checks_do_not_accept_matching_prefixes() {
    support::worker(|| {
        let name = "x".repeat(4097);
        let same = format!("{name} {name}");
        assert_eq!(inspect(same.as_bytes(), 2, 0), [4097, 4097, 8195, 1]);
        let different = format!("{name} {}y", "x".repeat(4096));
        assert_eq!(inspect(different.as_bytes(), 2, 0), [4097, 4097, 8195, 0]);
        let mut line = vec![b' '; 4098];
        line[4097] = b'\n';
        assert_eq!(inspect(&line, 3, 0), [0, 0, 4098, 0]);
    });
}

#[test]
fn full_jobs_use_the_explicit_total_source_allowance_across_modules() {
    support::worker(|| {
        let dependency = format!("module a pub const X:Field=13 //{}", " ".repeat(4096));
        let modules = [
            schema::Module {
                path: "a".into(),
                origin: "source-boundary".into(),
                version: "1".into(),
                source: dependency.into_bytes(),
            },
            support::module(b"program sample use a fn main()->Field{a.X}"),
        ];
        let total = modules.iter().map(|m| m.source.len() as u64).sum::<u64>();
        let mut caps = support::CAPS;
        caps[0] = total;
        caps[9] = 3145728;
        let result =
            support::try_compile_package(&modules, "sample", "main", support::options(), caps)
                .unwrap();
        assert_eq!(support::value(result), 13);

        let mut arena = Reduction::<{ 1 << 20 }>::try_new_boxed().unwrap();
        let compiler = artifact::decode(&mut arena, support::compiler(), LIMITS).unwrap();
        let job = schema::job(
            &mut arena,
            compiler,
            &modules,
            "sample",
            "main",
            &support::options(),
            &caps,
        )
        .unwrap();
        assert!(support::validate::job(&mut arena, job, compiler, caps).is_ok());
        caps[0] -= 1;
        let short = schema::job(
            &mut arena,
            compiler,
            &modules,
            "sample",
            "main",
            &support::options(),
            &caps,
        )
        .unwrap();
        assert!(support::validate::job(&mut arena, short, compiler, caps).is_err());
    });
}

#[test]
fn declarations_and_diagnostics_retain_offsets_after_large_comment_prefixes() {
    support::worker(|| {
        let prefix = format!("program sample //{}\n", " ".repeat(4096));
        let source = format!("{prefix}fn main()->Field{{13}}");
        let mut caps = support::CAPS;
        caps[0] = source.len() as u64;
        caps[9] = 3145728;
        let actual = support::try_compile_only_package(
            &[support::module(source.as_bytes())],
            "sample",
            "main",
            support::options(),
            caps,
        )
        .unwrap();
        let baseline = support::compile_only(&support::source("13"), support::CAPS);
        match (actual, baseline) {
            (
                Compilation::Program { bytes: actual, .. },
                Compilation::Program {
                    bytes: expected, ..
                },
            ) => {
                assert_eq!(actual, expected);
            }
            other => panic!("{other:?}"),
        }
        let invalid = format!("{prefix}fn main()->Field{{missing}}");
        caps[0] = invalid.len() as u64;
        match support::try_compile_only_package(
            &[support::module(invalid.as_bytes())],
            "sample",
            "main",
            support::options(),
            caps,
        )
        .unwrap()
        {
            Compilation::Errors(errors) => {
                assert_eq!(errors[0].code, 5);
                assert!(errors[0].start > 4096);
                assert_eq!(
                    &invalid[errors[0].start as usize..errors[0].end as usize],
                    "missing"
                );
            }
            other => panic!("{other:?}"),
        }
    });
}
