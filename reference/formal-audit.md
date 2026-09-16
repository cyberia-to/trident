# Formal audit contract

`trident audit` checks universal assertion and postcondition obligations under
entry preconditions, independently for each function. It does not prove program
execution or that an arbitrary witness exists. `#[requires(expr)]` constrains
entry inputs; `#[ensures(expr)]` is checked independently on each terminating path with `result`
bound to that returned scalar. U32/Bool parameter ranges are entry assumptions.

The supported formal subset is scalar Field/U32/Bool arithmetic, scalar
bindings/assignments, nested if/else, early or terminal return, and explicitly
modeled scalar builtins. Each branch has its own lexical environment and path
predicate; only fallthrough paths execute subsequent statements. Postconditions
are checked under the corresponding terminating path, including implicit tails and terminal if/else branch values.
Internal path predicates use logical 0/1. Source Bool values follow the selected
ABI: nox uses 0=true, Triton uses 1=true. Raw Field conditions branch on zero for
nox and nonzero for Triton; assertions/contracts require exactly the selected
true word (Triton Field 2 branches but does not satisfy assert). Only these two
Goldilocks Boolean ABIs are currently modeled. Scalar Field literals are
canonicalized modulo the prime, matching both native owners. Assertions remain
obligations, never assumptions that could erase their own counterexamples.

Analysis enumerates at most 256 simultaneous paths and 4096 path/statement steps;
exceeding either limit returns UNKNOWN. Constant unreachable branches are not
executed; symbolic unreachable paths are excluded by their guarded obligations.
Same-module ordinary scalar helpers may be inlined from their checked bodies.
Calls evaluate all arguments in the caller environment before binding callee
parameters. Every callee requires predicate and narrow parameter range is a
caller-path obligation; every callee assertion and ensures predicate is checked
on its actual selected execution path. Ensures are never assumed as summaries.
The caller environment is restored after a call. Only acyclic helpers without
I/O, nondeterminism, state, assembly or opaque operations are covered. Calls in
contracts, generic/foreign/ambiguous definitions and recursion remain UNKNOWN.
Inlining shares the4096 statement budget, permits at most64 nested calls and
256 total helper calls, and retains the256-path bound per execution frame.
A shared65,536-node expression/constraint expansion budget also prevents
acyclic helper duplication from growing without bound; exhaustion is UNKNOWN.
Aggregates, loops, match, opaque operations and assembly remain UNKNOWN. Unsupported paths and absent obligations are never SAFE.

Static tautologies can be discharged without a solver. Random testing and bounded
sampling can find counterexamples but cannot certify universal safety. With
`--z3`, only UNSAT of a supported nonempty obligation query discharges it; SAT
fails, and UNKNOWN, solver errors or a missing solver are inconclusive failures.
Exit 0 means every analyzed function's obligations were discharged; exit 1 means
counterexample or compilation error; exit 2 means incomplete/unsupported analysis.
SMT exports reset solver state between functions to avoid shared variable IDs.
