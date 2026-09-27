//! Exact legacy lookup allowance with packed-word path comparison.
#[allow(dead_code)]
#[path = "../examples/selfhost_data/model.rs"]
mod model;
#[path = "native_control/support.rs"]
mod support;
use nox::{artifact, sequential, NoTrace, Outcome, Reduction};
use std::sync::OnceLock;
use trident::NATIVE_ARTIFACT_LIMITS as LIMITS;

fn fixture() -> &'static [u8] {
    static FIXTURE: OnceLock<Vec<u8>> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let path = "lib/std/compiler/nox/job.tri";
        let source = if let Ok(revision) = std::env::var("TRIDENT_JOB_COMPARE_REVISION") {
            let output = std::process::Command::new("git")
                .args(["show", &format!("{revision}:{path}")])
                .current_dir(env!("CARGO_MANIFEST_DIR"))
                .output()
                .unwrap();
            assert!(output.status.success());
            String::from_utf8(output.stdout).unwrap()
        } else {
            std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
                .unwrap()
        };
        // Probe the exact private comparator source; no production visibility
        // or lookup/admission behavior changes for test access.
        let source = source.replacen("module std.compiler.nox.job", "program job_word_compare", 1);
        let source = format!(
            "{source}\n#[pure]\nfn main(input:Noun)->Noun{{
            let left=bytes.from_noun(noun.head(input),as_u32(255),as_u32(10000))
            let args=noun.tail(input)
            let right=bytes.from_noun(noun.head(args),as_u32(255),as_u32(10000))
            let allowance=as_u32(noun.as_field(noun.tail(args)))
            let (earlier,remaining)=before(left,right,allowance)
            let mut flag:Field=0
            if earlier {{ flag=1 }}
            noun.pair(noun.atom(flag),noun.atom(as_field(remaining)))
        }}"
        );
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("main.tri");
        std::fs::write(&file, source).unwrap();
        trident::compile_native_artifact_project(
            &file,
            &Default::default(),
            trident::NativeArtifactProfile::RawNoun,
            LIMITS,
        )
        .unwrap()
        .bytes
    })
}

#[derive(Debug)]
struct Observation {
    earlier: bool,
    remaining: u64,
    reductions: u64,
    nodes: u32,
    frames: u32,
}

fn run(left: &[u8], right: &[u8], allowance: u64) -> Result<Observation, String> {
    let mut arena = Reduction::<{ 1 << 18 }>::try_new_boxed().unwrap();
    assert!(arena.limit_allocations(196608));
    let root = artifact::decode(&mut arena, fixture(), LIMITS).unwrap();
    let mut fields = arena.tail(root).unwrap();
    for _ in 0..3 {
        fields = arena.tail(fields).unwrap();
    }
    let formula = arena.head(fields).unwrap();
    let left = model::Bytes::from_slice(&mut arena, left, 255)
        .unwrap()
        .encode(&mut arena)
        .unwrap();
    let right = model::Bytes::from_slice(&mut arena, right, 255)
        .unwrap()
        .encode(&mut arena)
        .unwrap();
    let allowance = model::atom(&mut arena, allowance).unwrap();
    let args = model::pair(&mut arena, right, allowance).unwrap();
    let input = model::pair(&mut arena, left, args).unwrap();
    let execution = sequential::reduce(
        &mut arena,
        input,
        formula,
        20_000_000,
        sequential::Limits { max_frames: 65536 },
        &mut NoTrace,
    )
    .unwrap();
    match execution.outcome {
        Outcome::Ok(value, remaining) => Ok(Observation {
            earlier: arena
                .atom_value(arena.head(value).unwrap())
                .unwrap()
                .as_u64()
                == 1,
            remaining: arena
                .atom_value(arena.tail(value).unwrap())
                .unwrap()
                .as_u64(),
            reductions: 20_000_000 - remaining,
            nodes: arena.count(),
            frames: execution.peak_frames,
        }),
        other => Err(format!("{other:?}")),
    }
}

fn comparison_cost(left: &[u8], right: &[u8]) -> u64 {
    let compared = left
        .iter()
        .zip(right)
        .position(|(a, b)| a != b)
        .map_or(left.len().min(right.len()), |i| i + 1);
    let cost = |length: usize| u64::from(model::height(length.div_ceil(4) as u32) + 1);
    compared as u64 * (cost(left.len()) + cost(right.len()))
}

fn verify(left: &[u8], right: &[u8], test_short: bool) {
    let required = comparison_cost(left, right);
    for extra in [0, 19] {
        let result = run(left, right, required + extra).unwrap();
        assert_eq!(result.earlier, left < right, "{left:?}, {right:?}");
        assert_eq!(result.remaining, extra, "{left:?}, {right:?}");
    }
    if test_short && required > 0 {
        assert!(
            run(left, right, required - 1).is_err(),
            "{left:?}, {right:?}"
        );
    }
}

#[test]
fn packed_comparison_preserves_exact_spent_allowance_and_prefix_order() {
    support::worker(|| {
        for length in [
            0, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 254, 255,
        ] {
            let equal = vec![b'x'; length];
            verify(&equal, &equal, true);
            if length > 0 {
                verify(&equal[..length - 1], &equal, true);
                verify(&equal, &equal[..length - 1], true);
                for at in std::collections::BTreeSet::from([0, length / 2, length - 1]) {
                    let mut changed = equal.clone();
                    changed[at] = b'y';
                    verify(&equal, &changed, true);
                    verify(&changed, &equal, true);
                }
            }
        }
    });
}

#[test]
fn packed_comparison_orders_every_byte_in_every_lane() {
    support::worker(|| {
        for lane in 0..4 {
            for value in 0..=255u8 {
                let left = [0, 0, 0, 0, 128, 128, 128, 128];
                let mut right = left;
                right[4 + lane] = value;
                let required = comparison_cost(&left, &right);
                let result = run(&left, &right, required).unwrap();
                assert_eq!(result.earlier, left < right, "lane={lane}, value={value}");
                assert_eq!(result.remaining, 0);
            }
        }
    });
}

#[test]
fn word_comparison_resource_probe() {
    support::worker(|| {
        for prefix in [0, 3, 4, 15, 31, 127, 254] {
            let mut left = vec![b'x'; prefix];
            left.push(b'a');
            let mut right = left.clone();
            right[prefix] = b'z';
            let required = comparison_cost(&left, &right);
            let result = run(&left, &right, required).unwrap();
            assert!(result.earlier);
            assert_eq!(result.remaining, 0);
            eprintln!(
                "job-word prefix={prefix} visits={required} reductions={} nodes={} frames={}",
                result.reductions, result.nodes, result.frames
            );
        }
    });
}
