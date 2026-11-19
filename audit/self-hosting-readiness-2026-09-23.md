# Self-hosting readiness — 2026-09-23

**Target correction:** the user's target is native soft3 self-compilation.
The Triton-first recommendation and estimate below are superseded by
[the soft3 assessment](soft3-self-compilation-readiness-2026-09-23.md).
The recorded prototype probes remain valid.

Active implementation follows [self-hosting on soft3](../reference/self-hosting.md)
and its [progress ledger](self-hosting-progress.md).

Assessment requested against today's implementation. The released compiler is
written in Rust. A substantial compiler prototype exists in Trident, but it
cannot yet compile ordinary multi-function programs reliably or emit an
executable through its pipeline. Self-compilation and a bootstrap fixed point
have not been demonstrated.

## Scope and evidence

- Fetched Trident `origin/master`: `360b737e073ca2f969ab0c78460b4228bcac7b78`.
- The inspected `lib/std/compiler/` is identical in the working tree, that
  master, and release commit `531e93c4a08bd715a8e8ec61d2089ba7768c6d39`.
  Its most recent implementation commit is `9babdf8` (2026-09-16).
- Fresh probes used the published macOS ARM64 bundle: Trident 0.3.0,
  Trisha 0.3.0, Joy 0.5.0. Archive SHA-256:
  `dca19d65224f9e99a6ac3d1f8d84961f4361f300246380903fb3b37718d314ae`,
  checked against the release's published SHA256SUMS. Older installed binaries
  were not used. Standard-library imports in the release compiler are embedded.
- Seven compiler modules contain 9,237 lines / 320,140 source bytes. This is an
  inventory, not a completion percentage.
- Today's probes are recorded in [the JSON receipt](self-hosting-2026-09-23/probes.json).
  No compiler implementation was changed by this assessment.

The [published release receipt](release-2026-09-16.md) remains the evidence for
the coordinated CPU release; those complete proof and cross-platform suites
were not repeated for this assessment.

## Fresh probe results

All seven `.tri` compiler modules pass `trident check --target triton` using the
Rust compiler. An instrumented wrapper around `std.compiler.pipeline.compile`
also builds and executes on Triton. Neither result establishes self-compilation.

The wrapper reports `[TIR record count, compiler error count, records...]`.
Each record contains four field words. The wrapper deliberately reports errors
instead of asserting zero errors, so VM exit 0 alone is not compilation success.

| Input to the compiler written in Trident | Rust check | Prototype result |
|---|---|---|
| `2 + 3 * 4` as a Field function body | Pass | 9 TIR records, 0 errors |
| `return 7` | Pass | 5 TIR records, 0 errors |
| `let x: Field = 7 x` | Pass | 5 TIR records, **1 error**, a zero literal in output |
| Helper function returning 7, followed by `main` returning 9 | Pass | **0 errors**, but output contains only one function with literal **7**; no main body with 9 |
| Two-parameter `add`, called by `main` | Pass | 9 TIR records, **2 errors** |

These are intermediate-representation observations. The prototype's emitted
program was not executed: its pipeline does not yet emit executable TASM.
The arithmetic and explicit-return probes demonstrate narrow working paths,
not general expression or control-flow completeness.

An initial one-line import probe failed Rust checking too, and is excluded
from the comparison; it is not evidence that a valid import regressed.

Building the same pipeline wrapper for nox fails with unsupported
`@operator:divmod`, `pub_read`, `pub_write`, `ram_read`, and `ram_write`.
This matches the [documented nox surface](../reference/nox.md).

## Gaps that block self-hosting

1. **Source semantics and AST connectivity.** Multiple items, parameters and
   interleaved statements need repair. This is also explicitly recorded in
   `trisha/baselines/triton/std/compiler/README.md`. The silent wrong output
   for two functions is more serious than a missing optimization.
2. **Complete language subset used by the compiler.** Names, scopes, calls,
   imports, types, control flow, memory operations and indexing need a
   differential corpus against Rust and independent expected outputs.
   `codegen.tri` currently handles runtime array indexing through a branch
   commented "For simplicity, treat as constant index 0". Unsupported
   constructs must reject explicitly instead of emitting plausible wrong code.
3. **Executable output.** `pipeline.tri` stops after optimized numeric TIR;
   stage 6 is explicitly unwired. `lower.tri` contains an emitter prototype,
   but a complete producer/consumer contract, including symbols, labels,
   calls, legalization and unknown-op rejection, must be established.
4. **Compiler-sized projects and memory.** The current baseline contract
   handles at most 1,024 input bytes. Pipeline scratch regions and work queues
   have fixed offsets and limits. Handling the compiler's roughly 320 KB plus
   its dependency closure requires module loading/linking and explicit capacity,
   termination and overflow contracts. This review did not demonstrate an
   overflow; it identifies capacity as an unclosed gate.
5. **Bootstrap and continuous verification.** No complete own-source build,
   repeated self-build comparison, or CI gate for that fixed point was found.

## Historical Triton alternative — superseded for the requested target

The shortest path uses **Triton / Trisha**. Keep shared language semantics and
portable IR in Trident; keep Triton emission, stack legalization and execution
in Trisha. Baselines stay in Trisha. Do not restore target-specific ownership
to Trident merely to wire the legacy `lower.tri` prototype.

