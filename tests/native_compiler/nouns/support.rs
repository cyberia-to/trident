use super::super::support::{self, Compilation};
use nox::{artifact, sequential, NoTrace, Order, Outcome, Reduction};
use trident::NATIVE_ARTIFACT_LIMITS as LIMITS;

#[derive(Clone, Debug)]
pub enum Noun {
    Atom(u64),
    Pair(Box<Noun>, Box<Noun>),
}
use Noun::Atom;
impl Noun {
    pub fn pair(a: Self, b: Self) -> Self {
        Self::Pair(Box::new(a), Box::new(b))
    }
    fn load<const N: usize>(&self, arena: &mut Reduction<N>) -> Order {
        match self {
            Self::Atom(value) => support::data::atom(arena, *value).unwrap(),
            Self::Pair(a, b) => {
                let a = a.load(arena);
                let b = b.load(arena);
                support::data::pair(arena, a, b).unwrap()
            }
        }
    }
    pub fn encoded(&self) -> Vec<u8> {
        let mut arena = Reduction::<{ 1 << 18 }>::try_new_boxed().unwrap();
        let root = self.load(&mut arena);
        artifact::encode(&arena, root, LIMITS).unwrap()
    }
}
pub fn nested() -> Noun {
    let shared = Noun::pair(Atom(17), Noun::pair(Atom(29), Atom(18446744069414584320)));
    Noun::pair(shared.clone(), shared)
}
pub fn caps() -> [u64; 11] {
    let mut limits = support::CAPS;
    limits[9] = 786432;
    limits
}
pub fn compile(source: &str) -> Vec<u8> {
    match support::compile_only(source.as_bytes(), caps()) {
        Compilation::Program { bytes, .. } => bytes,
        other => panic!("{source}: {other:?}"),
    }
}
pub fn source(body: &str) -> String {
    format!("program sample fn main(input:Noun)->Noun{{{body}}}")
}
pub fn seed(source: &str) -> Vec<u8> {
    trident::compile_raw_artifact(
        source,
        "oracle.tri",
        &trident::CompileOptions::default(),
        LIMITS,
    )
    .unwrap()
    .bytes
}
#[derive(Debug, PartialEq)]
pub struct Run {
    pub bytes: Vec<u8>,
    pub reductions: u64,
    pub frames: u32,
    pub nodes: u32,
}
pub fn run(bytes: &[u8], input: &Noun) -> Result<Run, String> {
    run_traced(bytes, input, 1_000_000, 65536, 196608, &mut NoTrace)
}
pub fn run_traced(
    bytes: &[u8],
    input: &Noun,
    budget: u64,
    frames: u32,
    nodes: u32,
    tracer: &mut impl nox::trace::Tracer,
) -> Result<Run, String> {
    let mut arena = Reduction::<{ 1 << 18 }>::try_new_boxed().unwrap();
    assert!(arena.limit_allocations(nodes));
    let root = artifact::decode(&mut arena, bytes, LIMITS).map_err(|e| format!("{e:?}"))?;
    let mut cursor = arena.tail(root).unwrap();
    for _ in 0..3 {
        cursor = arena.tail(cursor).unwrap();
    }
    let formula = arena.head(cursor).unwrap();
    let input = input.load(&mut arena);
    let execution = sequential::reduce(
        &mut arena,
        input,
        formula,
        budget,
        sequential::Limits { max_frames: frames },
        tracer,
    )
    .map_err(|e| format!("{e:?}"))?;
    match execution.outcome {
        Outcome::Ok(result, remaining) => Ok(Run {
            bytes: artifact::encode(&arena, result, LIMITS).unwrap(),
            reductions: budget - remaining,
            frames: execution.peak_frames,
            nodes: arena.count(),
        }),
        result => Err(format!("{result:?}")),
    }
}
pub fn agrees(source: &str, input: &Noun, expected: &Noun) -> Vec<u8> {
    let bytes = compile(source);
    let expected = expected.encoded();
    assert_eq!(run(&bytes, input).unwrap().bytes, expected, "{source}");
    assert_eq!(
        run(&seed(source), input).unwrap().bytes,
        expected,
        "seed {source}"
    );
    bytes
}
