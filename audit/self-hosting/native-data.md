# SH0.2 — native data contract and conformance

Date: 2026-09-23. Base Trident revision:
`2894aba992b737fc6cbc37d58d617a8061fb185c`.
Delivery: `feat/0.4-sh0-native-data`, targeting `release/0.4`.
Contract: [native compiler data](../../reference/self-hosting-data.md).
Machine receipt: [sh0-native-data-validation.json](sh0-native-data-validation.json).

## Delivered scope

The contract fixes first-class native Noun semantics, seven proposed intrinsic
signatures, canonical Seq/Bytes layouts, persistent updates, U32 format ceilings,
explicit validation/index limits and error behavior. Language, grammar, nox ABI
documentation and the compiler migration inventory link that same contract.
The existing compiler source inventory has not changed.

The conformance model uses actual `nox::Reduction` nodes and native identities.
It constructs, validates, reads and persistently updates canonical sequences and
packed byte strings. Rust performs the model's traversal, recursion, shifts and
integer division. The model is a contract oracle, not a `.tri` library or compiler
implementation. Its local `Order` handles never define the proposed source ABI.

Small separately constructed formulas execute on the actual nox evaluator with
`NullCalls`: checked projections, pair construction, runtime axis selection,
identity/equality and packed-byte extraction. This demonstrates primitive
feasibility without claiming source-level Noun support, a Joy structured entry
profile, a complete native collection program or Zheng proof coverage.

## Evidence

`cargo test --release --locked --example selfhost_data` passed **18 tests**:
12 model/vector tests and six native VM probes, zero failures/ignored.

| Area | Checked condition |
|---|---|
| Exact byte encoding | Independent expected noun strings; LE packing; exact length/domain; zero, all-ones and partial-word cases |
| Golden identities | Seven checked-in byte vectors with complete four-limb particle byte strings; stable regeneration |
| Construction history | Append equals bulk encoding through byte/word/tree boundaries; each byte-position update equals an independent byte-array oracle |
| Persistence | Prior roots/values survive writes; untouched sibling is shared; failed path copy after two allocations leaves all old values intact |
| Invalid input | Wrong tags/shapes, oversized length/word, noncanonical field value, high-byte padding and compressed/nonzero unused tree padding reject |
| Limits | Empty/out-of-range access, capacity, byte range, zero/exhausted visits and arena exhaustion reject |
| Shared DAG | Repeated occupied children consume visits; sharing cannot bypass validation work |
| U32 ceilings | Word-count arithmetic and sparse height-32 lookup/update/growth avoid overflow; append at maximum length rejects |
| Native checked operations | Pair projection fails on atom; atom projection fails on pair; native cons returns expected pair |
| Native dynamic lookup | Fixed formula accepts different runtime addresses; cost 6; budget 5 halts and 6 succeeds; traced/untraced results and costs agree |
| Native wide axis | Runtime Field axes address height-32 trees, including the final tree position |
| Native identity/equality | `[[1 2] 3]` differs from `[1 [2 3]]`; identity returns all four limbs; equality returns 0=true / 1=false |
| Native byte extraction | All 256 byte values in all four positions execute using AND and field multiplication by inverse constants; non-U32 input rejects |

The sparse maximum-height tests use shared trees constructed by the Rust harness.
They do not validate billions of occupied positions or execute a compiler-sized
workload. A small validation allowance on that logical sequence fails explicitly.
The append-only allocation checks use nox's actual three-quarter load ceiling.

## Reproduce

Use the pinned compatible sibling workspace recorded in the machine receipt.
From the Trident root:

```sh
cargo test --release --locked --example selfhost_data
cargo run --release --locked --example selfhost_data -- --output audit/self-hosting/native-data-vectors.json --check
cargo check --workspace --all-targets --locked
cargo test --workspace --release --locked
cargo run --release --locked --example selfhost_inventory -- --root . --output audit/self-hosting/compiler-subset.json --check
```

Omit `--check` only when intentionally regenerating vectors, then review exact
tree and particle changes. Checking stale output fails without modifying it.
The golden identity is full native particle bytes, four canonical LE limbs;
there is no host arena position or truncated single-limb identity in the vectors.

## Review and limitations

The contract was cross-checked against pinned nox `data/reduction.rs`,
`data/hash.rs`, `patterns/{axis,eq,compose,and}.rs` and scalar/word evaluation in
`reduce.rs`; against Joy's current flat input/output and fixed traced arena; and
against Trident's fixed-width `Ty` / `IntrinsicType` representation. Independent
read-only review found no blocking contract/model mismatch and prompted the
compressed-padding, shared-DAG, exact-budget and partial-allocation failure tests.

Determinism, types, errors and readability were reviewed: stable construction/
vector ordering; explicit tags/ranges; checked errors and persistence on failure;
bounded 32-level tree operations; separate source support and VM/model evidence.
All new Rust files are below the 500-line limit. No production compiler, runtime,
Cargo dependency or version change is included. No foreign-target code changed;
Triton benchmarks, neural features and the six-platform release matrix were not
rerun for this contract/model delivery. Workspace regression coverage is recorded
in the machine receipt; it does not establish self-hosting.

**SH0.2 is complete as a specification/conformance substep.** SH0 remains open.
Next is SH0.3: versioned compiler job/result, complete artifact codecs, canonical
module/package resolution and diagnostic behavior with golden transport vectors.
SH0.4 fixes runtime budgets/control flow; SH0.5 reviews all owner contracts.
Source implementation and Joy integration start at SH1; production native proof
acceptance remains SH7/SH8. No overall roadmap temperature changes at this step.
