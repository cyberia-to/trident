# Native compiler proof gap

Read-only assessment for the next SH7 increment. SH6 native reproduction is
still running; this assessment supplies no additional acceptance. The local
S1 self-builds and corpus results remain in [the ledger](../self-hosting-progress.md).
The governing gates are [SH7 and SH8](../../reference/self-hosting.md#sh7-native-proof-relation).

Inspected inputs are the exact current bootstrap pins: Trident `c17bd037`,
Joy `ec83bd8d`, nox `f8047c22`, Zheng `b54b209b`. The
[source receipt](native-proof-gap-sources.json) records their full revisions,
file hashes and actual `git show REV:PATH` commands. No compiler execution,
proof generation, performance measurement or production code change was
performed for this assessment.

## Observed boundaries

| Boundary | Implemented behavior | Required extension |
|---|---|---|
| Joy dispatch | The public execution proof entry accepts `ProgramBundle` and scalar `ProgramInput`; structured ART1/JOB1 uses `run-artifact` | A versioned structured proof entry and verifier binding complete program, subject and result nouns |
| Computed control | Production Zheng derives an unfolded relation from the public formula; `static_noun()` rejects a computed continuation | A checked transition in which the computed noun becomes the next formula |
| Value shape | Production `mux()` rejects branches with different output shapes | Authenticated atom/pair tags, child order and variable result topology |
| Statement bounds | `execution/statement.rs` admits at most 4096 program prefix nodes, depth 128, 64 scalar inputs and 4096 output atoms | A bounded canonical DAG contract suitable for the compiler input and output; increasing these constants alone does not implement dynamic execution |
| Compaction | The compacting evaluator explicitly uses `NoTrace`; collection relocates physical `u32` node references and live continuations | A bounded semantic transition observer with identities that survive relocation |
| Finalizer cache | A cache hit returns the cached result without invoking the finalizer closure | A checked semantic result for a hit, including its operands and cost behavior |
| Compiler protocol | Joy validates compiler identity, JOB1 package/options and RES1 job/output bindings | Those identities must be constrained against the relation's actual inputs and result |

Source locations, at the revisions in the receipt:

- Joy: `cli/main.rs`, `rs/execution.rs::Warrior::prove_execution`,
  `rs/structured/job.rs`, `rs/structured/job_result.rs`.
- nox: `rs/sequential/dispatch.rs`, `compacting.rs::step`,
  `compacting/collection.rs`, `finalizer_cache.rs::Cache::finish`.
- Zheng: `rs/src/execution/relation.rs::{static_noun,mux}` and
  `rs/src/execution/statement.rs`.

Replacing `NoTrace` with `VecTrace` would not itself supply the missing
continuation, cache-hit and relocation bindings. Existing postorder rows do
not expose the complete live evaluator state needed by this proposed profile.

## Existing proof components

Joy already dispatches bounded static private execution through `JOYZH001`
(`cli/prove.rs`, `rs/zk_execution.rs`). Zheng
`execution/private.rs` derives that relation through
`ExecutionStatement::relation`; it shares the computed-continuation boundary
above. The new dynamic compiler profile needs its own disclosure and proof
contract while preserving this existing private path.

Zheng's `execution/proof.rs` provides a public full-witness backend. Its
verifier authenticates the complete witness, checks the constant and public
coordinates, and checks satisfaction of every relation row. The execution
layer must independently derive the relation. Prover-selected matrices would
not establish the intended execution claim.

The experimental `execution/tagged/` kernel already constrains atom/pair tags,
padding, structural Hemera hashes and output payload. It still rejects
computed continuations and has no production Joy dispatch. Its public
coordinates also need an explicit adapter: the tagged kernel includes the
constant coordinate and does not return a strictly sorted list, whereas the
direct proof API fixes the constant itself and requires increasing nonzero
indices. Conflicting coordinates must reject before normalization.

The older universal trace relation is another distinct contract.
`ccs/universal.rs` has no in-row compose/cons constraints;
`specs/constraints.md` explicitly marks their cross-row/particle wiring as
pending. Joy's `rs/proof.rs::require_statement_only` refuses an emitted-value
claim for that legacy format. Its trace acceptance cannot replace the native
compiler execution relation.

## Proposed first complete increment

These are design proposals requiring implementation and independent review.
They are not established soundness results or additional closed gates.

1. Specify a versioned semantic state and add bounded witness capture in nox.
   Include Enter/Return state, subject/formula identity, continuation phase and
   stack, budget reservations/refunds and terminal status. Check computed
   formulas, selected branches, operand reads and cache-hit results.

   A candidate memory design uses complete four-limb Hemera identities for
   immutable nouns, making physical relocation transparent to logical state.
   It still needs constrained atom/pair construction, authenticated reads,
   ordered child links and preservation of all live continuation roots.
   Stored host hashes alone do not provide those guarantees.

2. Implement a bounded transition relation in Zheng using the existing public
   full-witness backend first. Fix limits on steps, nodes, proof bytes and
   verifier work. Derive constraints independently of an unverified execution
   path. Bind each chunk's job/profile, position, input/output state and
   accumulated semantic cost; require one final completed state with an empty
   continuation stack.

   Adversarial proofs must reject a substituted continuation or branch,
   reordered children with identical leaves, an unauthenticated read, wrong
   tag/payload, stale relocated reference, omitted cache-hit transition,
   incorrect cost/refund, and dropped/reordered/repeated/foreign chunks.
   A valid unfinished prefix must not verify as completed execution. Tests
   must construct invalid proofs independently of the honest prover.

3. Wire structured ART1/JOB1 prove/verify through Joy and complete a small
   production pilot. Extend a separate fixture based on
   `tests/fixtures/native_generated_literal_compiler.tri`: it reads actual
   source bytes and emits an ART1 program. Add a continuation computed from
   that input, plus branches returning success/program or compile-error/DIA1.
   Prove both outcomes and retain the original computed-apply regression as
   an independent guard.

   Verify in a fresh process against explicit compiler/JOB/result artifacts,
   without running the compiler again. Mutations of compiler, source,
   dependency, option, any job-identity limb, output topology/payload,
   extracted ART1, cost or profile must reject. Failed commands must preserve
   an existing output file. Next, exercise the same production path using the
   frozen whole compiler on a small source package; the fixture cannot replace
   that acceptance or the complete self-build proofs required by SH8.

## Claim and resource boundary

The profile must prove program/input/output identities and topology, semantic
transitions, selected continuations and branches, semantic reductions/budget,
logical memory operations, chunk continuity and final completion.

Frame bounds, allocation limits and GC policy are proved only when the
relation constrains the corresponding states and events. The proposed first
logical-memory profile treats elapsed time, RSS/resident storage, physical
allocation, collection count/work and cache statistics as host telemetry.
It therefore cannot claim cryptographic enforcement of every physical host
or LIM1 limit. The normative profile must settle the distinction before its
implementation is accepted.

The public backend authenticates the full disclosed witness and checks every
relation row. Work also depends on the relation matrices and multisets; its
suitability for whole-compiler scale remains unmeasured. Succinctness and zero
knowledge for this proposed dynamic compiler profile need separate
implementation and evidence. Proof of a chosen compiler's execution also
leaves source-language semantic preservation as a separate verification problem.
