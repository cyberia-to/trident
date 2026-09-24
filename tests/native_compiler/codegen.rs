use super::support;
use nox::{artifact, sequential, NoTrace, Outcome, Reduction};
use trident::{CompileOptions, NativeArtifactProfile, NATIVE_ARTIFACT_LIMITS as LIMITS};

// This isolates the generator's own depth check. Full JOB admission also checks
// C1 depth, so it cannot admit small output limits below the compiler's depth.
fn generate(cap: u64) -> Result<Vec<u8>, u32> {
    static PROBE: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    let bytes = PROBE.get_or_init(|| {
        trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_codegen_depth.tri"),
            &CompileOptions::default(),
            NativeArtifactProfile::RawNoun,
            LIMITS,
        )
        .unwrap()
        .bytes
    });
    let mut arena = Reduction::<{ 1 << 18 }>::new();
    assert!(arena.limit_allocations(196608));
    let compiler = artifact::decode(&mut arena, bytes, LIMITS).unwrap();
    let mut fields = arena.tail(compiler).unwrap();
    for _ in 0..3 {
        fields = arena.tail(fields).unwrap();
    }
    let formula = arena.head(fields).unwrap();
    let input = support::data::atom(&mut arena, cap).unwrap();
    let run = sequential::reduce(
        &mut arena,
        input,
        formula,
        100_000_000,
        sequential::Limits { max_frames: 65536 },
        &mut NoTrace,
    )
    .unwrap();
    let result = match run.outcome {
        Outcome::Ok(result, _) => result,
        other => panic!("{other:?}"),
    };
    let code = arena
        .atom_value(arena.head(result).unwrap())
        .unwrap()
        .as_u64();
    if code != 0 {
        return Err(code as u32);
    }
    Ok(artifact::encode(&arena, arena.tail(result).unwrap(), LIMITS).unwrap())
}

// Independent DAG-depth measurement; it does not mirror codegen's recurrence.
fn artifact_depth(bytes: &[u8]) -> u64 {
    let count = u32::from_le_bytes(bytes[40..44].try_into().unwrap());
    let mut depths = std::collections::BTreeMap::<[u8; 32], u64>::new();
    let mut cursor = 44;
    for _ in 0..count {
        let particle = bytes[cursor..cursor + 32].try_into().unwrap();
        let width = usize::from(bytes[cursor + 32]);
        cursor += 33;
        let depth = if width == 8 {
            0
        } else {
            assert_eq!(width, 64);
            let a: [u8; 32] = bytes[cursor..cursor + 32].try_into().unwrap();
            let b: [u8; 32] = bytes[cursor + 32..cursor + 64].try_into().unwrap();
            1 + depths[&a].max(depths[&b])
        };
        depths.insert(particle, depth);
        cursor += width;
    }
    assert_eq!(cursor, bytes.len());
    let root: [u8; 32] = bytes[8..40].try_into().unwrap();
    depths[&root]
}

#[test]
fn generator_depth_matches_independent_dag_with_result_wrapper() {
    support::worker(|| {
        let expected = generate(4096).unwrap();
        let exact = artifact_depth(&expected) + 4;
        assert_eq!(generate(exact - 1), Err(7));
        assert_eq!(generate(exact).unwrap(), expected);
        assert_eq!(generate(exact + 1).unwrap(), expected);
    });
}
