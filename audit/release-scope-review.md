# Independent release scope review

Reviewed current code, `reference/roadmap.md`, `reference/language.md`, the active
release ledger, Joy capability JSON/CLI spec, and Zheng execution/backend specs.
Read-only review; no heavy jobs or independent repetition of the parent's new
198 baseline proofs, installed Mac smoke or isolated real-node admission.
Those completed receipts supersede older pending statements, but do not prove
unrelated roadmap functionality. No reduction of the user's full scope is
proposed here.

## Immediate discrepancies and bounded implementation work

1. **Canonical state capability description is wrong.**
   `reference/language.md:401` still says Joy rejects state execution. Actual
   `joy/targets/nox/capabilities.json`, `joy/rs/state_execution.rs`,
   `joy/rs/zk_state.rs`, and `zheng/specs/ccs-execution-backends.md` implement
   authenticated public state and bounded hidden queries over complete public
   tables. Replace the stale sentence with this exact distinction; do not imply
   live synchronization or encrypted databases. Acceptance: the description
   matches the JOYST001 and JOYZK003 installed state smoke and their wrong-root/
   wrong-result rejection checks. Reported to the documentation owner.

2. **Nox lacks a normal bounded language path already supported on Triton.**
   `src/ir/tree/lower/nox.rs:548` rejects every return inside a for-loop, including
   a fixed small loop. The existing test near1996 expects this rejection. This
   is a concrete completeness task, not a novel proof protocol. Implement
   continuation-aware early exit for bounded unrolling while preserving subject
   layout, caller return values, and skipped effects. Acceptance: scalar and
   aggregate returns, nested if/loops, imported helpers, no subsequent state/
   secret reads or failed assertions after return, and default/release profiles.
   Run source on native nox and prove/verify supported fixed-shape cases through
   Joy; compare independent expected results with Triton. Do not merely delete
   the rejection or weaken the existing test.

3. **Formal audit remains intentionally incomplete on ordinary library code.**
   `src/verify/sym/coverage.rs` rejects branches, aggregates, loops and user calls;
   `reference/formal-audit.md` honestly describes a scalar straight-line subset.
   This is fail-closed behavior, not false SAFE. The full audit requirement is
   nevertheless unclosed: actual inventory has1321 UNKNOWN function results and
   three guard counterexamples without adequate input contracts. Next bounded
   implementation: path-sensitive scalar if/return with condition assumptions
   and disjoint SSA scopes, then pure acyclic helper contracts. Acceptance:
   branch-local names never alias, false postconditions produce SAT, impossible
   branches do not create obligations, missing summaries remain UNKNOWN, and
   Z3 unavailable/SAT/UNKNOWN preserve non-success CLI/API results. Add genuine
   caller requirements to guards only when justified by their public contract;
   never insert assumptions merely to erase counterexamples. Aggregate/RAM and
   loop invariants are further required work for whole-library formal claims.

4. **General nox execution proofs still cover a restricted relation.**
   `zheng/rs/src/execution/relation.rs:199,260` rejects differing output tree
   shapes and dynamic continuations; `zheng/specs/execution.md:49` and Joy's
   capability restrictions agree. Native execution success is not a promise of
   proof support for these inputs. This is still an explicit original release
   ledger objective, not completed by the static CCS checks. Next implementable
   step is an agreed bounded tagged-noun shape representation with canonical
   inactive storage, constrained selection and fixed public output encoding.
   Acceptance: both branch shapes against native nox; corrupt tags/padding/
   selected payload rejected by the CCS itself; unselected invalid arithmetic
   remains inactive; exact budget and public output mutations reject. Dynamic
   formula evaluation needs a separate bounded transition relation authenticated
   against the program, not witness-selected CCS matrices. Preserve current
   fail-closed behavior until those relations are specified and tested.

## Promised larger features: not closed by arithmetic fixtures

- Roadmap claims of completed self-hosted stages conflict with
  `lib/std/compiler/pipeline.tri:10–14`: stage6 remains unwired and compile emits
  prototype optimized TIR. The existing fixture proves one small compiler
  pipeline, not self-compilation. Concrete next milestone: full supported TIR
  record decoding plus Trisha-owned emission, actual emitted program execution,
  reject unsupported records instead of dropping them, and differential corpus
  equivalence with Rust. Full self-compilation then requires compiling the
  compiler sources and a second-stage bootstrap comparison. Keep that objective
  open; editing the label alone does not implement it.
