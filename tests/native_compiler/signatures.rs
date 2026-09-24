use super::support;
use nox::{artifact, sequential, NoTrace, Outcome, Reduction};
use trident::{CompileOptions, NativeArtifactProfile, NATIVE_ARTIFACT_LIMITS as LIMITS};

fn headers(source: &str, cap: u64) -> (u64, usize, usize) {
    static PROBE: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    let code = PROBE.get_or_init(|| {
        trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_function_headers.tri"),
            &CompileOptions::default(),
            NativeArtifactProfile::RawNoun,
            LIMITS,
        )
        .unwrap()
        .bytes
    });
    let (arena, result) = run_probe(source, cap, code);
    let scalar = |order| arena.atom_value(order).unwrap().as_u64();
    let span = arena.tail(result).unwrap();
    (
        scalar(arena.head(result).unwrap()),
        scalar(arena.head(span).unwrap()) as usize,
        scalar(arena.tail(span).unwrap()) as usize,
    )
}

pub(super) type Arena = Reduction<{ 1 << 18 }>;

pub(super) fn run_probe(source: &str, cap: u64, code: &[u8]) -> (Arena, nox::Order) {
    run_component::<{ 1 << 18 }>(source, cap, code, 196608)
}

// Component-only budgets are explicit; full JOB tests keep support::CAPS.
pub(super) fn run_component<const N: usize>(
    source: &str,
    cap: u64,
    code: &[u8],
    nodes: u32,
) -> (Reduction<N>, nox::Order) {
    let mut arena = Reduction::<N>::new();
    assert!(arena.limit_allocations(nodes));
    let artifact = artifact::decode(&mut arena, code, LIMITS).unwrap();
    let mut body = arena.tail(artifact).unwrap();
    for _ in 0..3 {
        body = arena.tail(body).unwrap();
    }
    let formula = arena.head(body).unwrap();
    let bytes = support::schema::bytes(&mut arena, source.as_bytes()).unwrap();
    let cap = support::data::atom(&mut arena, cap).unwrap();
    let input = support::data::pair(&mut arena, bytes, cap).unwrap();
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
        Outcome::Ok(value, _) => value,
        other => panic!(
            "{source}: {other:?}; nodes={}; frames={}",
            arena.count(),
            run.peak_frames
        ),
    };
    (arena, result)
}

#[test]
fn native_headers_retain_positional_parameters_and_comment_safe_body_spans() {
    support::worker(|| {
        let source = "program sample fn helper(x:Field,b:Bool,)->Bool {// }\n if b {true} else {false}} fn main()->Field {7}";
        let (summary, start, end) = headers(source, 4096);
        assert_eq!(summary, 10202);
        assert_eq!(&source[start..end], "// }\n if b {true} else {false}}");
        assert_eq!(
            headers(
                "program sample fn main()->Field{1} fn main()->Field{2}",
                4096
            )
            .0,
            10200
        );
        assert_eq!(
            headers(
                "program sample fn f(x:Field,x:Field){} fn main()->Field{7}",
                4096
            )
            .0,
            10202
        );
        assert_eq!(
            headers("program sample fn f(){} fn main()->Field{7}", 4096).0,
            10200
        );
    });
}

#[test]
fn native_headers_reject_missing_entry_types_delimiters_and_table_growth() {
    support::worker(|| {
        for (source, cap, error) in [
            ("program sample", 4096, 3),
            ("program sample fn f()->Field{7}", 4096, 3),
            ("program sample fn main(x:Field)->Field{x}", 4096, 3),
            ("program sample fn main(){}", 4096, 3),
            ("program sample fn main()->Bool{true}", 4096, 3),
            ("program sample fn main()->Field{", 4096, 2),
            (
                "program sample fn main()->Field{7} fn f(x Field){}",
                4096,
                2,
            ),
            (
                "program sample fn main()->Field{7} fn f(x:Field y:Bool){}",
                4096,
                2,
            ),
            ("program sample fn main()->Field{7} fn f()->{}", 4096, 2),
            (
                "program sample fn main()->Field{7} fn f(x:XField){}",
                4096,
                6,
            ),
            (
                "program sample fn main()->Field{7} fn f()->Other{}",
                4096,
                6,
            ),
            ("program sample fn main()->Field{7}", 0, 7),
            ("program sample fn main()->Field{7} fn f(){}", 1, 7),
            (
                "program sample fn main()->Field{7} fn f(x:Field,y:Bool){}",
                1,
                7,
            ),
        ] {
            let (actual, start, end) = headers(source, cap);
            assert_eq!(actual, error * 1_000_000, "{source}");
            assert!(start <= end && end <= source.len());
        }
        let source = "program sample fn f(x:Field,y:Bool){} fn main()->Field{7}";
        assert_eq!(headers(source, 1).0, 7_000_000);
        assert_eq!(headers(source, 2).0, 10202);
    });
}
