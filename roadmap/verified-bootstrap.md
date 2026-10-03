---
status: draft
milestone-status: open
---

# Verified Bootstrap on soft3

Milestone: **VB — Verified Bootstrap**, a verified bootstrap of the declared
soft3 delivery from an independently justified trust root. All VB gates are
open. This is an implementation plan; the gate identifiers below name planned
executable checks. Their runners and acceptance evidence are future work.

The question this milestone must answer is: **does the delivered compiler and
execution/proof stack correspond to its reviewed sources and specifications,
under an explicit, independently justified set of trust assumptions?**

The [accepted SH0–SH8 workload](../audit/self-hosting/bootstrap-results/whole-proof-final/acceptance/README.md)
provides self-compilation on nox through Joy, a reproducible fixed point and
independently checked public execution certificates. VB adds source/binary
correspondence, semantic correctness and the independent root. Existing SH
receipts retain their original claims and source identities.

The published claim after VB acceptance must name its exact source closure,
language domain, proof profiles, platform coverage, assumptions and residual
trust. Source correspondence excludes hidden binary additions under those
assumptions. Source review and the formal specifications determine which
source-level behaviors are allowed. General absence of malicious intent,
unmodeled side channels and unconditional hardware correctness remain separate
claims.

## Mandatory delivery scope

VB includes the compiler, canonical nox, **the complete scoped Zheng prover and
verifier**, and **the complete scoped Joy delivery**. Rust acceleration may
coexist with canonical Trident implementations once its obligations are met.
Compiling `.tri` code and proving its execution are implementation steps;
correspondence with the specification requires its own evidence.

| Owner | Canonical implementation and verification responsibility |
|---|---|
| Trident | `.tri` compiler and its complete imports; source admission, frontend semantics and nox generation; independent source/binary correspondence for the delivered compiler |
| nox | `.tri` evaluator, noun/encoding semantics, arithmetic, control, errors, logical cost, state/witness boundaries and every operation in the frozen delivery; jets and caches preserve the canonical result and declared cost |
| Zheng | `.tri` relation construction, witness handling, proof generation and verification for every frozen production profile; transcripts, commitments/openings, folding/deciding, transport and dependencies actually used |
| Joy | Every shipped CLI/library surface and adapter in the frozen inventory: argument/package admission, dispatch, execution/proof selection, expected-statement binding, codecs, resource/error/cancellation policy and output publication decisions |
| Arithmetic, Hemera, Lens and other dependency owners | Canonical operations and correctness obligations for the exact transitive algorithm closure used above; state-related dependencies are included where a frozen profile uses them |
| soft3 | Composition contract, trust ledger, independent root and complete bootstrap/acceptance manifest |

Keep implementations with their component owners; Trident remains the language
and compiler. Joy remains soft3/cyber; Triton/Neptune remain Trisha-owned. VB
does not assert self-hosting on Triton. Trisha contributes the existing foreign
target regression/distribution gates where shared delivery changes affect it.

VB0 inventories existing shipped behavior, including faithful refusals, rather
than importing every aspirational network or deployment feature into this gate.
Full Rust CLI/LSP language parity is a separately named scope decision. Compiler
features needed by the canonical ports must be added to the frozen VB closure
and validated before those ports can pass.

Joy's host boundary must enumerate each filesystem, process, network, clock and
entropy operation. Canonical `.tri` code owns semantic and security decisions.
Each remaining native routine has an explicit refinement obligation or a named
trust assumption; an unexplained permanent Rust Joy exemption cannot close VB.
Classify trust separately for result soundness, confidentiality and availability.
An accelerator untrusted for result validity may still see secret data.

## Claims that receive separate evidence

| Claim | Required evidence |
|---|---|
| Binary/source correspondence | An independently obtained compiler/interpreter route, exact parent/source/build inputs and DDC comparison with the delivered artifact |
| Compiler correctness | A checked semantic-preservation theorem for the frozen source domain, or checked translation certificates for every delivered program with the narrower scope stated explicitly |
| nox correctness | Refinement of the canonical interpreter to the specified machine, including errors, state/witness behavior and logical cost |
| Proof validity | Checked soundness of the selected relations/protocols and refinement of the actual verifier; explicit cryptographic assumptions, parameters and any soundness error |
| Prover correctness | The canonical producer implements the protocol and generates accepted proofs for valid admitted workloads under its declared resource/precondition model |
| Private-profile protection | Protocol privacy and implementation obligations for randomness, secret handling and the declared leakage model; public compiler certificates keep their disclosed profile |
| Delivery correctness | Source-to-binary closure for every component, exact statement/artifact binding, host-boundary obligations and reproducible verification of the distributed package |

