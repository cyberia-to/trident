# Native compiler: Field locals and persistent frames

Source: Trident `6edc4c198d41e20f2dc246666b932050187be832`, Joy `a3dd4c5c7c2f870f6632deace5b137a141796173`,
Trisha `908d5e22a669e4aa0b6c02b5e9134e67c1a25675`, nox `5271961a72a1f1e9922df36e3e4e3c65d1acff81`.
[Exact sibling inputs and commands](sh3-native-locals-validation.json) are local
development evidence. The source compiler now accepts inferred or explicit
Field declarations, mutable assignments, local reads and same-scope shadowing.

Initializers resolve before installing their new binding. Every declaration
owns a stable runtime slot; assignment reuses it and evaluates its right-hand
side once. Thus `let mut x=7 let y=x x=9 y*100+x` produces 709. The guest compares
full identifier spans, so long names with equal prefixes stay distinct.
Literal and parenthesized statement tails follow the Rust seed, including LF
separating an identifier from a following parenthesis; CR alone retains a call.
Unsupported equality is recognized before assignment tokenization.

The guest creates a postorder expression arena and ordered writes, then emits
a balanced native frame. Its shape comes from actual declarations. Requested
limits cannot change a successful program's bytes; the previous arithmetic
artifacts retain their exact independent golden formulas. Both parser and
emitter return explicitly between bounded chunks. The
[subset contract](../../reference/self-hosting.md#native-compiler-subset-contract)
defines syntax, diagnostics, runtime slots and limits.

## Execution acceptance

The installed compiler owner was rebuilt from the source revision above before:

```sh
python3 audit/self-hosting/run-native-compiler.py \
  --joy ../install/bin/joy --output /tmp/native-locals-cli.json
```

[The CLI receipt](native-locals-cli.json) records 109 commands and
39 observations. C1 is built once, before the fresh source
corpus is created. Joy packages exact source, runs compilation inside nox, saves
ART1 and independently executes that artifact. Accepted cases cover declarations,
dependent values, mutable snapshots, shadowing, long identifiers and parser/emitter
chunk transitions. Unknown/self-read names, immutable writes, malformed declarations
and unsupported constructs return diagnostics and preserve previous output.
Sequence caps are exercised below and at the required expression-record count;
permissive caps reproduce the same artifact. The existing shared-admission,
reduction, frame and arena boundaries are rerun against this compiler.

C1 particle: `4c9d5f5b9383bf5758afe18c99fe3111f890ef7af66a13a719e8b9968dd34f77`.
Measurements below come from that installed command and source revision;
nodes are lifetime allocations, not live memory.

| Case | Generated result | Charged reductions | Allocated nodes | Peak frames |
|---|---:|---:|---:|---:|
| precedence | 14 | 819755 | 41676 | 473 |
| typed-local | 7 | 924341 | 38776 | 565 |
| mutable-snapshot | 709 | 1590896 | 55790 | 653 |
| body-chunks | 9 | 3448420 | 111554 | 710 |

The complete gates passed 946 Trident workspace tests,
120 Joy workspace tests and 380 Trisha CPU tests
(4 existing ignored cases), with zero Rust warnings. Trisha verified
133 fixtures and 43 independent manual baselines; all result/cycle rows match
the preceding halting delivery. The receipt names each command and revision.

A separate native generator component probe measures emitted DAG depth
independently, then checks the exact RES1/ART1 depth and one less. It uses
`tests/fixtures/native_codegen_depth.tri` through the real nox evaluator.
This isolates output-depth checking: full JOB admission also applies the limit
to C1, so a small output limit is rejected before guest execution. That component
probe is distinct from the complete source/JOB corpus above.

## Remaining work

The installed 31-increment-assignment job and the full source-ceiling workload
still fail at runtime with Unavailable and preserve the previous program.
The configured arena allowance is 196608; earlier draft Rust probes record its
exhaustion explicitly. A larger full-JOB output-depth experiment also exceeded
the arena. Failed drafts and their resolutions remain in the validation receipt.
This is the SH4 compiler-scale resource gap, not successful compilation evidence.

This closes the Field-locals slice of SH3. Next are Bool, equality, scoped
if/else and early return, then typed reusable functions and the remaining
compiler language. Whole-module compilation, C2/C3 self-build, six-platform
acceptance and native Zheng proofs remain open. All native compiler modules
and its entry retain UNKNOWN formal verdicts; no proof claim follows from
these execution tests. Noun's roadmap temperature remains 128K.
