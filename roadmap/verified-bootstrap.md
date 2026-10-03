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

VB requires **maintained Rust and Trident implementations of every critical
soft3 component**: compiler, Eidos, nox, the complete scoped Zheng prover and
verifier, the complete scoped Joy delivery, and their critical dependencies.
Both implementations have acceptance obligations; Rust remains a required
implementation and cross-verification route, including where it accelerates
canonical Trident execution.
Compiling `.tri` code and proving its execution are implementation steps;
correspondence with the specification requires its own evidence.

| Owner | Required Rust/Trident implementation pair and verification responsibility |
|---|---|
| Trident | `.tri` compiler and its complete imports; source admission, frontend semantics and nox generation; independent source/binary correspondence for the delivered compiler |
| Eidos | Proof-term decoding, environment admission, type checking, conversion/reduction, inductive rules and all proof-production/import/export operations used in the delivery; independently justified logic, implementation and executable provenance |
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
The source admission and compilation behavior used by every mandatory port is
required in both compiler implementations. Auxiliary editor features can have a
separately declared scope only when they cannot influence accepted artifacts or
claims. Compiler features needed by the ports extend the frozen VB closure and
must be validated before those ports can pass.

Joy's host boundary must enumerate each filesystem, process, network, clock and
entropy operation. Canonical `.tri` code owns semantic and security decisions.
Every critical semantic routine has both implementations and checked obligations.
The OS/firmware/hardware boundary has named assumptions for actual external
effects; listing a portable semantic routine as a host assumption cannot waive
the implementation pair. An unexplained permanent Rust Joy exemption keeps VB open.
Classify trust separately for result soundness, confidentiality and availability.
An accelerator untrusted for result validity may still see secret data.

## Cross-verification contract

VB0 records one row per critical operation: specification, Rust source, Trident
source, artifact/build lineage, dependency closure, proof obligation, comparison
rule, corpus and status. A missing implementation, required proof or comparison
keeps the owning gate and VB8 open. Common specifications, test fixtures and
proof formats are intentional; shared semantic code, generators and toolchain
ancestry must be disclosed as correlated trust. Translating one implementation
mechanically into the other alone supplies no independent implementation check.

Each pair needs checked refinement to the same reviewed specification, or checked
validation covering every admitted operation with that narrower claim explicit.
Add differential tests for positive, malformed, boundary and exhaustion cases.
Compare deterministic results, errors and specified logical charge; compare exact
bytes where canonical encoding requires it. For differing valid compiler layouts,
check emitted-program semantics as well as per-compiler reproducibility; VB2's
final DDC comparison remains exact. Check randomized proofs in both directions
(Rust producer/Trident verifier and Trident producer/Rust verifier), as well as
both same-implementation paths. Freeze budgets and test-only randomness first.
Agreement is regression evidence; formal obligations and independent provenance
remain required even when both implementations agree.

### Language capabilities and cost guarantees

Preserve explicit resource contracts while adding only features required by the
ports. Trident's bounded loops and acyclic source call graph simplify analysis;
they do not alone bound arbitrary dynamic nox application, decoding, allocations,
normalization or physical proving time. Runtime fuel bounds admitted work and
may end in exhaustion; a bound sufficient for successful completion needs its
own argument. Track logical charge, memory and host time separately.

Finite sum types and statically specialized type parameters can preserve bounded
execution. They are separate compiler/design obligations, rather than necessary
sources of unbounded cost. Unrestricted recursion and escaping closures remain
outside the current language contract. Eidos binding/substitution can operate on
explicit term data with bounded stacks; it does not require Trident closures.
If a port needs a language extension, first specify its lowering, size/termination
or fuel contract, exhaustion behavior and cost model; implement it in both
compilers and extend VB2/VB3. Resolve older target-wide exclusion prose against
the actual native Noun/collection/runtime contract in that owning feature PR.
The [bounded metaprogramming assessment](bounded-metaprogramming.md) maps existing
roadmap ideas to concrete feature gates. This plan does not silently change the
language specification.

