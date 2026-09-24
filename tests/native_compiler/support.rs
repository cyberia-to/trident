#![allow(dead_code)]
use nox::{artifact, sequential, NoTrace, Order, Outcome, Reduction};
use std::sync::OnceLock;
use trident::{CompileOptions, NativeArtifactProfile, NATIVE_ARTIFACT_LIMITS as LIMITS};
#[path = "../../examples/selfhost_data/model.rs"]
pub mod data;
#[path = "../native_control/support.rs"]
mod native;
#[path = "../../examples/selfhost_jobs/schema.rs"]
pub mod schema;
#[path = "../../examples/selfhost_jobs/validate.rs"]
pub mod validate;
pub use native::worker;
type Arena = Reduction<{ 1 << 18 }>;
pub const CAPS: [u64; 11] = [
    4096,
    128,
    16,
    4096,
    1_000_000,
    16_777_216,
    196608,
    4096,
    100_000_000,
    196608,
    65536,
];

pub fn compiler() -> &'static [u8] {
    static COMPILER: OnceLock<Vec<u8>> = OnceLock::new();
    COMPILER.get_or_init(|| {
        trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("compiler/nox/main.tri"),
            &CompileOptions::default(),
            NativeArtifactProfile::CompilerJob,
            LIMITS,
        )
        .unwrap()
        .bytes
    })
}

pub fn source(expression: &str) -> Vec<u8> {
    format!("program sample fn main() -> Field {{ {expression} }}").into_bytes()
}

pub fn module(source: &[u8]) -> schema::Module {
    schema::Module {
        path: "sample".into(),
        origin: "pilot".into(),
        version: "1".into(),
        source: source.to_vec(),
    }
}

pub fn options() -> schema::Options {
    schema::Options {
        input: 0,
        output: 0,
        optimization: 0,
        cfg: vec![],
    }
}

#[derive(Debug)]
pub enum Result {
    Program {
        bytes: Vec<u8>,
        value: u64,
        reductions: u64,
        nodes: u32,
        frames: u32,
    },
    Errors(Vec<schema::Diagnostic>),
}

fn formula(ar: &Arena, root: Order) -> Order {
    let mut p = ar.tail(root).unwrap();
    for _ in 0..3 {
        p = ar.tail(p).unwrap();
    }
    ar.head(p).unwrap()
}

pub fn compile(source: &[u8]) -> Result {
    compile_package(&[module(source)], "sample", "main", options(), CAPS)
}

pub fn compile_package(
    modules: &[schema::Module],
    name: &str,
    function: &str,
    opts: schema::Options,
    caps: [u64; 11],
) -> Result {
    try_compile_package(modules, name, function, opts, caps).unwrap()
}

pub fn try_compile_package(
    modules: &[schema::Module],
    name: &str,
    function: &str,
    opts: schema::Options,
    caps: [u64; 11],
) -> std::result::Result<Result, String> {
    let mut ar = Arena::new();
    assert!(ar.limit_allocations(caps[9] as u32));
    let c1 = artifact::decode(&mut ar, compiler(), LIMITS).unwrap();
    let job = schema::job(&mut ar, c1, modules, name, function, &opts, &caps).unwrap();
    let mut host = CAPS;
    host[0] = 4_194_304;
    host[3] = 65_536;
    let admitted = validate::job(&mut ar, job, c1, host).unwrap();
    let code = formula(&ar, c1);
    let run = sequential::reduce(
        &mut ar,
        job,
        code,
        caps[8],
        sequential::Limits {
            max_frames: caps[10] as u32,
        },
        &mut NoTrace,
    )
    .map_err(|error| format!("guest executor: {error:?}; nodes={}", ar.count()))?;
    let (result, left) = match run.outcome {
        Outcome::Ok(result, left) => (result, left),
        err => {
            return Err(format!(
                "guest execution: {err:?}; nodes={}; frames={}",
                ar.count(),
                run.peak_frames
            ))
        }
    };
    let nodes = ar.count();
    Ok(
        match validate::result(&mut ar, result, job, &admitted).unwrap() {
            validate::ResultValue::Failure(errors) => Result::Errors(errors),
            validate::ResultValue::Success {
                artifact: program, ..
            } => {
                let bytes = artifact::encode(&ar, program, LIMITS).unwrap();
                let value = native::run(&bytes, 0, 1_000_000, 65536, 196608).unwrap().0;
                Result::Program {
                    bytes,
                    value,
                    reductions: caps[8] - left,
                    nodes,
                    frames: run.peak_frames,
                }
            }
        },
    )
}

pub fn value(result: Result) -> u64 {
    match result {
        Result::Program { value, .. } => value,
        other => panic!("{other:?}"),
    }
}

pub fn error(source: &[u8], code: u32) -> schema::Diagnostic {
    match compile(source) {
        Result::Errors(mut errors) => {
            assert_eq!(errors.len(), 1);
            let error = errors.remove(0);
            assert_eq!(error.code, code, "source={source:?}");
            assert!(error.end as usize <= source.len());
            error
        }
        other => panic!("accepted {source:?}: {other:?}"),
    }
}

