# Scalar path audit validation

The bounded scalar branch/return gate is implemented under the canonical
[formal audit contract](../reference/formal-audit.md). This receipt does not
certify the whole standard library or Neptune SDK.

## Corrected behavior

The former formal coverage gate rejected every conditional and nonterminal
return. The new executor enumerates scalar paths, keeps declaration scopes and
mutations separate, stops each returning path, and checks postconditions for
each explicit return or function tail. Nested branches share monotonically fresh
symbol IDs instead of reusing branch-local IDs. Assertions remain obligations;
they never become assumptions that could hide their own failure. Path and step
limits fail closed as UNKNOWN.

Branch semantics are target-aware: nox raw zero means true; Triton raw nonzero
branches true, while an assertion requires exactly one. Source Bool comparisons
and literals preserve each ABI. The regression also found contract serialization
turning source `true`/`false` into `True`/`False`; the two parser lexeme arms now
preserve source spelling.

## Executed checks

- `cargo test --release --locked --test formal_audit`: **12 passed**, no
  failures or ignored tests. Log: `/tmp/formal-path-cli-final-current.log`.
  Real Z3 checks include nested and sequential early returns, independent
  function symbols, lexical shadowing after an outer assignment, false
  postconditions, failing assertions, infeasible paths, no obligations,
  enumeration limits, and missing/UNKNOWN/error solvers. Explicit Z3 test
  executions ran with Z3 available.
- `cargo test --release --locked --lib verify::`: **83 passed**, no failures,
  no ignored tests; 592 filtered. Log: `/tmp/formal-path-verify-final.log`.
- Native nox execution agrees with formal results for Field conditions 0, 1,
  and 2 and Bool conditions 0 and 1. This is part of the 12-test CLI suite.
  Additional installed Triton execution checked all three Field conditions
  (`/tmp/formal-triton-native-branch.log`). That installed binary predates the
  typed-entry/API-3 rebuild; the cases use public reads and writes, and this is
  branch-semantics evidence only, not a current installed-release receipt.
- The targeted Cargo runs emitted no Rust warnings. The final broad workspace
  neural suite is a separate coordinated release check, not inferred here.

## Current library inventory

The fresh API-3 owner-assisted audit ran all **48 modules**: 34 Trident core
library modules and 14 Trisha production SDK modules. It used the freshly built
Trident release CLI and current Trisha debug target discovery with
`audit --target neptune --json --z3`. Results are in
`/tmp/formal-path-inventory/results.json` and per-module logs beside it.

**46 modules returned UNKNOWN (exit 2); two returned a counterexample verdict
(exit 1).** Across 1,324 functions, 1,321 remained UNKNOWN and three were UNSAFE;
no entire module was certified. The three universal counterexamples are
`custom_token.checked`, `plumb.assert_non_negative`, and `plumb.check_index`:
their explicit guards reject some unconstrained caller inputs. This does not
establish that a valid caller violates a token policy; a compositional contract
would need to describe and prove the caller's preconditions.

Remaining formal algorithms are explicit: aggregate/digest/array values and
mutation, bounded-loop induction or complete unrolling with obligations,
match coverage, imported/helper contract summaries, RAM and state effects,
cryptographic operation models, and opaque/assembly operations. The new scalar
path support closes the conditional/return gate without representing these
unimplemented analyses as a formal pass. Existing compiler, execution, and proof
receipts remain different evidence from universal source-level verification.


Subsequent terminal-value correction: source terminal if/else branch values are
normalized to explicit returns through the shared AST helper before formal
execution. The real-Z3 suite rejects a false zero postcondition for a 7/9
terminal conditional. Scalar Field literals and negation of zero now use
canonical Goldilocks values. These regressions pass in the 883-test workspace
neural run `/tmp/generic-return-workspace-neural-final3.log`; no unsupported
helper/aggregate/loop analyses were promoted to formal safety.

## Body-derived same-module scalar helpers — 2026-09-12

