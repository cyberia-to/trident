//! Bounded structural admission. No lexing, import resolution or compilation.
use super::{data, schema::*};
use nox::{Order, Reduction};

struct Reader<'a, const N: usize> {
    ar: &'a mut Reduction<N>,
    visits: u32,
    seq_limit: u32,
}

impl<const N: usize> Reader<'_, N> {
    fn charge(&mut self, n: u32) -> Result<()> {
        self.visits = self
            .visits
            .checked_sub(n)
            .ok_or("validation visits exhausted")?;
        Ok(())
    }
    fn field(&mut self, n: Order) -> Result<u64> {
        self.charge(1)?;
        data::value(self.ar, n).map_err(|e| format!("field: {e:?}"))
    }
    fn record(&mut self, n: Order, tag: u64, count: usize) -> Result<Vec<Order>> {
        self.charge(2)?;
        let actual = self.ar.head(n).ok_or("record shape")?;
        if self.field(actual)? != tag {
            return Err("record tag".into());
        }
        let mut body = self.ar.tail(n).ok_or("record body")?;
        let mut fields = Vec::new();
        for _ in 0..count {
            self.charge(2)?;
            fields.push(self.ar.head(body).ok_or("record arity")?);
            body = self.ar.tail(body).ok_or("record arity")?;
        }
        if self.field(body)? != 0 {
            return Err("record terminator".into());
        }
        Ok(fields)
    }
    fn list(&mut self, n: Order, limit: u32) -> Result<Vec<Order>> {
        let seq = data::Seq::decode_budget(self.ar, n, limit.min(self.seq_limit), &mut self.visits)
            .map_err(|e| format!("sequence: {e:?}"))?;
        let mut result = Vec::new();
        for i in 0..seq.len() {
            self.charge(data::height(seq.len()) + 1)?;
            result.push(seq.get(self.ar, i).map_err(|e| format!("index: {e:?}"))?);
        }
        Ok(result)
    }
    fn bytes(&mut self, n: Order, limit: u32) -> Result<Vec<u8>> {
        let bytes = data::Bytes::decode_budget(self.ar, n, limit, &mut self.visits)
            .map_err(|e| format!("bytes: {e:?}"))?;
        let mut result = Vec::new();
        for i in 0..bytes.len() {
            self.charge(data::height(data::word_count(bytes.len())) + 1)?;
            result.push(
                bytes
                    .get(self.ar, i)
                    .map_err(|e| format!("byte index: {e:?}"))?,
            );
        }
        Ok(result)
    }
    fn string(&mut self, n: Order, limit: u32) -> Result<String> {
        String::from_utf8(self.bytes(n, limit)?).map_err(|_| "invalid UTF-8".into())
    }
}

pub fn path_valid(s: &str) -> bool {
    !s.is_empty() && s.len() <= 255 && s.split('.').all(identifier)
}
fn identifier(s: &str) -> bool {
    s.as_bytes()
        .first()
        .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
        && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
}
fn label(s: &str) -> bool {
    !s.is_empty() && s.len() <= 255 && s.bytes().all(|c| (32..=126).contains(&c))
}

pub struct Job {
    identity: nox::Digest,
    modules: Vec<Module>,
    entry: String,
    function: String,
    options: Options,
    limits: [u64; LIMIT_COUNT],
}

impl Job {
    pub fn modules(&self) -> &[Module] {
        &self.modules
    }
    pub fn entry(&self) -> &str {
        &self.entry
    }
    pub fn function(&self) -> &str {
        &self.function
    }
    pub fn options(&self) -> &Options {
        &self.options
    }
}

pub fn job<const N: usize>(
    ar: &mut Reduction<N>,
    root: Order,
    compiler: Order,
    caps: [u64; LIMIT_COUNT],
) -> Result<Job> {
    check_container(ar, root, caps)?;
    check_container(ar, compiler, caps)?;
    let mut r = Reader {
        ar,
        visits: u32::try_from(caps[4]).map_err(|_| "host visit cap")?,
        seq_limit: u32::try_from(caps[3]).map_err(|_| "host seq cap")?,
    };
    program(
        &mut r,
        compiler,
        &Options {
            input: 1,
            output: 1,
            optimization: 0,
            cfg: Vec::new(),
        },
    )?;
    let fields = r.record(root, JOB, 6)?;
    let expected =
        r.ar.read_hash_data(fields[0])
            .ok_or("compiler digest shape")?;
    if r.ar.digest(compiler) != Some(&expected) {
        return Err("compiler identity mismatch".into());
    }
    let lim = r.record(fields[5], LIMITS, LIMIT_COUNT)?;
    let mut limits = [0; LIMIT_COUNT];
    for (i, n) in lim.into_iter().enumerate() {
        let v = r.field(n)?;
        if v == 0 || v > caps[i] || (i != 8 && v > u64::from(u32::MAX)) {
            return Err("unsupported job limit".into());
        }
        limits[i] = v;
    }
    // Work already used admitting the limits record counts toward the request.
    let spent = (caps[4] as u32) - r.visits;
    r.visits = (limits[4] as u32)
        .checked_sub(spent)
        .ok_or("job visit limit")?;
    r.seq_limit = limits[3] as u32;
    check_container(r.ar, root, limits)?;
    check_container(r.ar, compiler, limits)?;
    let entry = r.string(fields[2], 255)?;
    let function = r.string(fields[3], 255)?;
    if !identifier(&entry) || !identifier(&function) {
        return Err("entry identifier".into());
    }
    let options = r.record(fields[4], OPTIONS, 5)?;
    if r.field(options[0])? != 0 {
        return Err("unsupported target".into());
    }
    let input = r.field(options[1])?;
    let output = r.field(options[2])?;
    let optimization = r.field(options[3])?;
    if input > 1 || input != output || optimization != 0 {
        return Err("unsupported compile option".into());
    }
    let mut cfg = Vec::new();
    for n in r.list(options[4], r.seq_limit)? {
        let flag = r.string(n, 255)?;
        if !identifier(&flag) || cfg.last().is_some_and(|s| s >= &flag) {
            return Err("cfg order/name".into());
        }
        cfg.push(flag);
    }
    let package = r.record(fields[1], PACKAGE, 1)?;
    let mut modules: Vec<Module> = Vec::new();
    let mut remaining_source = limits[0] as u32;
    for n in r.list(package[0], limits[1] as u32)? {
        let m = r.record(n, MODULE, 4)?;
        let path = r.string(m[0], 255)?;
        if !path_valid(&path) || modules.last().is_some_and(|m| m.path >= path) {
            return Err("module order/name".into());
        }
        let origin = r.string(m[1], 255)?;
        let version = r.string(m[2], 255)?;
        if !label(&origin) || !label(&version) {
            return Err("origin label".into());
        }
        let source = r.bytes(m[3], remaining_source)?;
        remaining_source -= source.len() as u32;
        modules.push(Module {
            path,
            origin,
            version,
            source,
        });
    }
    if !modules.iter().any(|m| m.path == entry) {
        return Err("entry module absent".into());
    }
    Ok(Job {
        identity: *r.ar.digest(root).ok_or("missing job identity")?,
        modules,
        entry,
        function,
        options: Options {
            input,
            output,
            optimization,
            cfg,
        },
        limits,
    })
}

