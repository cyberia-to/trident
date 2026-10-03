---
status: draft
milestone-status: open
---

# Bounded metaprogramming and ML-style abstractions

Question: which ML-style abstractions improve Trident's compiler/Eidos/soft3 ports
while preserving deterministic compilation, explicit execution resources and a
tractable compiler-correctness argument?

This is an assessment and proposed order, not accepted syntax or an implementation.
[Verified Bootstrap](verified-bootstrap.md) requires both Rust and Trident
implementations of critical components. A feature enters its mandatory language
closure only when a port needs it; the component's complete scope stays mandatory.

## Existing roadmap and implementation

The [language contract](../reference/language.md#size-generic-functions) implements
integer size parameters and monomorphization. Module visibility, nominal structs,
exhaustive literal/struct matches and explicit bounded loops also exist. Ordinary
calls form a DAG. General sum types, type parameters, function values and source
recursion remain outside the implemented language. Constant declarations accept
integer literals or constant references; this differs from general compile-time
function execution.

The historical [roadmap](../reference/roadmap.md) already proposes:

- §1.2 pass 12: constant evaluation through function calls.
- §1.3: supercompilation and first-class partial evaluation through `specialize`.
- §2.1–2.2: compositional proof-cost and table/effect types.
- §2.3–2.5: linear, refinement and dependent dimension types.
- §3.2–3.3: loop invariants and termination evidence.

Those examples describe future designs. The Rust seed already bounds size
specialization in `src/api/pipeline.rs`; that machinery is a useful starting
point, not evidence of general type/function specialization. The draft
[noun-types](noun-types.md) explicitly discusses sum types versus a built-in Noun.
No accepted general module-functor or higher-order-function contract was found.

## Preserve the actual premises

Track distinct obligations: deterministic meaning; termination under declared
preconditions; an upper bound on successful work; enforced exhaustion limits;
logical proof cost; and physical host/prover consumption. A runtime budget can
stop work safely without guaranteeing a successful answer within that budget.
A structural decrease can prove termination without providing a constant bound.
A bound on dynamic input size/depth is separate from its recursive type.

SML higher-order functions normally run at runtime. Compile-time evaluation is a
separate language phase. Ordinary SML supplies no general automatic bound on
execution. Trident extensions need their own termination, logical-cost and
storage contracts; adopting ML-style syntax supplies none of these automatically.

For every extension: specify types/effects and representation first; define
lowering and resource/exhaustion behavior; implement both compiler paths; update
cost/refinement obligations and independent VB2 interpretation; then compare
positive, malformed and resource-boundary cases. Preserve secrecy obligations:
secret-dependent branching/length/cost and compile-time capture can disclose data.
Only public declared build inputs may influence compile-time specialization.

## Feature decisions

| Feature | Benefit and need | Proposed bounded form and remaining cost |
|---|---|---|
| Finite sum types and exhaustive patterns | High value: AST nodes, proof terms, explicit success/error states; replaces manual tags | Start with fixed-layout variants. Validate tags and payloads at admission. Charge tag tests, selected branches and representation; runtime choices survive compilation. Recursive data needs an additional size/storage contract. |
| Type parameters | High value: typed collections and reusable proof/VM algorithms | Monomorphize a finite set of instantiations. Cap specialization work, instance count and output bytes; reject polymorphic recursion or expanding instantiation cycles. Runtime type dispatch is unnecessary. |
| Pure compile-time functions / partial evaluation | Reuses existing roadmap; useful for field constants, codecs and specialized kernels | Explicit phase, deterministic declared inputs, no ambient I/O/time/entropy, bounded evaluator work/storage/output. Required evaluation exhausts with a compilation error. Optional optimization has a specified deterministic fallback. Charge compilation separately from residual execution. |
| Static function parameters and non-escaping captures | Useful `map`/`fold` and reusable traversals; comes after type parameters | Specialize known callees; lower captures to explicit arguments. Verify a finite call graph, effects and code-size limits. Some closures need no runtime dispatch, but specialization can increase artifact size. |
| Abstract module interfaces / static functors | Useful component boundaries and interchangeable arithmetic implementations; current modules already provide some encapsulation | Extend signatures and instantiate modules statically if actual ports need them. Track nominal type identity, capabilities, imports, specialization and closure size. Full SML module calculus is not a current bootstrap prerequisite. |
| Structural or explicitly fuelled recursion | Convenient for recursive syntax/proof data; explicit stacks already provide an implementation route | Require a decreasing measure plus input size/depth and work/storage bounds, or an explicit fuelled contract. Define failure on exhaustion. Lower to a bounded internal machine; branching recursion needs total-work analysis, not just a depth cap. |
| Cost/effect/refinement contracts | Directly strengthens predictability, admission and safety | Prioritize lengths, indices, nonzero values, effects and logical-resource upper bounds. Distinguish checked static theorems from runtime assertions. Verify certificates rather than trusting a solver's answer; extensions remain target-specific where costs differ. |
| Escaping runtime closures, unrestricted recursion, arbitrary macros or exceptions | Larger semantic/runtime burden; no mandatory port currently establishes their necessity | Defer. A later proposal must account for environments, dispatch, allocation, unwind/effects, resource policy and proof obligations. Explicit sum-type errors cover many exception use cases. |

These are choices about representation and staging, not a claim that all SML
abstractions have zero cost. Every specialization is still code that the compiler
must validate and the bootstrap must reproduce. Finite sums/type specialization
are the first candidates; add bounded compile-time evaluation next when a concrete
port benefits. Static higher-order helpers can follow. Preserve bounded loops
until a separate recursion contract is accepted. Keep advanced type-level theorem
proving in Eidos and let Trident check the required certificates at a narrow
boundary rather than silently making arbitrary proof search part of compilation.

## Historical claims requiring reconciliation

The owning feature proposals must correct these before relying on them:

- `noun-types.md` claims arbitrary Noun variants are statically known and recursive
  types bound recursion. Runtime input shape requires actual validation/dispatch;
  size/depth and work bounds need separate evidence.
- Its zero-cost claim for branch dispatch is unsupported: using an existing nox
  instruction still consumes logical work and may enlarge the proof relation.
- `language.md` attributes every closure to dynamic dispatch. Static non-escaping
  captures can lower to ordinary arguments; the present exclusion remains policy
  until a concrete replacement contract is accepted.
- Roadmap §1.3 does not establish that specialization yields a minimal proof.
  Generated recursive control must respect a bounded internal-machine contract.
- Runtime refinement predicates establish properties of accepted executions;
  a universal semantic theorem and adequate-budget completion need separate proof.

These findings qualify historical proposals; accepted SH receipts and the current
language contract retain their existing scope.

## Decision and acceptance

Proposed order: sums and type specialization; bounded compile-time evaluation;
static function parameters; then recursion/module extensions only with an actual
port requirement. Cost/effect contracts progress alongside each stage. This
captures useful ML abstraction while keeping the source/runtime/proof boundaries
explicit. No general SML compatibility goal is introduced.

Each feature PR must include a real motivating component, Rust/Trident behavior,
accepted/rejected syntax, target lowering, deterministic specialization, resource
limits, source-to-artifact/refinement updates and measured compile/runtime/proving
costs. Ratios and savings are open until measured from pinned revisions. Keep
language features on separate branches from the VB trust-root implementation.

Reference: [Standard ML definition](https://smlfamily.github.io/sml97-defn.pdf).
This is comparative language research. The bootstrap uses the self-contained
soft3 root specified in the VB plan.