For an honest self-hosted compiler, every semantic compilation stage after the
seed build must be implemented in Trident, including the selected target's
emitter (which can live in Trisha). A host driver may transport source bytes,
invoke the VM and write artifacts; it must not quietly call Rust's parser,
typechecker or code generator. Trisha's VM/prover need not themselves be
rewritten to claim compiler self-hosting.

| Gate | Deliverable and acceptance | Rough focused sessions |
|---|---|---|
| A: executable vertical slice | `.tri` compiler produces a small executable; run its output, compare observable results, reject unsupported IR | 3–5 |
| B: compiler language subset | Correct items/scopes/calls/control flow/types/memory; positive and negative differential coverage | 8–15 |
| C: complete compiler project | Resolve its own module closure; deterministic linking, explicit buffer bounds and usable input/output driver | 5–10 |
| D: repeatable self-build | Compile own sources, obtain a fixed point, run regression corpus with the resulting compiler, gate in CI | 4–8 |

Planning order of magnitude: **20–40 focused three-hour sessions (60–120 hours)**
for the first minimal self-host on one target. This is an engineering estimate
with low-to-medium confidence, not measured remaining work or a delivery promise.
Re-estimate after gate A exposes the actual TIR/emitter repair cost. Deep AST
or memory redesign could increase it. Full Rust CLI/LSP parity, all targets and
formal semantic correctness are outside this estimate.

Let `S` include the compiler, its libraries and the selected `.tri` emitter:

```text
B1 = Rust compiler(S)
B2 = execute B1 on S
B3 = execute B2 on S
accept reproducible bootstrap only when B2 == B3
```

Compare deterministic executable artifacts and retain options, source hashes,
library versions and resource measurements. B1 and B2 may legitimately differ
because Rust and Trident implementations optimize differently. Comparing the
second and third build follows the conventional
[three-stage bootstrap](https://gcc.gnu.org/install/build.html).

Nox self-hosting is a separate later gate: the current RAM-oriented compiler
does not fit its statically lowered, bounded source surface. It needs either
a compiler designed around nox's noun/state model or an explicit extension of
the execution model. Proving that workload with Zheng adds proof-relation and
resource requirements beyond native execution.

## Roadmap overview and corrections needed

The canonical roadmap mixes vision, historical check marks and implementation
status. Its Kelvin table is not an auditable percentage of work remaining.

| Track | Grounded status | Next meaningful milestone |
|---|---|---|
| CPU toolchain and distribution | Coordinated 0.3/0.3/0.5 release published for six Mac/Linux/Windows native targets | Maintain release regression and compatibility gates |
| Self-hosting | Seven prototype modules pass Rust checking; the pipeline runs on Triton, with incomplete semantics and final emission | Gates A–D above |
| Joy / nox / Zheng | Bounded public/private execution and authenticated public-state paths released | Broader execution relations, dynamic behavior and live integration, each with negative proof tests |
| Formal assurance | Scalar Field/U32/Bool contracts, branches and bounded acyclic helper analysis implemented | Aggregates, loops and further semantics; unsupported analysis remains UNKNOWN |
| Atlas / packages | Packaging/deployment machinery exists; this audit establishes no live on-chain registry | Independently verify publish, discovery, installation and execution on the intended network |
| Trinity | Arithmetic components/demos exist | Independently close inference, secure FHE and quantum-simulation contracts; do not infer security from arithmetic proof success |
| New warriors | Vitalina, gaw, tolya, pearla and zenda were scaffolded as stubs | Implement and validate one actual target adapter before advertising capability |
| Neural optimization / GPU | Separate experimental and performance work | Verified corpus wins and measured proving improvement on explicit supported hardware |

Specific roadmap issues observed:

- Its introduction still calls the released versions a source candidate.
- `64K` marks a `.tri` NounBuilder complete; no corresponding implementation
  was found. The implemented NounBuilder is in Rust.
- Some compiler entries correctly say prototype, while `32K` still describes
  self-compilation as a release property.
- Part V equates a STARK execution proof with compiler semantic correctness.
  A buggy compiler can execute faithfully and receive a valid proof. A fixed
  point establishes reproducibility/self-consistency, not semantic preservation
  or elimination of trust in the bootstrap seed.

Three distinct claims should have separate gates:

1. **Self-hosting:** the compiler written in Trident builds itself.
2. **Proven compilation execution:** a pinned compiler ran on committed input
   and produced committed output under a specified proof relation.
3. **Correct compilation:** the output preserves the source language's meaning,
   justified by verified transformations or translation validation against
   explicit semantics.

The requested next milestone follows the native soft3 assessment linked above.
Atlas, new warriors, GPU work and full Trinity are independent tracks and need
not block the first self-build. The 0.3 CPU release and this future milestone
have different acceptance criteria.

## Reproduction

Use the verified released `trident` and `trisha` binaries on PATH. The probe
wrapper is derived from
`trisha/baselines/triton/fixtures/compiler-pipeline-precedence/main.tri` by
removing `assert(mem.read(2011) == 0)` and replacing `pub_write(0)` with
`pub_write(mem.read(2011))`. Compile it with:

```sh
trident build pipeline-probe.tri --target triton -o pipeline-probe.tasm
```

For each exact `source` in the JSON receipt, pass comma-separated decimal
`[len(source.encode()), *source.encode()]` to:

```sh
trisha run --tasm pipeline-probe.tasm --input-values "$probe_values"
```

Read the first two output words as count and errors and group the remaining
words into four-word TIR records. Check each source independently with
`trident check <source-file> --target triton`.