A sound verifier must withstand arbitrary prover output. Including the whole
prover also addresses completeness, secret handling and delivery correctness.
Porting either side to Trident supplies no automatic proof of these properties.
Recursive proofs terminate at the independently justified checker and its
assumptions; a component's self-approval cannot establish that checker.

## Gates and dependencies

Dependencies below govern **acceptance**. Prototypes and contracts may proceed
in parallel in separate file scopes; a passing prototype does not close a gate.

| Gate | Result | Closes after | Lead owners | Status |
|---|---|---|---|---|
| VB0 | Frozen claims, delivery inventory and trust ledger | SH baseline recorded | soft3 + all component owners | open |
| VB1 | Independently justified executable checking root | VB0 | soft3 | open |
| VB2 | Compiler binary/source correspondence through independent bootstrap | VB1 | Trident + soft3 | open |
| VB3 | Compiler semantic correctness for the declared domain | VB1, VB2 | Trident | open |
| VB4 | Canonical nox with checked semantic refinement | VB3 | nox + dependency owners | open |
| VB5 | Canonical Zheng prover and verifier with checked obligations | VB3, VB4 | Zheng + dependency owners | open |
| VB6 | Complete scoped Joy on the canonical stack | VB4, VB5 | Joy + cyber adapter owners | open |
| VB7 | Adversarial trust-boundary acceptance | VB2–VB6 | independent review + all owners | open |
| VB8 | Reproducible, independently checked full-stack bootstrap delivery | VB0–VB7 | soft3 + all owners | open |

### VB0 — Freeze scope and assumptions

Produce a machine-readable inventory of commands, library APIs, proof profiles,
source/dependency closures, artifacts, platforms and external effects. Link
every item to its canonical owner and acceptance check. Include public compiler
certificates and every other currently shipped production profile, including
supported feature combinations. An unported shipped profile keeps VB open;
private profiles remain mandatory even though the first compiler workload is
public. The inventory covers the complete pinned delivery.

Write the trust ledger: logic, specification review, cryptographic assumptions,
checker implementation/binary, toolchain lineage, OS/firmware/hardware, artifact
acquisition/comparison and any secret-bearing host. Distinguish assumed facts,
proved implications, measured tests and open obligations. List omitted shipped
surfaces as unresolved scope gaps, with their owner.

Acceptance `vb.scope`: inventory completeness and owner review, exact initial
source pins, a claim-to-obligation matrix and negative tests for missing profile,
dependency, host capability and changed artifact. Freeze budgets and comparison
rules before each later measured run. Record the first independent-root decision
experiment without declaring any VB implementation accepted.

### VB1 — Establish the independent root

Select an existing small formal checking kernel/proof format as the first
candidate; justify a new kernel only if assessed candidates fail explicit needs.
Record the decision, logical rules, parser/decoder, arithmetic, executable
construction and all I/O assumptions. Establish an auditable binary bootstrap
path that does not depend solely on the candidate Trident/Rust lineage. A small
seed assembled from inspected bytes is one candidate; its loader and execution
environment remain explicit assumptions.

First run a vertical experiment: accepted and rejected formal certificates,
their exact executable checker, and a source-to-executable correspondence claim
for a small program. Demonstrate that the checked statement binds the actual
source bytes and artifact bytes. This selects a workable proof/export route
before committing to a whole compiler proof effort.

Acceptance `vb.root`: reproducible checker bytes from the chosen root, reviewed
dependency/lineage ledger, independent execution/readback and rejection of changed
axioms, statement, proof or checker selection. Document residual assumptions.
Agreement between implementations is supporting evidence; it cannot discharge
the specification/refinement obligations by itself.

### VB2 — Independently establish compiler correspondence

Obtain a compiler or source interpreter for the actual parent compiler's source
domain through the VB1 route. Implement it independently of the candidate's
parser, lowering and code generator, or explicitly account for shared trusted
code. Running one candidate binary on two VMs cannot establish its source origin.

Apply the generalized DDC construction: independently compile the parent source,
use that result to compile the claimed compiler source, and compare the final
output with the delivered self-built compiler. Record parent identities and
semantics, all source/library inputs, flags, environment effects and the trusted
comparison. Independent first-stage executables may differ. The final comparison
is exact over executable bytes and behavior-affecting metadata; any excluded
metadata requires an established semantics-preserving rule.