## Claims that receive separate evidence

| Claim | Required evidence |
|---|---|
| Binary/source correspondence | An independently obtained compiler/interpreter route, exact parent/source/build inputs and DDC comparison with the delivered artifact |
| Compiler correctness | A checked semantic-preservation theorem for the frozen source domain, or checked translation certificates for every delivered program with the narrower scope stated explicitly |
| Eidos correctness | Soundness of the admitted logic and checked refinement of both executable checkers, including environment admission, reduction, decoding, budgets and proof import/export |
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
| VB1 | Independently justified executable checking root and Eidos proof bridge | VB0 | soft3 + Eidos | open |
| VB2 | Compiler binary/source correspondence through independent bootstrap | VB1 | Trident + soft3 | open |
| VB3 | Compiler semantic correctness for the declared domain | VB1, VB2 | Trident | open |
| VB4 | Canonical nox with checked semantic refinement | VB3 | nox + dependency owners | open |
| VB5 | Canonical Zheng prover and verifier with checked obligations | VB3, VB4 | Zheng + dependency owners | open |
| VB6 | Complete scoped Joy on the canonical stack | VB4, VB5 | Joy + cyber adapter owners | open |
| VB7 | Adversarial trust-boundary acceptance | VB2–VB6, E4 | independent review + all owners | open |
| VB8 | Complete Rust/Trident pairs and independently checked full-stack bootstrap | VB0–VB7, E0–E4 below | soft3 + all owners | open |

### VB0 — Freeze scope and assumptions

Produce a machine-readable inventory of commands, library APIs, proof profiles,
source/dependency closures, artifacts, platforms and external effects. Link
every item to its canonical owner and acceptance check. Include public compiler
certificates and every other currently shipped production profile, including
supported feature combinations. An unported shipped profile keeps VB open;
private profiles remain mandatory even though the first compiler workload is
public. The inventory covers the complete pinned delivery, the Eidos proof-development
and checking closure, and both implementations of each critical operation.
Mark every absent port or missing Eidos feature as an open mandatory obligation;
an early pilot may reduce its workload, but cannot reduce final VB scope.

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

The root is a small, explicitly justified mechanism for accepting formal claims:
reviewed logical rules, a checker that implements them, its executable provenance,
and the environment that reads the intended statement and runs the checker.
Its acceptance must not rely solely on approval by the stack under examination.
Rust/Trident agreement cannot exclude a shared rule error or contaminated build.
The trusted foundation includes the chosen logic/model and physical assumptions;
independent checking makes these explicit and reviewable, not assumption-free.

Use Eidos as the stack's proof language and checker. The proposed first VB1
implementation is a small independent Eidos proof checker in **CakeML**, with
its rules and implementation justified in **HOL4**. Start on Linux x86_64 as an
external bootstrap/release-audit tool; it is separate from the ordinary Joy
runtime package. This is a third implementation of the checking kernel. It adds
no third complete compiler, optimizing VM, Zheng producer or Joy product.

Its scope is a bounded canonical proof/theorem/environment decoder, scope and
substitution checks, admitted universe/type/inductive/reduction rules, explicit
assumption tracking and exact statement/artifact binding. It consumes closed
proof terms and an independently supplied expected statement. Source elaboration,
tactics, proof search, editor tooling, networking and proof generation remain
outside this checker. Implement the minimal fragment first, then cover every
rule used by the complete VB delivery. Resource exhaustion is a refusal to
certify. Both the implementation and its resource policy need formal obligations.

