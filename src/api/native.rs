//! Explicit native profiles; complete ART1 trees never pass through text.
use super::{pipeline::PreparedProject, require_nox_target, CompileOptions};
use crate::ast::FileKind;
use crate::diagnostic::Diagnostic;
use crate::ir::tree::lower::{nox::NoxCompiler, Noun};
use crate::span::Span;
use nebu::Goldilocks;
use nox::{artifact, Order, Reduction};
use std::path::Path;

const ARENA: usize = 1 << 18;
const STACK_BYTES: usize = 256 * 1024 * 1024;
const MAX_FORMULA_VISITS: usize = 2_000_000;
const MODULUS: u64 = 0xffff_ffff_0000_0001;
const ART1: u64 = 0x4152_5431;

/// Initial seed emission ceilings, matching Joy's raw-artifact transport.
pub const NATIVE_ARTIFACT_LIMITS: artifact::Limits = artifact::Limits {
    max_bytes: 16 * 1024 * 1024,
    max_nodes: (ARENA / 4 * 3) as u32,
    max_depth: 4096,
};

/// Compatibility name for the native seed emission ceilings.
pub const RAW_ARTIFACT_LIMITS: artifact::Limits = NATIVE_ARTIFACT_LIMITS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeArtifactProfile {
    RawNoun,
    CompilerJob,
}

impl NativeArtifactProfile {
    pub const fn value(self) -> u64 {
        match self {
            Self::RawNoun => 0,
            Self::CompilerJob => 1,
        }
    }
}

#[derive(Debug)]
pub struct NativeArtifact {
    /// Complete canonical NOXDAG01 container of ART1(0, profile, profile, formula).
    pub bytes: Vec<u8>,
    /// Full native identity of that ART1, independent of source metadata.
    pub particle: [u8; 32],
    pub name: String,
    pub profile: NativeArtifactProfile,
}

pub type RawArtifact = NativeArtifact;

/// Compile a resolved source package with exactly `fn main(input: Noun) -> Noun`.
/// The profile is pure: host witness and persistent-state services are forbidden.
/// Limits bound emission, not source parsing or host compilation time.
pub fn compile_raw_artifact_project(
    entry: &Path,
    options: &CompileOptions,
    limits: artifact::Limits,
) -> Result<RawArtifact, Vec<Diagnostic>> {
    compile_native_artifact_project(entry, options, NativeArtifactProfile::RawNoun, limits)
}

/// Compile an explicit structured entry profile. CompilerJob declares the
/// JOB1/RES1 boundary; the warrior must validate those records at execution.
pub fn compile_native_artifact_project(
    entry: &Path,
    options: &CompileOptions,
    profile: NativeArtifactProfile,
    limits: artifact::Limits,
) -> Result<NativeArtifact, Vec<Diagnostic>> {
    worker(|| {
        require_nox_target(options)?;
        validate_limits(limits)?;
        let project = PreparedProject::build_quiet(entry, options)?;
        let entry = project
            .modules
            .iter()
            .find(|m| m.file.kind == FileKind::Program)
            .or_else(|| project.modules.last())
            .ok_or_else(|| error("no entry module"))?;
        let files: Vec<_> = project.modules.iter().map(|m| &m.file).collect();
        let formula = NoxCompiler::new()
            .compile_raw_modules_with_origins(
                &files,
                &entry.file,
                &options.cfg_flags,
                &project.native_origins,
            )
            .map_err(error)?;
        emit(formula, entry.file.name.node.clone(), profile, limits)
    })
}

/// Single-file counterpart; imports require `compile_raw_artifact_project`.
pub fn compile_raw_artifact(
    source: &str,
    filename: &str,
    options: &CompileOptions,
    limits: artifact::Limits,
) -> Result<RawArtifact, Vec<Diagnostic>> {
    compile_native_artifact(
        source,
        filename,
        options,
        NativeArtifactProfile::RawNoun,
        limits,
    )
}

