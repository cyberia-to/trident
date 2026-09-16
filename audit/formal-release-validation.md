# Formal release validation

## Result

The formal release gate is **open**. Successful compilation and execution
receipts do not establish all library contracts. The audit command now reports
missing coverage instead of fabricating SAFE.

- Full Trident release/locked tests: **773 passed**, zero failures
  (`/tmp/formal-core-full.log`); includes six adversarial formal CLI/API tests.
- Inventory: **34 core modules + 14 Trisha production SDK modules**.
- Neptune-target compiler checks: **48/48 passed**.
- Neptune-target warrior builds: **48/48 passed**.
- Nox-target compiler checks: **23/34 passed**; the other11 reject unsupported
  target operations. No nox runtime or proof success is implied by typechecking.
- Formal audit with real Z3: **0 safe files,46 unknown files,2 unsafe files**.
  Per-function verdicts: **1321 unknown,3 unsafe**. Intrinsic declarations without
  bodies have no symbolic proof obligations and are never counted as proved.

Commands: `trident check --target neptune FILE`, `trisha build --target neptune
FILE -o TEMP`, `trident audit --target neptune --json --z3 FILE`, plus
`trident check --target nox FILE` for each core module. The installed target owner
was the locally built release Trisha on PATH. Detailed stdout/stderr and machine
results: `/tmp/formal-release-inventory/`; no live node/wallet operations.

The three counterexample obligations are input validators:
`os.neptune.custom_token::checked`, `os.neptune.standards.plumb::assert_non_negative`
and `os.neptune.standards.plumb::check_index`. Arbitrary Field inputs need not fit
U32; unconstrained id/index inputs include zero id and out-of-tree indices.
These helpers intentionally reject such inputs. Universal safety requires entry
assumptions/compositional caller obligations; this does not demonstrate a broken
Merkle authentication relation. Final solver-symbol corrections revealed the
first two SAT results, which previously returned UNKNOWN rather than a false pass.

## Corrected soundness/reporting defects

1. CLI analyzes functions independently and resets exported SMT state between
   queries. Project API aggregation uses disjoint names and retains coverage.
   SSA versions cannot collide with user identifier suffixes; SMT identifiers
   encode original name bytes and version separately.
2. SAFE requires supported, nonempty, discharged obligations. Empty/opaque and
   sampling-only checks return UNKNOWN in CLI, solver API, static API and JSON.
   Randomly satisfied assertions are no longer removal recommendations.
3. Requested Z3 SAT fails; UNKNOWN, missing solver, nonzero process status and
   solver errors cannot produce a successful audit exit. Query-only SMT avoids
   invalid get-model requests after UNSAT. Solver scripts use per-process stdin,
   eliminating the shared temporary-filename race between concurrent audits.
4. Straight-line scalar requires/ensures are modeled: entry predicates and
   U32/Bool parameter ranges guard obligations; terminal return supplies result.
   False postconditions produce counterexamples. Unknown names, opaque calls,
   malformed predicates and unsupported language shapes retain explicit reasons.
5. Assert means exactly1, matching VM assertions. Treating arbitrary nonzero
   Field values as successful assertions was removed from evaluator and SMT.

Canonical scope: `reference/formal-audit.md`, linked from `reference/language.md`.
No compiler lowering, library algorithms, benchmark or mining code was modified.

## Remaining algorithms and release work

A complete formal compiler audit still needs typed aggregate/tuple/digest values
and projections; call argument substitution with return values and callee
contracts; recursive call induction/bounds; precise branch/early-return/path
merging; bounded-loop invariants or complete bounded expansion with lexical
scope; RAM/array mutation and alias modeling; inverse/division/u32 arithmetic
exception conditions; target intrinsic summaries; and authenticated hash/crypto
relations. Those constructs currently return UNKNOWN. `requires` assumptions
are not themselves a proof that a satisfying input exists, and the audit is not
an execution proof or cryptographic security proof.

The11 nox compatibility failures are explicit capability checks: RAM access,
public stream IO and/or divmod operator requirements. Fixing them requires
implementing the corresponding nox semantics or choosing supported module entry
points; an audit result must not silently reclassify them as working.

## Per-file receipt

`check/build` below refers to Neptune. `unknown` is not a pass.