// Independent expression-tree construction, never derived from emitted output.
#[derive(Clone)]
pub enum Expr {
    Number(u64),
    Add(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
}
impl Expr {
    pub fn text(&self) -> String {
        match self {
            Self::Number(v) => v.to_string(),
            Self::Add(a, b) => format!("({}+{})", a.text(), b.text()),
            Self::Mul(a, b) => format!("({}*{})", a.text(), b.text()),
        }
    }
    pub fn value(&self) -> u64 {
        const P: u128 = 18446744069414584321;
        match self {
            Self::Number(v) => (*v as u128 % P) as u64,
            Self::Add(a, b) => ((a.value() as u128 + b.value() as u128) % P) as u64,
            Self::Mul(a, b) => ((a.value() as u128 * b.value() as u128) % P) as u64,
        }
    }
    fn noun(&self, ar: &mut Arena) -> Order {
        let (tag, a, b) = match self {
            Self::Number(v) => {
                let op = data::atom(ar, 1).unwrap();
                let value = data::atom(ar, nebu::Goldilocks::new(*v).as_u64()).unwrap();
                return data::pair(ar, op, value).unwrap();
            }
            Self::Add(a, b) => (5, a, b),
            Self::Mul(a, b) => (7, a, b),
        };
        let left = a.noun(ar);
        let right = b.noun(ar);
        let operands = data::pair(ar, left, right).unwrap();
        let op = data::atom(ar, tag).unwrap();
        data::pair(ar, op, operands).unwrap()
    }
    pub fn artifact(&self) -> Vec<u8> {
        let mut ar = Arena::new();
        let formula = self.noun(&mut ar);
        let root = schema::artifact(&mut ar, formula, 0, 0).unwrap();
        artifact::encode(&ar, root, LIMITS).unwrap()
    }
}

// Separate Rust-seed differential on the identical source, using its flat
// entry convention only as an oracle. Guest artifacts use canonical raw ART1.
pub fn rust_value(source: &str) -> u64 {
    fn load(ar: &mut Arena, text: &[u8], pos: &mut usize) -> Order {
        if text[*pos] == b'[' {
            *pos += 1;
            let a = load(ar, text, pos);
            assert_eq!(text[*pos], b' ');
            *pos += 1;
            let b = load(ar, text, pos);
            assert_eq!(text[*pos], b']');
            *pos += 1;
            data::pair(ar, a, b).unwrap()
        } else {
            let start = *pos;
            while *pos < text.len() && text[*pos].is_ascii_digit() {
                *pos += 1;
            }
            let value = std::str::from_utf8(&text[start..*pos])
                .unwrap()
                .parse::<u64>()
                .unwrap();
            ar.atom(nebu::Goldilocks::new(value)).unwrap()
        }
    }
    let assembly = trident::compile(source, "oracle.tri").unwrap();
    let mut ar = Arena::new();
    let mut pos = 0;
    let code = load(&mut ar, assembly.as_bytes(), &mut pos);
    assert_eq!(pos, assembly.len());
    let zero = data::atom(&mut ar, 0).unwrap();
    let run = sequential::reduce(
        &mut ar,
        zero,
        code,
        1_000_000,
        sequential::Limits { max_frames: 65536 },
        &mut NoTrace,
    )
    .unwrap();
    let mut result = match run.outcome {
        Outcome::Ok(value, _) => value,
        other => panic!("{other:?}"),
    };
    while let Some(head) = ar.head(result) {
        result = head;
    }
    ar.atom_value(result).unwrap().as_u64()
}

// Isolate the guest pass: the model's input reader has a different visit
// accounting implementation. Installed Joy tests cover host+guest together.
pub fn admission_boundary(source: &[u8]) {
    let mut ar = Arena::new();
    let name = schema::bytes(&mut ar, b"sample").unwrap();
    let function = schema::bytes(&mut ar, b"main").unwrap();
    let content = schema::bytes(&mut ar, source).unwrap();
    let zero = schema::atom(&mut ar, 0).unwrap();
    let modules = schema::seq(&mut ar, &[zero]).unwrap();
    let mut remaining = 1_000_000;
    data::Bytes::decode_budget(&mut ar, name, 255, &mut remaining).unwrap();
    data::Bytes::decode_budget(&mut ar, function, 255, &mut remaining).unwrap();
    data::Seq::decode_budget(&mut ar, modules, 4096, &mut remaining).unwrap();
    data::Bytes::decode_budget(&mut ar, content, 4096, &mut remaining).unwrap();
    // Bootstrap14, JOB10, LIM15, limit slots16, JOB slots14, OPT9,
    // profile slots7, PKG5, package slot1, module path1, MOD8, path slot1,
    // four main-byte reads4, source slot/length7 =112 fixed projections.
    let required = 112 + 1_000_000 - remaining;
    for allowance in [required - 4, required - 1, required, required + 1] {
        let mut ar = Arena::new();
        assert!(ar.limit_allocations(196608));
        let c1 = artifact::decode(&mut ar, compiler(), LIMITS).unwrap();
        let mut caps = CAPS;
        caps[4] = u64::from(allowance);
        let job = schema::job(
            &mut ar,
            c1,
            &[module(source)],
            "sample",
            "main",
            &options(),
            &caps,
        )
        .unwrap();
        let code = formula(&ar, c1);
        let result = sequential::reduce(
            &mut ar,
            job,
            code,
            CAPS[8],
            sequential::Limits { max_frames: 65536 },
            &mut NoTrace,
        );
        if allowance < required {
            assert!(
                !matches!(result,Ok(ref result) if matches!(result.outcome,Outcome::Ok(..))),
                "accepted {allowance} below {required}"
            );
        } else {
            let run = result.unwrap();
            let root = match run.outcome {
                Outcome::Ok(root, _) => root,
                other => panic!("{allowance}: {other:?}"),
            };
            let fields = ar.tail(root).unwrap();
            let identity = ar.head(fields).unwrap();
            assert_eq!(
                ar.read_hash_data(identity).unwrap(),
                *ar.digest(job).unwrap()
            );
            let status = ar.head(ar.tail(fields).unwrap()).unwrap();
            assert_eq!(ar.atom_value(status).unwrap().as_u64(), 0);
        }
    }
}