Reuse CakeML's verified compilation/bootstrap infrastructure; assess the existing
verified HOL/OpenTheory checkers for reusable infrastructure. They do not accept
Eidos judgments directly. The required Eidos metatheory/interpretation and checked
proof-format bridge are new work. CakeML compilation alone proves neither Eidos
logic soundness nor this checker's correctness. HOL4 proofs must separately bind
the reviewed logical model, checker algorithm and actual executable behavior;
state any stronger foundational assumptions required by the admitted universes.
If the whole required fragment cannot be justified, E0 must revise the root
proposal explicitly and VB1 remains open. No complete CIC-to-HOL bridge is claimed.

Record kernel soundness, implementation refinement, proof-format/statement
translation, decoder/arithmetic and executable construction separately. Proposed
owner paths are `eidos/bootstrap/cakeml/` for this checker and
`eidos/proofs/hol4/` for its justification, beside maintained `eidos/rs/` and
planned `eidos/tri/`. These paths describe future work, not existing code.

The external route checks the actual intended theorem and Eidos rules/refinement;
a checked translation or an independently specified checked interpretation must
preserve their meaning and assumptions. Exporting an opaque “Eidos accepted” flag
cannot establish them. When proof-language features grow, extend and revalidate
this bridge before accepting claims using them. If no assessed route covers the
required judgments, keep VB1 open and implement the missing bridge/checker.

Establish an auditable binary bootstrap path independent of the candidate
Trident/Rust lineage for the root's checker, executor and comparison. A small seed
assembled from inspected bytes is one candidate; the loader and execution
environment remain explicit assumptions. Independently review source/rules and
record retained source, seed and executable identities.

First run a vertical experiment: accepted and rejected formal certificates,
their exact executable checker, and a source-to-executable correspondence claim
for a small program. Demonstrate that the checked statement binds the actual
source bytes and artifact bytes. This selects a workable proof/export route
before committing to a whole compiler proof effort.

Acceptance `vb.root`: reproducible checker bytes from the chosen root, reviewed
dependency/lineage ledger, independent execution/readback and rejection of changed
axioms, statement, proof or checker selection. Document residual assumptions.
Agreement between implementations is supporting evidence; it cannot discharge
the specification/refinement obligations by itself. VB1 first accepts a minimal
external checking route for the frozen rules. Eidos's final Trident executable
is accepted later through VB3/VB4 and that route, avoiding a dependency cycle.
Eidos prototypes can propose terms for VB2–VB6 before E4 acceptance; the external
route checks them directly. Acceptance never depends on trusting an unaccepted
Eidos executable to certify its own prerequisites.

### Eidos implementation workstream — required for VB8

Eidos currently provides a Rust strict checker for fixed Nat/Bool/Eq/Pos/BNat
proof terms; arbitrary inductive admission, bounded normalization, a Trident
implementation and independent soundness/refinement remain work. Existing
arithmetic certificates provide a starting corpus. The owner must reconcile
aspirational kernel/nox specifications with implemented behavior in E0; theorem
counts and unchecked complexity sketches do not establish these obligations.

Every item below is mandatory work, with status **open**. Needed features are
implemented in Rust and Trident; existing admissions remain closed until the
corresponding rule and implementation checks pass.

| Slice | Implementation and required evidence | Acceptance dependencies |
|---|---|---|
| E0: contract and root experiment | Freeze the logic, proof format, arithmetic/cost model and implementation inventory; demonstrate VB1 on a small actual source/artifact claim; reject changed rules, proof and expected statement | VB0; closes with the bounded VB1 experiment |
| E1: bounded kernel pair | Independent decoding, scopes/substitution, dependent products, universes, conversion and reduction; explicit work/storage limits with exhaustion reported as resource failure, never acceptance; differential strict corpus and checked rule/refinement obligations | E0/VB1; Trident artifact acceptance also VB3/VB4 |
| E2: admitted inductives and recursion | Validated inductive environments, parameters/indices, positivity, universe/elimination restrictions, constructor/recursor typing and definitional reduction; termination discipline for admitted definitions and sufficient resources for the declared workload; malformed/cyclic/ill-scoped declarations rejected | E1; each added rule covered by VB1 bridge |
| E3: semantics and proof pipeline | ASTs, nouns, traces, finite maps/lists, machine/field arithmetic, serialization/hash semantics and algebra/probability sufficient for the complete frozen compiler/nox/Zheng/Joy/dependency specifications; both implementations of elaboration, tactics, automation and import/export actually used; closed explicit terms rechecked at the kernel boundary | E1/E2, extended VB1 bridge, component specifications |
| E4: accepted checker delivery | Build both checkers through justified routes, bind exact theorem/dependency/assumption manifests, cross-check all delivery proofs, reject corrupted terms/theories/identities, and independently validate logic/refinement and statement-preserving export | E3, VB2–VB6; required by VB7/VB8 |

