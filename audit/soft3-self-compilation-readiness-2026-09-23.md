# Self-compilation on soft3 — 2026-09-23

This dated assessment records the starting evidence. Implementation now follows
the [SH0–SH8 milestone contract](../reference/self-hosting.md) and
[progress ledger](self-hosting-progress.md). The recommendations and estimate
below retain their assessment-time context; those two documents own the active
acceptance criteria and next task.

The requested target is **Trident → nox → Joy, with Zheng for execution
proofs**. This report supersedes the Triton-first recommendation and effort
estimate in [the earlier assessment](self-hosting-readiness-2026-09-23.md).
Its prototype probes remain valid evidence; its proposed target was wrong for
the user's goal.

## Exact milestone

Let `S` contain the compiler's Trident sources, complete library/module closure,
and its nox generator. Pin compiler options and all dependency identities.

```text
Rust seed compiler(S)       -> C1.nox
Joy / nox executes C1 on S  -> C2.nox
Joy / nox executes C2 on S  -> C3.nox
```

The run-only milestone requires complete executable outputs, canonical
`C2 == C3`, and language regression tests compiled by the resulting compiler.
All parsing, checking, lowering and compilation decisions in the second and
third builds happen inside nox. The host may load/save bytes, invoke the
runtime and compare artifacts. Joy and nox's native executor may remain Rust.

The next proof milestone requires Zheng to authenticate that same compilation
workload and its exact input/output. It must not silently substitute a
Triton-backed proof profile. A proof of compiler execution and a bootstrap
fixed point still do not establish preservation of source-language semantics.

The [soft3 execution model](../../soft3/specs/execution-model.md) allows pure,
stateless local execution without network, node deployment, Atlas or BBG state.
Those integrations are not prerequisites for this bootstrap.

## Existing foundation

- Rust Trident already lowers typed AST directly to nox formulas, bypassing
  foreign stack TIR. The implementation is
  `src/ir/tree/lower/nox.rs` and its sibling modules.
- Native nox supports runtime pair construction and application of a computed
  formula: `nox/rs/patterns/cons.rs:18` and `compose.rs:19`.
- Nox data is an immutable, hash-consed DAG. Its canonical encoding and Hemera
  identities already exist in `nox/rs/encode.rs`; reuse these contracts.
- Joy runs nox formulas, including behavior beyond the current proof profile.
- Zheng already binds public program/input/output/cost in its supported bounded
  execution certificate. The missing work concerns the broader compiler
  workload, not redoing that existing binding.
- The seven `.tri` compiler prototype modules provide lexer/parser/typechecker
  algorithms and tests. Their present RAM/TIR representation is not a native
  soft3 compiler architecture, and the earlier probes found semantic failures.

Nox reference lowering currently belongs to Trident; Joy owns runtime/proof
integration. This ownership is explicit in the soft3 execution model. There
is no requirement to copy Trisha's internal repository layout.

## Fresh release-binary probes

Used the same verified Trident 0.3.0 / Joy 0.5.0 release bundle as the earlier
assessment. [Source probe receipt](self-hosting-2026-09-23/soft3-source-probes.json)
and [runtime receipt](self-hosting-2026-09-23/soft3-runtime-probes.json).

| Probe | Observed result |
|---|---|
| Scalar `main(x) { x + 1 }`, input 6 | Compiles for nox; Joy returns 7 in 22 reductions |
| Entry argument `[Field; 59]` | Source check passes; nox build rejects the aggregate limit |
| Loop with runtime end and `bounded 4097` | Source check passes; build rejects unrolling beyond 4096 |
| Runtime index into a two-element array | Source check passes; nox build rejects nonconstant indexing |
| Computed nox formula, input 1 | Joy run returns 42 in 6 reductions |
| Prove that same computed formula | Rejected: `Unsupported("dynamic continuation")` |

The last formula is:

```text
[2 [[1 0] [3 [[0 2] [1 42]]]]]
```

With public input 1, it constructs `[1 42]` at runtime and applies it to zero.
These fresh observations distinguish source/backend limits from native VM
capability, and native execution from the production proof relation.

## Work remaining

### 1. Compiler data model and input/output contract

Define a deterministic compiler job taking a complete source package and
options, returning a formula plus diagnostics and metadata. Source, tokens,
AST, symbol tables and module maps need dynamically sized bounded structures.
The current Trident types have fixed-size arrays/tuples/structs and no native
source-level noun type (`src/ast/mod.rs:189`, `src/typecheck/types.rs:8`).

Recommended direction: typed wrappers over native nouns, packed/chunked bytes,
balanced sequences/maps and explicit persistent compiler state. This is an
architectural recommendation, not an implemented API. A compatible opaque
collection abstraction could satisfy the same requirement; exposing arbitrary
untyped dynamic evaluation to application source is unnecessary.

The seven existing modules contain hundreds of explicit `mem.read/write`
operations. Their algorithms can be reused, but their RAM access and fixed
scratch layouts require adaptation. Adding a Triton-style RAM machine to nox
is not necessary for a compiler expressed over native data.

### 2. Rust seed support for the native compiler subset

The first Rust compiler must be able to compile the new `.tri` implementation.
Extend its types/intrinsics/ABI and nox lowering together:

- Structured input/output beyond the current 58-word typed entry limit.
- Dynamic collection access/update with explicit bounds and ownership of data.
- Runtime bounded loops and reusable function code. Current calls inline;
  loops unroll with a 4096-iteration limit and a 2,000,000-node construction
  budget (`src/ir/tree/lower/nox.rs:144`). Existing prototype loops reach
  8193 in the lexer, 32768 in the parser and 131072 in the typechecker/codegen.
