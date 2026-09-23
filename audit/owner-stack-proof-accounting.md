# FINAL5 → owner-only stack lowering: proof accounting

2026-09-12. Statement identity review of the exact native-program comparison
recorded in `owner-stack-legalization-receipt.json`; no proof generation or new
proof verification was performed here. Frozen FINAL5 is unchanged.

The complete inventory remains **43 baselines, 99 positive fixtures, 34 negative
fixtures, 198 positive fixture/implementation proof obligations**.

| Proof obligation | Changed statement | Identical statement | Total |
| --- | ---: | ---: | ---: |
| Compiler/classic | 51 | 48 | 99 |
| Independent hand | 0 | 99 | 99 |
| Total | 51 | 147 | 198 |

All 51 changed statements have a changed actual native `Program.hash()`. Three
custom-token positives additionally change the public input roots and private
streams because their selected coin type is the executing program hash. Their
amounts, policy, expected result and hand statements stay unchanged. No output,
target selection, native claim version or proof format changes. There are no
private-stream changes under an unchanged statement.

Thus **51 fixture proof obligations require proofs of a different statement**.
The other 147 are mathematically identical statements under the compared native
Triton7/default-security contract. Identity is not a fresh verification receipt:
reusing any exact old proof would require retained proof bytes and a fresh
verification against the explicit new expected claim, plus accepted provenance.
The release gate's requirement of 198 fresh generated-and-verified events is
not weakened by this accounting. It must still check the complete denominator.

The 198 obligations contain 191 distinct mathematical statement keys both before
and after. Duplicate theorem keys do not erase fixture coverage or justify
reducing the 198-event gate. Negative fixtures remain 34 honest execution
rejections and must never produce successful-proof events.

## Exact accounting mechanism

- Reused all 84 independently computed before/after native source/target hashes,
  not TASM text hashes or cycle similarity. Of these, 40 programs changed, but
  several serve multiple positive fixtures; hence 51 changed positive keys.
- Constructed each hand program exactly as bench does: fixture `hand_prefix`,
  the complete hand assembly, then newline-separated `hand_libraries`. Computed
  its actual native program hash independently. All hand statements are equal.
- Parsed canonical numeric public input/output values and respected hand input,
  secret and digest overrides. Appended shared witness-file streams in order
  before applying hand overrides, matching `cli/bench.rs`.
- Included target, fixture/source byte identities, final private-stream hashes,
  witness-file identities and word counts alongside both statements. Private
  witness contents are not copied into the receipt.
- Audit statement keys are SHA256 of sorted compact JSON containing format,
  native claim version, native program digest and full public input/output.
  This identifier is not represented as a native Claim cryptographic hash.
- At review time, 93 completed `TRISHA_PROOF_VERIFIED` events from the still-running
  frozen FINAL5 gate matched their exact before program/input/output fields.
  This is a partial observation, not a completed 198-event proof receipt.

Machine-readable record: `/tmp/owner-stack-proof-reconciliation.json`.
SHA256: `52b3fbc228e91dc41a2c94abd3aad83334ed98fc4a66924fd6f82e278506b40e`.
Reproducer: `/tmp/owner-stack-reconcile-proofs.py`. Both use the native-program
snapshot in the linked repair receipt; any later compiler change requires a
new native comparison before extending these conclusions.

## Changed fixture/classic obligations

All rows below change program hash; the last column marks additional public
input/private-stream rebinding. All corresponding hand keys are unchanged.

| Positive fixture | Additional input rebinding |
| --- | --- |
| `fixtures/bigint-carry/vector.bench.toml` | none |
| `fixtures/compiler-parser-associativity/vector.bench.toml` | none |
| `fixtures/compiler-parser-parentheses/vector.bench.toml` | none |
| `fixtures/compiler-parser-precedence/vector.bench.toml` | none |
| `fixtures/compiler-pipeline-associativity/vector.bench.toml` | none |
| `fixtures/compiler-pipeline-parentheses/vector.bench.toml` | none |
| `fixtures/compiler-pipeline-precedence/vector.bench.toml` | none |
| `fixtures/custom-poseidon2-lanes/vector.bench.toml` | none |
| `fixtures/custom-poseidon2-ram/vector.bench.toml` | none |
| `fixtures/custom-poseidon2-zero/vector.bench.toml` | none |
| `fixtures/custom-token-v2-burn/vector.bench.toml` | public roots + private streams |
| `fixtures/custom-token-v2-mint/vector.bench.toml` | public roots + private streams |
| `fixtures/custom-token-v2-transfer/vector.bench.toml` | public roots + private streams |
| `fixtures/ecdsa-cross-limb/vector.bench.toml` | none |
| `fixtures/ecdsa-even-order/vector.bench.toml` | none |
| `fixtures/ecdsa-half/vector.bench.toml` | none |
| `fixtures/ecdsa-high/vector.bench.toml` | none |
| `fixtures/ecdsa-invalid-r/vector.bench.toml` | none |
| `fixtures/ecdsa-order/vector.bench.toml` | none |
| `fixtures/ecdsa-zero/vector.bench.toml` | none |
| `fixtures/keccak-lanes/vector.bench.toml` | none |
| `fixtures/keccak-zero/vector.bench.toml` | none |
| `fixtures/kernel-timestamp/vector.bench.toml` | none |
| `fixtures/merkle-proof/vector.bench.toml` | none |
| `fixtures/plumb-config/vector.bench.toml` | none |
| `fixtures/plumb-v2-card-0/vector.bench.toml` | none |
| `fixtures/plumb-v2-card-1/vector.bench.toml` | none |
| `fixtures/plumb-v2-card-2/vector.bench.toml` | none |
| `fixtures/plumb-v2-card-3/vector.bench.toml` | none |
| `fixtures/plumb-v2-card-4/vector.bench.toml` | none |
| `fixtures/plumb-v2-card-config/vector.bench.toml` | none |
| `fixtures/plumb-v2-coin-0/vector.bench.toml` | none |
| `fixtures/plumb-v2-coin-1/vector.bench.toml` | none |
| `fixtures/plumb-v2-coin-2/vector.bench.toml` | none |
| `fixtures/plumb-v2-coin-3/vector.bench.toml` | none |
| `fixtures/plumb-v2-coin-4/vector.bench.toml` | none |
| `fixtures/quantum-pure/0.bench.toml` | none |
| `fixtures/quantum-pure/1.bench.toml` | none |
| `fixtures/quantum-pure/2.bench.toml` | none |
| `fixtures/quantum-pure/3.bench.toml` | none |
| `fixtures/quantum-ram-1/target-0.bench.toml` | none |
| `fixtures/quantum-ram-2/target-0.bench.toml` | none |
| `fixtures/quantum-ram-2/target-1.bench.toml` | none |
| `fixtures/quantum-ram-3/target-0.bench.toml` | none |
| `fixtures/quantum-ram-3/target-1.bench.toml` | none |
| `fixtures/quantum-ram-3/target-2.bench.toml` | none |
| `fixtures/symmetric/vector.bench.toml` | none |
| `fixtures/timelock-16/vector.bench.toml` | none |
| `fixtures/timelock-17/vector.bench.toml` | none |
| `fixtures/trinity-ascending/vector.bench.toml` | none |
| `fixtures/trinity-descending/vector.bench.toml` | none |