| File | Check/build | Nox check | Formal |
|---|---|---|---|
| `trident/lib/std/compiler/codegen.tri` | 0/0 (exit codes) | rejected | unknown |
| `trident/lib/std/compiler/lexer.tri` | 0/0 (exit codes) | pass | unknown |
| `trident/lib/std/compiler/lower.tri` | 0/0 (exit codes) | pass | unknown |
| `trident/lib/std/compiler/optimize.tri` | 0/0 (exit codes) | rejected | unknown |
| `trident/lib/std/compiler/parser.tri` | 0/0 (exit codes) | pass | unknown |
| `trident/lib/std/compiler/pipeline.tri` | 0/0 (exit codes) | rejected | unknown |
| `trident/lib/std/compiler/typecheck.tri` | 0/0 (exit codes) | pass | unknown |
| `trident/lib/std/crypto/bigint.tri` | 0/0 (exit codes) | pass | unknown |
| `trident/lib/std/crypto/ecdsa.tri` | 0/0 (exit codes) | rejected | unknown |
| `trident/lib/std/crypto/ed25519.tri` | 0/0 (exit codes) | pass | unknown |
| `trident/lib/std/crypto/keccak256.tri` | 0/0 (exit codes) | pass | unknown |
| `trident/lib/std/crypto/lut_sponge.tri` | 0/0 (exit codes) | pass | unknown |
| `trident/lib/std/crypto/poseidon.tri` | 0/0 (exit codes) | pass | unknown |
| `trident/lib/std/crypto/poseidon2.tri` | 0/0 (exit codes) | pass | unknown |
| `trident/lib/std/crypto/preimage.tri` | 0/0 (exit codes) | pass | unknown |
| `trident/lib/std/crypto/secp256k1.tri` | 0/0 (exit codes) | pass | unknown |
| `trident/lib/std/crypto/sha256.tri` | 0/0 (exit codes) | pass | unknown |
| `trident/lib/std/fhe/lwe.tri` | 0/0 (exit codes) | rejected | unknown |
| `trident/lib/std/fhe/pbs.tri` | 0/0 (exit codes) | rejected | unknown |
| `trident/lib/std/fhe/rlwe.tri` | 0/0 (exit codes) | rejected | unknown |
| `trident/lib/std/io/storage.tri` | 0/0 (exit codes) | rejected | unknown |
| `trident/lib/std/math/fibonacci.tri` | 0/0 (exit codes) | pass | unknown |
| `trident/lib/std/math/lut.tri` | 0/0 (exit codes) | rejected | unknown |
| `trident/lib/std/nn/tensor.tri` | 0/0 (exit codes) | pass | unknown |
| `trident/lib/std/private/poly.tri` | 0/0 (exit codes) | rejected | unknown |
| `trident/lib/std/quantum/gates.tri` | 0/0 (exit codes) | pass | unknown |
| `trident/lib/std/target.tri` | 0/0 (exit codes) | pass | unknown |
| `trident/lib/std/trinity/inference.tri` | 0/0 (exit codes) | rejected | unknown |
| `trident/lib/vm/core/assert.tri` | 0/0 (exit codes) | pass | unknown |
| `trident/lib/vm/core/convert.tri` | 0/0 (exit codes) | pass | unknown |
| `trident/lib/vm/core/field.tri` | 0/0 (exit codes) | pass | unknown |
| `trident/lib/vm/core/u32.tri` | 0/0 (exit codes) | pass | unknown |
| `trident/lib/vm/io/io.tri` | 0/0 (exit codes) | pass | unknown |
| `trident/lib/vm/io/mem.tri` | 0/0 (exit codes) | pass | unknown |
| `trisha/lib/os/neptune/auth.tri` | 0/0 (exit codes) | not applicable | unknown |
| `trisha/lib/os/neptune/custom_token.tri` | 0/0 (exit codes) | not applicable | unsafe |
| `trisha/lib/os/neptune/kernel.tri` | 0/0 (exit codes) | not applicable | unknown |
| `trisha/lib/os/neptune/native_currency.tri` | 0/0 (exit codes) | not applicable | unknown |
| `trisha/lib/os/neptune/recursive.tri` | 0/0 (exit codes) | not applicable | unknown |
| `trisha/lib/os/neptune/standards/plumb.tri` | 0/0 (exit codes) | not applicable | unsafe |
| `trisha/lib/os/neptune/transaction.tri` | 0/0 (exit codes) | not applicable | unknown |
| `trisha/lib/os/neptune/utxo.tri` | 0/0 (exit codes) | not applicable | unknown |
| `trisha/lib/os/neptune/xfield.tri` | 0/0 (exit codes) | not applicable | unknown |
| `trisha/lib/vm/triton/context.tri` | 0/0 (exit codes) | not applicable | unknown |
| `trisha/lib/vm/triton/hash.tri` | 0/0 (exit codes) | not applicable | unknown |
| `trisha/lib/vm/triton/merkle.tri` | 0/0 (exit codes) | not applicable | unknown |
| `trisha/lib/vm/triton/merkle_proof.tri` | 0/0 (exit codes) | not applicable | unknown |
| `trisha/lib/vm/triton/proof.tri` | 0/0 (exit codes) | not applicable | unknown |

Trisha release rebuild after compiler changes passed without Rust warnings
(`/tmp/formal-warrior-rebuild.log`). Final inventory was repeated after the
solver-symbol correction; the counts above are the final receipts.

Final mandatory neural-feature suite: `cargo test --release --features neural
--locked` passed **817 tests**, zero failures, zero Rust warnings
(`/tmp/formal-trident-neural-final.log`). This includes the parent's subsequent
registry-deploy state rejection regression.

Complete workspace closure: `cargo test --release --workspace --features neural
--locked` passed **851 tests**, zero failures/ignored/Rust warnings, including
trident-silicon (`/tmp/formal-trident-workspace-neural-final.log`).
