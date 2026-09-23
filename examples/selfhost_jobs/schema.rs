//! SH0.3 reference constructors. These serialize nouns; they do not compile source.
use super::data;
use nox::{Order, Reduction};

pub const JOB: u64 = 0x4a4f4231;
pub const PACKAGE: u64 = 0x504b4731;
pub const MODULE: u64 = 0x4d4f4431;
pub const OPTIONS: u64 = 0x4f505431;
pub const LIMITS: u64 = 0x4c494d31;
pub const RESULT: u64 = 0x52455331;
pub const ARTIFACT: u64 = 0x41525431;
pub const DIAGNOSTIC: u64 = 0x44494131;
pub type Result<T> = std::result::Result<T, String>;

pub fn atom<const N: usize>(ar: &mut Reduction<N>, v: u64) -> Result<Order> {
    data::atom(ar, v).map_err(|e| format!("atom: {e:?}"))
}
pub fn pair<const N: usize>(ar: &mut Reduction<N>, a: Order, b: Order) -> Result<Order> {
    data::pair(ar, a, b).map_err(|e| format!("pair: {e:?}"))
}
pub fn record<const N: usize>(ar: &mut Reduction<N>, tag: u64, fields: &[Order]) -> Result<Order> {
    let mut body = atom(ar, 0)?;
    for &field in fields.iter().rev() {
        body = pair(ar, field, body)?;
    }
    let tag = atom(ar, tag)?;
    pair(ar, tag, body)
}
pub fn bytes<const N: usize>(ar: &mut Reduction<N>, bytes: &[u8]) -> Result<Order> {
    data::Bytes::from_slice(ar, bytes, u32::MAX)
        .and_then(|b| b.encode(ar))
        .map_err(|e| format!("bytes: {e:?}"))
}
pub fn seq<const N: usize>(ar: &mut Reduction<N>, fields: &[Order]) -> Result<Order> {
    data::Seq::from_values(ar, fields, u32::MAX)
        .and_then(|s| s.encode(ar))
        .map_err(|e| format!("seq: {e:?}"))
}
pub fn identity<const N: usize>(ar: &mut Reduction<N>, n: Order) -> Result<Order> {
    let digest = *ar.digest(n).ok_or("missing identity")?;
    ar.hash_data(&digest)
        .ok_or_else(|| "identity allocation".into())
}

pub fn artifact<const N: usize>(
    ar: &mut Reduction<N>,
    formula: Order,
    input: u64,
    output: u64,
) -> Result<Order> {
    let machine = atom(ar, 0)?;
    let input = atom(ar, input)?;
    let output = atom(ar, output)?;
    record(ar, ARTIFACT, &[machine, input, output, formula])
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Module {
    pub path: String,
    pub origin: String,
    pub version: String,
    pub source: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    pub input: u64,
    pub output: u64,
    pub optimization: u64,
    pub cfg: Vec<String>,
}

/// Positional LIM1 values match reference/self-hosting-jobs.md.
/// Host caps are independent of untrusted job requests.
pub const LIMIT_COUNT: usize = 11;
pub const FIXTURE_CAPS: [u64; LIMIT_COUNT] = [
    4096,
    32,
    16,
    4096,
    100_000,
    1 << 20,
    3000,
    128,
    1_000_000,
    3000,
    16_384,
];

pub fn job<const N: usize>(
    ar: &mut Reduction<N>,
    compiler: Order,
    modules: &[Module],
    entry: &str,
    function: &str,
    options: &Options,
    limits: &[u64; LIMIT_COUNT],
) -> Result<Order> {
    let mut module_nodes = Vec::new();
    for m in modules {
        let path = bytes(ar, m.path.as_bytes())?;
        let origin = bytes(ar, m.origin.as_bytes())?;
        let version = bytes(ar, m.version.as_bytes())?;
        let source = bytes(ar, &m.source)?;
        module_nodes.push(record(ar, MODULE, &[path, origin, version, source])?);
    }
    let modules = seq(ar, &module_nodes)?;
    let package = record(ar, PACKAGE, &[modules])?;
    let compiler = identity(ar, compiler)?;
    let entry = bytes(ar, entry.as_bytes())?;
    let function = bytes(ar, function.as_bytes())?;
    let zero = atom(ar, 0)?;
    let input = atom(ar, options.input)?;
    let output = atom(ar, options.output)?;
    let optimization = atom(ar, options.optimization)?;
    let flags = options
        .cfg
        .iter()
        .map(|s| bytes(ar, s.as_bytes()))
        .collect::<Result<Vec<_>>>()?;
    let flags = seq(ar, &flags)?;
    let options = record(ar, OPTIONS, &[zero, input, output, optimization, flags])?;
    let limits = limits
        .iter()
        .map(|&v| atom(ar, v))
        .collect::<Result<Vec<_>>>()?;
    let limits = record(ar, LIMITS, &limits)?;
    record(
        ar,
        JOB,
        &[compiler, package, entry, function, options, limits],
    )
}

pub fn success<const N: usize>(
    ar: &mut Reduction<N>,
    job: Order,
    artifact: Order,
) -> Result<Order> {
    let identity = identity(ar, job)?;
    let status = atom(ar, 0)?;
    record(ar, RESULT, &[identity, status, artifact])
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diagnostic {
    // Declaration order is the canonical sort key, independent of wire order.
    pub module: u32,
    pub start: u32,
    pub end: u32,
    pub code: u32,
    pub message: String,
}

pub fn failure<const N: usize>(
    ar: &mut Reduction<N>,
    job: Order,
    diagnostics: &[Diagnostic],
) -> Result<Order> {
    let mut nodes = Vec::new();
    for d in diagnostics {
        let code = atom(ar, u64::from(d.code))?;
        let module = atom(ar, u64::from(d.module))?;
        let start = atom(ar, u64::from(d.start))?;
        let end = atom(ar, u64::from(d.end))?;
        let message = bytes(ar, d.message.as_bytes())?;
        nodes.push(record(
            ar,
            DIAGNOSTIC,
            &[code, module, start, end, message],
        )?);
    }
    let diagnostics = seq(ar, &nodes)?;
    let identity = identity(ar, job)?;
    let status = atom(ar, 1)?;
    record(ar, RESULT, &[identity, status, diagnostics])
}