The frozen-S1 starting comparison targets delivered C2: its declared parent C1
implements the same S1 source. Keep the Rust-seed → C1 → C2 lineage explicit.
For a later compiler with different parent source, use that exact parent in the
generalized construction. Both execution of the independently generated parent
and final artifact comparison must use the independently justified route. Record
the executor/comparator source and binary provenance; second-stage execution on
an unverified candidate nox/Joy lineage cannot establish this gate.

Acceptance `vb.correspondence`: the real compiler closure passes through the
independent route and matches the delivered artifact. A controlled contaminated
seed must remain self-reproducing under ordinary bootstrap and be rejected by
the independent correspondence check. Record which assumptions establish the
conditional binary/source claim. Preserve the original SH artifacts; later
compiler sources acquire new identities and acceptance evidence.

### VB3 — Establish compiler semantic correctness

Freeze source semantics independently of the candidate implementation, including
parsing, modules, types, evaluation order, arithmetic, effects, errors and bounds.
Connect exact source bytes to this semantics and the exact emitted artifact to
the nox semantics. Review the specification against intended language behavior.

Choose and record a checked preservation theorem for the frozen language domain,
or translation validation covering every program in the complete delivered
closure. The latter accepts only those translations; it supplies no universal
compiler-correctness claim. Cover the compiler's own translation and the later
nox/Zheng/Joy programs. Features introduced by those ports extend this gate.

Acceptance `vb.compiler-semantics`: VB1 checks the proof/certificates, including
rejection of a semantically changed instruction despite an otherwise valid
execution certificate. Independent positive/error corpora remain regression
evidence. A DDC pass alone supplies source correspondence, including any bugs
already present in that source.

### VB4 — Canonical nox

Implement the frozen evaluator and its required primitives in Trident in the
nox repository. Cover the entire declared machine surface, including the
state/query/witness boundaries absent from the pure SH compiler workload.
Specify errors, admission limits, noun encoding/identity and logical charge.
Jets, memoization and native acceleration require checked equivalence, separately
checked outputs with sufficient obligations, or an explicit disabled path.

Acceptance `vb.nox`: source-to-artifact correspondence and VB1-checked refinement
to the machine specification, plus independently executed positive/adversarial
corpora. Trap hidden host semantic fallbacks. Every enabled primitive must map
to its implementation and proof obligation. Physical time, memory and GC remain
measured quantities unless separately modeled and proved.

### VB5 — Canonical Zheng, including the prover

Port the full VB0 profile inventory into the Zheng repository and dependency
owners. Include witness/relation construction, trace handling, commitments and
openings, sumcheck/folding/deciding where used, transcript/domain separation,
randomness, serialization and both producing and checking APIs. The public
disclosed compiler relation remains one explicitly identified profile.

Acceptance `vb.zheng` has separate mandatory receipts for relation soundness,
verifier refinement, honest-prover correctness/completeness, canonical binary
correspondence and cross-implementation interoperability. Bind expected program,
input, output, profile and logical resources outside producer-controlled metadata.
Check chunk order/completion, encodings, transcript parameters and dependencies.
Prove the selected algorithmic bounds; measure host consumption separately.

For each declared private profile, check protocol privacy and implementation
randomness/secret-handling obligations under the chosen leakage model. Document
timing, host memory and other residual assumptions. Fixed randomness belongs only
to controlled test fixtures. Randomized valid certificates need compatible
verification, not unconditional byte equality. Rust remains a differential or
accelerated implementation with explicit obligations; its existence cannot
replace the canonical prover delivery.

### VB6 — Complete scoped Joy delivery

Implement or refine every VB0 Joy surface against canonical Trident logic in the
Joy repository. Preserve soft3/cyber ownership, expected-statement selection,
source/package admission, dispatch, codecs, limits, failures, cancellation and
atomic publication. Keep the syscall/transport boundary narrow and explicit;
trace every native call to its declared capability and obligation.

Acceptance `vb.joy`: VB1 checks refinement of canonical Joy decisions to their
specifications, or operation certificates with explicitly complete delivery
coverage. Include expected-statement selection, dispatch, admission, secret
handling and publication. A fresh process also executes every frozen command/API
case using the supplied canonical artifacts, including unsupported-capability
refusals and negative paths. Native semantic substitution must fail the gate.
Each actual host effect is either checked against its contract or listed as a
residual assumption with its security impact. The complete inventory and
canonical/native correspondence must be reviewable; successful `prove`/`verify`
smoke alone cannot close this gate.

### VB7 — Adversarial trust-boundary acceptance

Build isolated, inert adversarial fixtures for a self-propagating compiler
injection absent from source, malicious source logic, modified nox semantics,
permissive verifier, incorrect relation, corrupt prover output, wrong public
statement, secret disclosure in a private fixture, swapped dependencies and
misleading host output. Keep fixtures outside normal build/publication paths.