- Lower loops through native compose/continuations or a state machine rather
  than multiplying the formula by the iteration bound. Source-level arbitrary
  recursion is not required: bounded source loops can use recursive cores
  internally, with enforced budgets.
- Resolve actually needed integer operations and target ABI differences
  explicitly; the existing prototype also fails on nox's unsupported divmod.

Ordinary calls use deterministic composition. Nox pattern16 is nondeterministic
witness injection and must not become a hidden host compiler service.

### 3. Nox/Joy execution at compiler scale

Native runtime support for computed formulas does not yet guarantee usable
compiler-scale execution:

- Nox's evaluator recursively invokes `reduce_inner`, including tail-position
  compose, and has `MAX_DEPTH=1000` (`nox/rs/reduce.rs:64`,
  `nox/rs/patterns/compose.rs:31`). Just emitting recursive loops can hit this
  limit. Use an iterative continuation mechanism, or a deliberately bounded
  chunked evaluation strategy, preserving trace/cost semantics.
- Joy uses a fixed `Reduction<1<<18>` arena (`joy/rs/warrior.rs:24`). Allocation
  stops at a 3/4 load factor: **196,608 distinct nodes**, not 262,144
  (`nox/rs/data/reduction.rs:56`). Allocation is append-only within a reduction.
  Source, compiler formula, AST, environments, output and temporary nodes need
  a measured memory strategy. Raising the reduction budget does not enlarge it.
- Run-only Joy currently collects a full `VecTrace`
  (`joy/rs/warrior.rs:165,212`). Nox already has `NoTrace`; expose an appropriate
  counted run mode. Proof tracing/chunking is a separate contract.
- Current Joy input is a flat word list and output is flattened atom leaves
  (`joy/rs/formula.rs:105,120`). A compiler artifact must preserve tree topology:
  `[[1 2] 3]` and `[1 [2 3]]` cannot become the same output. Transport a complete
  canonical noun artifact or an unambiguous serialized program and exact length.
  Existing node encodings still need a deterministic complete-package envelope.

Measure peak nodes, memory, reductions and time on compiler-sized source before
claiming this gate closed. No compiler-sized runtime test was attempted today.

### 4. Native compiler implementation on Trident

Repair the observed frontend errors in variables, parameters and multiple
functions. Adapt lexer/parser/typechecker data access to the native model.
Add deterministic module resolution/linking and complete the subset needed by
the compiler itself. Differential tests must include independent expected
results and negative inputs, not just agreement of two implementations.

Implement **typed AST -> nox formula generation in `.tri`**. The existing Rust
backend is a reference implementation; no equivalent `.tri` NounBuilder was
found. `lib/std/compiler/codegen.tri` and `lower.tri` currently target stack
TIR/TASM. Wiring that last stage does not implement this nox generator.
The native pipeline should preserve the existing direct AST-to-nox architecture.

### 5. Bootstrap, artifacts and release gate

Build C1/C2/C3 using a pinned source/dependency closure. Preserve canonical
formula structure, source/options identities, failures and diagnostics.
Compare C2/C3, run the language corpus with the self-built compiler, record
resource limits and add the reproducible bootstrap to CI. Unsupported syntax
and resource exhaustion must fail explicitly without publishing partial output.

## Zheng proof milestone

Today's public native profile is an execution certificate with full witness
disclosure and linear verification. It has no Triton dependency. The private
`JOYZK003` path still directly uses Trisha/Triton7
(`joy/rs/zk_execution.rs:7,96,217`). It cannot count as an independently native
soft3 private compiler proof.

Production limits include 64 input atoms, 4096 program nodes, depth128,
4096 symbolic calls, 32768 gates/rows and 4096 output atoms. The relation
requires static continuations and compatible branch shapes. Experimental
tagged-noun work is present, but has no production proof dispatch and does not
close computed-formula execution.

Proving the proposed compiler requires a relation covering its actual runtime
control flow and data representation, with bounded/chunked execution if needed.
Bind compiler identity, complete source/module/options commitments, the exact
output artifact and execution cost to that relation. Increasing existing limits
alone cannot implement dynamic execution semantics.

A valid proof establishes this compiler's execution. Semantic preservation of
the source language needs its own validation/formal argument.

## Revised sequencing and effort

| Work package | Rough three-hour sessions |
|---|---:|
| Native data/job ABI and minimal compiler subset contract | 4–7 |
| Rust seed types, collection operations, nox calls/loops/entry | 8–14 |
| Nox/Joy execution scale and artifact transport | 5–10 |
| Adapt/repair frontend and implement `.tri` nox generator | 15–25 |
| Full source closure, bootstrap and regression/CI hardening | 8–14 |

**Planning envelope: 40–70 focused sessions (120–210 hours) for native run-only
self-compilation.** This is a low-confidence engineering estimate, not a
measured remaining-task total or a promise. Some packages can overlap; data
representation and evaluator changes can expand the work. Re-estimate after
the first native compiler slice and a compiler-sized scan/allocation benchmark.
The earlier 20–40-session Triton estimate does not estimate this target.

The first acceptance slice is a compiler written in Trident, executed by Joy
on nox, consuming supplied source and producing a small nox formula which Joy
then executes correctly. Complete that alongside realistic data/loop/arena
probes before investing in the full port.

Zheng proof support for the complete compiler workload is an additional
architecture and proof-system package. A credible effort bound requires an
accepted relation and measured workload; this review supplies neither. Keep
that gate visible rather than folding it into the native-run estimate.