- Trinity's full29argument demo and independent proofs establish its arithmetic.
  `reference/trinity-arithmetic.md:25–32` and
  `lib/std/fhe/pbs.tri:189,264` expose missing canonical quotient, rotation,
  source/sign and key-switch bindings. Secure private inference/FHE remains a
  substantive protocol implementation obligation. A next bounded correctness
  milestone can constrain rotation/key-switch arithmetic and commit actual
  key/weight arrays, with independently generated mutation tests for every
  witness class. That alone still needs reviewed parameters/noise/encoding and
  upstream interoperability before a secure TFHE/FHE claim. No ad-hoc witness
  masking or final-output equality can replace these missing equations.
- Hemera's external cryptographic assurance is a real security gate that cannot
  be closed by passing execution tests or renaming it unsupported. Replacing a
  hash would require explicit full-chain format/root/transcript migration and
  reproof. An independent review is distinct from the implemented Poseidon2
  interoperability tests; this local bounded task cannot manufacture it.
- Self-verifying CORE, encrypted databases, broad GPU proving, multi-target
  formal compiler correctness, large AI systems and future hardware are roadmap
  objectives. They are not regressions in the documented CPU bounded runtime,
  nor can current benchmark success close them. The roadmap's historical check
  marks and large milestone headings must not become a release assertion that
  those implementations exist.

## Gates now requiring evidence reconciliation, not duplicate implementation

The old remaining-gates text still groups recursive verification, Neptune
validation, versions, baselines and installed artifacts as pending. Parent has
new receipts for the198 baseline proofs, genuine recursive integration, actual
isolated Neptune admission and full Mac installed smoke. Update the ledger to
those exact receipts, leaving wallet coin selection/finality and other genuinely
unimplemented behavior separate. Generic runtime deploy=false is consistent
with the trait lacking transaction intent; the explicit canonical adapter is a
different implemented interface, not proof that all deploy variants work.

Linux installed smoke is still owned by the other agent; do not infer its result
from Darwin or cross-compilation. Final coordinated source/binary receipts must
bind the last source changes and tested hashes, followed by the authorized
publication workflow. Registry sibling-resource/root-patch blockers remain real
for registry publication; the implemented complete source/binary archive route
is a separate distribution mechanism, not registry validation.

No new proof-forgery or supported-path acceptance bypass was established by this
bounded review. The highest-yield next code repair is bounded nox early return;
formal branch/contracts and general execution relations are distinct next gates.

## Follow-up implementation checkpoint

The owner authorized and the next work implements item2 via bounded return-aware
continuations (`src/ir/tree/lower/nox/loops.rs`) and correct outer-scope restoration
for all loops. The original installed v7 compiler rejects a valid typed example
(`/tmp/nox-loop-original-installed-rejection.log`). Current native nox surface
suite passes38tests in both relevant source profiles; existing nox lowerer
unit suite passes59tests. Eight installed Triton runs match the new nox expected
results. Fixtures cover zero/exclusive bounds, nested loops, aggregate/imported
helper returns, skipped secret/state effects and fallthrough shadowing/cost.
A further independently reported aggregate-shadow defect was reproduced with
actual999 instead of11 (`/tmp/nox-loop-type-shadow-before.log`). Sealing now
clears type and name frames together; type lookup follows the nearest binding,
and rebinding clears obsolete same-frame layout. The canonical language state
sentence and prototype self-hosting descriptions
were corrected as authorized; these corrections do not close the larger
self-hosting/formal/FHE obligations. Final Joy proof regression passed **2 tests in11.74s**, zero warnings:
`/tmp/joy-nox-loop-proof-final.log`. It generates six public execution
certificates across two profiles/three control-flow paths and one genuine
Triton7 private execution proof. The private case consumes only its first secret
and skips the later divine after return. Fresh decoded verification succeeds;
changed public input, result and cost fail. These are actual proofs, not mocks.

Final post-fix `cargo test --release --workspace --features neural --locked`
passed **858 tests**, zero failures/ignored/warnings, across18 binaries, exit0
(`/tmp/trident-nox-return-workspace-neural-final.log`). This includes39 actual
nox surface tests and the aggregate-shadow regression. The larger formal,
general dynamic relation, self-hosting and secure FHE obligations above remain
open; this receipt closes the bounded nox loop-return implementation task only.