Joy must execute the Trident checker on nox. Zheng execution certificates bind
its exact checker, theorem, proof and environment identities; the independent
root still checks the logical/refinement claims. Reject substitution of any of
these identities.

Map each required theorem to the minimal adequate fragment; implement missing
expressiveness until all mandatory obligations are covered. General inductive
admission and usable semantic models are planned features, not permanent escapes
to assumed evaluation functions. Replace axiomatized executable operations with
actual definitions and checked refinement. Retain explicit foundational and
cryptographic assumptions in each theorem's manifest; law parameters remain
premises, never discharged by giving them a name.

Proof search and tactics may propose arbitrary terms; both kernels must recheck
the complete term/environment closure. Porting those producers is part of the
required delivery where used, while kernel checking keeps their mistakes from
becoming axioms. Encode recursive proof terms as bounded data traversals in
Trident; recursion in Eidos's object language needs justified elimination or
termination rules. A fuel limit controls checker work and cannot prove logical
normalization. Prove normalization if conversion/decidability/completeness claims
rely on it; otherwise state the resource-bounded partial-checking claim precisely.

### VB2 — Independently establish compiler correspondence

Obtain a compiler or source interpreter for the actual parent compiler's source
domain through the VB1 route. Implement it independently of the candidate's
parser, lowering and code generator, or explicitly account for shared trusted
code. Running one candidate binary on two VMs cannot establish its source origin.

The proposed first route adds a reference interpreter for the frozen parent
Trident source domain and an exact comparator in CakeML, obtained through VB1's
justified construction route. Let P be the exact parent compiler source and A
the claimed compiler source. Execute **Interpret(P, A)** and compare its output
directly with the delivered artifact attributed to that parent. The independent
interpreter plus P acts as the independently obtained parent compiler. Its
parser/semantics must cover the complete parent source and imports; A is that
compiler's input. This is additional work in VB2 beyond the VB1 proof checker,
and is not supplied by CakeML's existing compiler. A separate minimal reference
nox evaluator can check emitted-program behavior; it adds no generation to this
source-correspondence comparison. Jets, optimizing backends, the Zheng producer
and Joy's product surfaces stay outside the reference interpreter.

The alternative compiled DDC route has two stages: independently compile P to p1,
then independently execute p1 on A to obtain a2 and compare a2 with the delivered
artifact. A p1 targeting nox needs an independently justified nox executor. Keep
the direct-interpreter and compiled routes distinct, including when P differs
from A. Each interpreter/executor and comparator needs reviewed semantics,
checked refinement, resource contracts and exact executable provenance.

Record parent identities and semantics, source/library inputs, flags, environment
effects and the trusted comparison. Independent first-stage executables may
differ. The final comparison is exact over executable bytes and behavior-affecting
metadata; exclusions require an established semantics-preserving rule.