Each fixture names the property and gate expected to detect it. Source-visible
malice belongs to the specification/source review and semantic checks; DDC may
correctly accept its source correspondence. A malicious prover can fail to
produce a certificate; the required soundness outcome is refusal to accept a
false statement. A faulty verifier must be caught by the independently justified
route, never accepted because it agrees with itself.

Acceptance `vb.adversarial`: all expected detections, authentic positive controls,
unchanged statement selection and exact artifact identities, with original
failed attempts preserved. Test success supplements formal obligations and does
not establish a universal absence-of-backdoors theorem.

### VB8 — Full bootstrap and independent delivery

Build the full frozen compiler/nox/Zheng/Joy closure through the accepted root.
Produce and check actual self-builds with the canonical prover and verifier;
also verify via the independent root. Bind all formal certificates, DDC results,
sources, dependencies, parameters, platform results and trust assumptions to one
delivery manifest. Repeat fresh package consumption on the existing claimed
macOS/Linux/Windows ARM64/x64 matrix, retaining the exact scope for each result.

Acceptance `vb.delivery`: every required VB gate passes for this exact closure;
another environment can reproduce the bootstrap and check the retained package
without the original private workspace or an opaque unrecorded seed. The source
inventory, comparison tools and expected statements are independently authenticated.
Any unmatched artifact, missing profile, native semantic fallback or unresolved
mandatory assumption keeps the milestone open. Owner-controlled version, default
branch, tag and public-release operations retain their existing policy.

## Delivery sequence and first work item

Keep each implementation slice on an owning feature branch with a PR to
`release/0.4`. This planning delivery changes no component version or Kelvin
temperature. VB is a separate roadmap item and remains open after SH8; a release
claiming verified bootstrap must pass VB8. A narrower self-hosting release must
state its SH scope explicitly under the owner release policy.

The next slice is **VB0: inventory and trust contract**. Start by capturing the
accepted Trident/Joy/nox/Zheng pins, then enumerate the actual delivered surfaces
and dependency algorithms from those revisions. Add owner specifications and a
machine-readable claim/assumption matrix, with inventory rejection checks. Its
PR must list the proposed VB1 root, the small correspondence experiment, fixed
resource caps and unresolved selection criteria. Do not start an expensive full
compiler proof run to decide whether that root is viable.

Reconcile the historical `cyber/research/bootstrap.md` in its own delivery:
permanent Rust-prover exemptions and unconditional trust claims must be replaced
by the full canonical scope and explicit assumptions, preserving historical
context. The Trident roadmap and acceptance ledger already distinguish SH and VB.

After the VB1 experiment, size subsequent slices in sessions/pomodoros using its
measured proof/export and execution costs. A calendar completion date or whole
stack effort estimate is currently open. Parallel canonical ports may prepare
VB4–VB6 while VB1–VB3 proceed; their final acceptance waits for the listed gates.

Place specifications with their owners and measured results in each repository's
`audit/verified-bootstrap/`. Each receipt records the exact command, source and
tool revisions, assumptions, artifacts, outcome, resource profile and original
failures. Planning documents and test counts cannot stand in for proof receipts.
The Trident ledger links the component evidence; soft3 owns cross-component
composition and the final trust manifest.

## Sources and decisions still open

- [Thompson, Reflections on Trusting Trust](https://www.cs.cmu.edu/~rdriley/487/papers/Thompson_1984_ReflectionsonTrustingTrust.pdf): self-propagating binary injection motivates VB2 and VB7.
- [Wheeler, DDC](https://dwheeler.com/trusting-trust/dissertation/html/wheeler-trusting-trust-ddc.html): conditional source correspondence, parent compilation and trusted environment/comparison obligations.
- [GNU Mes bootstrap](https://www.gnu.org/software/mes/manual/html_node/Bootstrappable-Builds.html): auditable bootstrap provenance is a candidate technique for VB1.
- [CakeML verified bootstrap](https://cakeml.org/popl14.pdf): an existing example of semantic proof linked to compiler bootstrap; adopting its machinery is an open engineering decision.
- [ZKProof reference](https://docs.zkproof.org/pages/reference/reference.pdf): soundness, completeness and privacy are distinct obligations; the disclosed SH profile retains its explicit disclosure.

Root/kernel selection, formal semantics/proof-language integration and the exact
host-effect boundary close through VB0/VB1 evidence. These are tracked decisions,
not implicit permission to omit the prover, Joy or independent source provenance.