The canonical reference was extended before implementation. Supported ordinary
acyclic scalar helpers now execute symbolically from their bodies, rather than
being represented by an opaque fresh call variable. The existing function map
already carries local definitions; no compiler, AST, TIR, parser or external
function-environment API change was required for this work.

All call arguments are evaluated in the caller environment before callee
parameter binding. The callee receives an isolated environment, retains fresh
symbol versions, and restores the caller environment and path after returning.
Actual return paths form a guarded symbolic conditional result. Callee narrow
parameter ranges and requires predicates are call-site obligations; assertions
and ensures predicates are obligations under their selected paths. No ensures
predicate is assumed as an uninterpreted summary. Root entry requires remain
assumptions, as specified by the existing formal contract.

Coverage rejects recursive or ambiguous definitions, generic/conditional or
intrinsic definitions, foreign calls, contract helper calls, and helpers with
I/O/nondeterminism/state/opaque effects. Scalar helper purity is derived from the
supported body/call graph, not trusted from a `#[pure]` annotation. The legacy
exploratory control-flow evaluator is now test-only; production supported calls
cannot accidentally fall back into its old incomplete inlining behavior.

Bounds are explicit:4096 shared path/statement steps,256 paths per frame,
64 nested calls,256 helper calls, and65,536 charged symbolic expression/constraint
nodes. The last bound prevents acyclic doubling from evading a call-depth bound.
Exhaustion marks the system unsupported; placeholder values after exhaustion
cannot produce SAFE because the unsupported reason remains attached.

Executed independently on the current workspace:

- Full `formal_audit` integration suite:17 passed,0 failed, using actual CLI
  and installed Z3 4.15.4. Log `/tmp/formal-scalar-full-audit.log`.
- Symbolic executor units:16 passed,0 failed.
  Log `/tmp/formal-scalar-sym-units.log`.
- No compiler warnings in these receipts; no heavy execution proof was run.

New positive checks prove swapped/asymmetric caller arguments, callee branch
shadowing, nested helper results and selected-path assertions. Changed caller
postconditions, false callee ensures and violated callee requires fail. A helper
without a proved contract is checked from its actual body rather than trusted.
Two adversarial acyclic fixtures independently exceed call/statement expansion
and expression-duplication budgets and remain unsupported.

Direct symbolic recursion is UNKNOWN. The public CLI applies the language's
existing recursion rejection earlier and exits1 with `recursive call cycle`;
this is a compilation failure, not a successful proof or a changed symbolic
verdict. An effectful helper reaches formal coverage and exits2/UNKNOWN.
The standalone actual recursion receipt is `/tmp/formal-recursive-cli.json`.

These gates extend the documented bounded scalar subset only. They do not
certify arbitrary imported libraries, generic bodies, aggregate semantics,
loops, opaque cryptography or whole-program execution safety.

## Complete library inventory after scalar helpers

The same48-module inventory was rerun with the current release binaries and
actual Z3 4.15.4. All48 invocations completed in2.04s;46 modules returned
UNKNOWN and two returned UNSAFE. Of1,324 functions,1,320 remain UNKNOWN and
four are UNSAFE. No whole module is certified. Complete JSON and source/binary
hashes are in `/tmp/formal-scalar-inventory/{results,receipt}.json`;
the command transcript is `/tmp/formal-scalar-inventory-run.log`. The
[durable inventory receipt](formal-scalar-inventory.json) preserves the exact
binary, source and detailed-result hashes.

The newly analyzed `plumb.next_nonce` now exposes its actual helper obligations:
both `nonce` and `nonce + 1` must fit U32. An unconstrained Field parameter
does not meet those universal obligations; in particular U32::MAX cannot be
incremented. Its new UNSAFE verdict reports the existing rejection guard and
is not evidence of an accepted invalid nonce. The other three guard verdicts
are unchanged. No input assumption was inserted to turn this inventory green.
This checkpoint supersedes the earlier1,321-UNKNOWN/three-UNSAFE count above.