The frozen-S1 starting comparison targets delivered C2: its declared parent C1
implements the same S1 source. Keep the Rust-seed → C1 → C2 lineage explicit.
For a later compiler with different parent source, use that exact parent in the
chosen construction. Both execution of the independently obtained parent
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
Eidos/nox/Zheng/Joy programs and all critical dependencies. Features introduced
by those ports extend this gate. Both compiler implementations cover that domain
and meet the cross-verification contract.

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
to its implementation and proof obligation. Both implementations meet the cross-verification contract. Physical time, peak
host memory and GC overhead remain measured quantities unless separately modeled
and proved. Memory/GC semantics, including object lifetime, noun identity,
allocation failure and secret retention where applicable, remain part of the
paired correctness/security obligations.

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
verification, not unconditional byte equality. Rust and Trident producers/verifiers are both mandatory, maintained and subject
to the cross-verification contract. Neither implementation replaces the other
required delivery.

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
Rust/Trident refinement and cross-verification must be reviewable; successful `prove`/`verify`
smoke alone cannot close this gate.

### VB7 — Adversarial trust-boundary acceptance

Build isolated, inert adversarial fixtures for a self-propagating compiler
injection absent from source, malicious source logic, modified nox semantics,
permissive verifier, incorrect relation, corrupt prover output, wrong public
statement, secret disclosure in a private fixture, swapped dependencies and
misleading host output. Add unsound Eidos rule admission, invalid universes or
inductives, proof-export meaning substitution and exhaustion accepted as proof.
Keep fixtures outside normal build/publication paths.

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

Build both Rust and Trident implementations of the full frozen
compiler/Eidos/nox/Zheng/Joy/dependency closure through justified routes. Require
all Eidos E0–E4 obligations and every operation in the implementation-pair matrix.
Produce and check actual self-builds with the canonical prover and verifier;
also verify via the independent root. Bind all formal certificates, DDC results,
sources, dependencies, parameters, platform results and trust assumptions to one
delivery manifest. Repeat fresh package consumption on the existing claimed
macOS/Linux/Windows ARM64/x64 matrix, retaining the exact scope for each result.

Acceptance `vb.delivery`: every required VB gate passes for this exact closure;
another environment can reproduce the bootstrap and check the retained package
without the original private workspace or an opaque unrecorded seed. The source
inventory, comparison tools and expected statements are independently authenticated.
Any unmatched artifact, missing implementation/profile/proof, native semantic
fallback or unresolved mandatory assumption keeps the milestone open. A pilot or
one completed pair closes only its declared slice; VB8 requires the whole scope. Owner-controlled version, default
branch, tag and public-release operations retain their existing policy.

## Delivery sequence and first work item

Keep each implementation slice on an owning feature branch with a PR to
`release/0.4`. This planning delivery changes no component version or Kelvin
temperature. VB is a separate roadmap item and remains open after SH8; a release
claiming verified bootstrap must pass VB8. A narrower self-hosting release must
state its SH scope explicitly under the owner release policy.

The next slice is **VB0: inventory and trust contract**. Start by capturing the
accepted Trident/Joy/nox/Zheng pins and the actual Eidos/dependency revisions,
then enumerate delivered surfaces, theorem needs and dependency algorithms.
Record both implementation paths and missing ports for every critical operation. Add owner specifications and a
machine-readable claim/assumption matrix, with inventory rejection checks. Its
PR must list the proposed VB1 root, the small correspondence experiment, fixed
resource caps, Eidos E0–E4 owner slices and unresolved selection criteria. Do not start an expensive full
compiler proof run to decide whether that root is viable.

Reconcile the historical `cyber/research/bootstrap.md` in its own delivery:
permanent Rust-prover exemptions and unconditional trust claims must be replaced
by the full canonical scope and explicit assumptions, preserving historical
context. The Trident roadmap and acceptance ledger already distinguish SH and VB.

After the VB1 experiment, size subsequent slices in sessions/pomodoros using its
measured proof/export and execution costs. A calendar completion date or whole
stack effort estimate is currently open. Parallel canonical ports may prepare
VB4–VB6 and Eidos work while VB1–VB3 proceed; acceptance waits for the listed
gates. Incremental delivery changes execution order, never the required scope.

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
not implicit permission to omit Eidos, either implementation, the prover, Joy or
independent source provenance.