/// Single-file counterpart; imports require compile_native_artifact_project.
pub fn compile_native_artifact(
    source: &str,
    filename: &str,
    options: &CompileOptions,
    profile: NativeArtifactProfile,
    limits: artifact::Limits,
) -> Result<NativeArtifact, Vec<Diagnostic>> {
    worker(|| {
        require_nox_target(options)?;
        validate_limits(limits)?;
        let file = crate::parse_source_silent(source, filename)?;
        let (file, exports, origins) =
            PreparedProject::source_with_origins(file, source, filename, options)?;
        exports.check_entry_requirements(&file, options)?;
        let formula = NoxCompiler::new()
            .compile_raw_modules_with_origins(&[&file], &file, &options.cfg_flags, &origins)
            .map_err(error)?;
        emit(formula, file.name.node, profile, limits)
    })
}

fn validate_limits(limits: artifact::Limits) -> Result<(), Vec<Diagnostic>> {
    if limits.max_bytes == 0
        || limits.max_bytes > RAW_ARTIFACT_LIMITS.max_bytes
        || limits.max_nodes == 0
        || limits.max_nodes > RAW_ARTIFACT_LIMITS.max_nodes
        || limits.max_depth == 0
        || limits.max_depth > RAW_ARTIFACT_LIMITS.max_depth
    {
        return Err(error(
            "raw artifact limits must be positive and within seed emission ceilings",
        ));
    }
    Ok(())
}

fn worker<T: Send>(
    job: impl FnOnce() -> Result<T, Vec<Diagnostic>> + Send,
) -> Result<T, Vec<Diagnostic>> {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("trident-artifact".into())
            .stack_size(STACK_BYTES)
            .spawn_scoped(scope, job)
            .map_err(|e| error(format!("artifact worker: {e}")))?
            .join()
            .map_err(|_| error("artifact compiler worker panicked"))?
    })
}

fn emit(
    formula: Noun,
    name: String,
    profile: NativeArtifactProfile,
    limits: artifact::Limits,
) -> Result<NativeArtifact, Vec<Diagnostic>> {
    let mut arena = Reduction::<ARENA>::new();
    arena.limit_allocations(limits.max_nodes);
    // Bounded iterative postorder avoids a bracket-text round trip and does not
    // conflate the Rust tree's addresses with portable native identity.
    let mut pending = vec![Some(&formula)];
    let mut values: Vec<Order> = Vec::new();
    let mut visits = 0;
    while let Some(next) = pending.pop() {
        visits += 1;
        if visits > MAX_FORMULA_VISITS {
            return Err(error("raw formula visit limit exceeded"));
        }
        let node = match next {
            Some(Noun::Atom(v)) => {
                if *v >= MODULUS {
                    return Err(error("noncanonical formula atom"));
                }
                arena.atom(Goldilocks::new(*v))
            }
            Some(Noun::Cell(left, right)) => {
                pending.extend([None, Some(right.as_ref()), Some(left.as_ref())]);
                continue;
            }
            None => {
                let right = values
                    .pop()
                    .ok_or_else(|| error("formula traversal underflow"))?;
                let left = values
                    .pop()
                    .ok_or_else(|| error("formula traversal underflow"))?;
                arena.pair(left, right)
            }
        }
        .ok_or_else(|| error("raw artifact allocation limit exceeded"))?;
        values.push(node);
    }
    let formula = values.pop().ok_or_else(|| error("empty formula"))?;
    let zero = arena
        .atom(Goldilocks::ZERO)
        .ok_or_else(|| error("raw artifact allocation limit exceeded"))?;
    let tag = arena
        .atom(Goldilocks::new(ART1))
        .ok_or_else(|| error("raw artifact allocation limit exceeded"))?;
    let mut root = arena
        .pair(formula, zero)
        .ok_or_else(|| error("raw artifact allocation limit exceeded"))?;
    let profile_node = arena
        .atom(Goldilocks::new(profile.value()))
        .ok_or_else(|| error("native artifact allocation limit exceeded"))?;
    for head in [profile_node, profile_node, zero, tag] {
        root = arena
            .pair(head, root)
            .ok_or_else(|| error("raw artifact allocation limit exceeded"))?;
    }
    let bytes = artifact::encode(&arena, root, limits)
        .map_err(|e| error(format!("raw artifact encoding: {e:?}")))?;
    let particle = nox::data::digest_bytes(
        arena
            .digest(root)
            .ok_or_else(|| error("artifact identity missing"))?,
    );
    Ok(NativeArtifact {
        bytes,
        particle,
        name,
        profile,
    })
}

fn error(message: impl Into<String>) -> Vec<Diagnostic> {
    vec![Diagnostic::error(message.into(), Span::dummy())]
}