fn program<const N: usize>(r: &mut Reader<'_, N>, root: Order, options: &Options) -> Result<Order> {
    let fields = r.record(root, ARTIFACT, 4)?;
    if r.field(fields[0])? != 0
        || r.field(fields[1])? != options.input
        || r.field(fields[2])? != options.output
    {
        return Err("artifact profile mismatch".into());
    }
    Ok(fields[3])
}

pub enum ResultValue {
    Success { artifact: Order, formula: Order },
    Failure(Vec<Diagnostic>),
}

fn check_container<const N: usize>(
    ar: &Reduction<N>,
    root: Order,
    limits: [u64; LIMIT_COUNT],
) -> Result<()> {
    let cap = nox::artifact::Limits {
        max_bytes: usize::try_from(limits[5]).map_err(|_| "artifact byte cap")?,
        max_nodes: u32::try_from(limits[6]).map_err(|_| "artifact node cap")?,
        max_depth: u32::try_from(limits[7]).map_err(|_| "artifact depth cap")?,
    };
    nox::artifact::encode(ar, root, cap)
        .map(|_| ())
        .map_err(|e| format!("artifact: {e:?}"))
}

pub fn result<const N: usize>(
    ar: &mut Reduction<N>,
    root: Order,
    job_root: Order,
    job: &Job,
) -> Result<ResultValue> {
    if ar.digest(job_root) != Some(&job.identity) {
        return Err("admitted job mismatch".into());
    }
    check_container(ar, root, job.limits)?;
    let mut r = Reader {
        ar,
        visits: job.limits[4] as u32,
        seq_limit: job.limits[3] as u32,
    };
    let fields = r.record(root, RESULT, 3)?;
    let identity = r.ar.read_hash_data(fields[0]).ok_or("job digest shape")?;
    if r.ar.digest(job_root) != Some(&identity) {
        return Err("job identity mismatch".into());
    }
    match r.field(fields[1])? {
        0 => Ok(ResultValue::Success {
            artifact: fields[2],
            formula: program(&mut r, fields[2], &job.options)?,
        }),
        1 => {
            let mut diagnostics: Vec<Diagnostic> = Vec::new();
            for n in r.list(fields[2], job.limits[2] as u32)? {
                let d = r.record(n, DIAGNOSTIC, 5)?;
                let values = d[..4]
                    .iter()
                    .map(|&n| {
                        r.field(n)
                            .and_then(|v| u32::try_from(v).map_err(|_| "diagnostic range".into()))
                    })
                    .collect::<Result<Vec<_>>>()?;
                let (code, module, start, end) = (values[0], values[1], values[2], values[3]);
                let source = &job
                    .modules
                    .get(module as usize)
                    .ok_or("diagnostic module")?
                    .source;
                if !(1..=8).contains(&code) || start > end || end as usize > source.len() {
                    return Err("diagnostic span/code".into());
                }
                let message = r.string(d[4], job.limits[5] as u32)?;
                let diagnostic = Diagnostic {
                    module,
                    start,
                    end,
                    code,
                    message,
                };
                if code == 8
                    && (start != 0
                        || end != 0
                        || job.modules[module as usize].path != job.entry
                        || diagnostics.iter().any(|d| d.code == 8))
                {
                    return Err("diagnostic limit marker".into());
                }
                if diagnostics.last().is_some_and(|d| d > &diagnostic) {
                    return Err("diagnostic order".into());
                }
                diagnostics.push(diagnostic);
            }
            if diagnostics.is_empty() {
                return Err("empty failure result".into());
            }
            Ok(ResultValue::Failure(diagnostics))
        }
        _ => Err("result status".into()),
    }
}
