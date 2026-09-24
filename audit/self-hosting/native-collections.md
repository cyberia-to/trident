# Native Seq and Bytes execution

Date: 2026-09-24. Trident source:
`fb7d6962f34c2a7206d5403025928f39eeb004dc`.
Joy: `773151304a9076963e8bd9a771fb28080d3f82af`.
Trisha: `cf89bf3cee97581ba6bd4bae87d42de793045085`.
Full dependency pins, commands and log digests:
[validation receipt](sh1-collections-validation.json),
[installed Joy execution receipt](sh1-collections-cli.json).

## Implementation and boundaries

`std.nox.seq` and `std.nox.bytes` implement the canonical SH0.2 wrappers
in Trident source. Private nominal handles require validated admission.
`std.nox.tree` owns persistent balanced tree operations and iterative validation.
Byte words pack four exact bytes; unused high bytes and empty tree padding must
be zero. Validation charges a shared allowance, including repeated logical
visits to shared nodes. Chunked validation unwinds inner loop frames.

The tree handles height 32 and capacity 2^32 without narrowing the capacity
before descent. Sparse boundary tests cover the final U32 index and persistent
updates. They do not decode billions of occupied elements. Runtime allocation
and frame quotas bound all execution independently of the wrapper visit budget.

The arena retains historical allocations. Outer chunk-driver frames still grow
with the number of chunks. Full compiler memory/runtime capacity remains SH4.

## Validation

Commands ran before the listed source commit; tested code was committed unchanged.
This is local development evidence, not CI or a release candidate.

| Command at the listed revisions | Result |
|---|---|
| Trident `cargo test --workspace --locked` | 907 passed, no failures or ignored tests |
| Trident `cargo check --workspace --all-targets --locked` | Pass, no Rust warnings |
| Joy `CARGO_TARGET_DIR=../trident/target cargo test --workspace --release --locked` | 89 passed, no failures or ignored tests |
| Trisha `CARGO_TARGET_DIR=../trident/target cargo test --release --locked -p trisha-rs` | 362 passed, 4 existing heavy proof tests ignored |
| Trisha `CARGO_TARGET_DIR=../trident/target cargo run --release --locked -p trisha -- bench` | 133/133 fixtures and 43/43 independent baselines verified |

Benchmark result/cycle rows equal the privacy delivery at Trident
`2c66db3cb133ee6b461b7dc9f6e3cb0662a59c54`.
New execution tests compare full particles and complete canonical container
bytes with the independent Rust data model. They cover persistence, pair-valued
leaves, chunk boundaries, exact and insufficient visit allowances, malformed
wrappers, padding, range/cap failures, and private-handle forgery.

`target/debug/trident audit lib/std/nox/{tree,seq,bytes}.tri --json` (run
separately for each file) reports UNKNOWN, exit 2. No formal proof is claimed.

## Installed CLI acceptance

Generate inputs with
`cargo run --locked --example selfhost_collections -- --output-dir /tmp/trident-04-collection-fixtures`.
For each fixture run `../install/bin/joy build MAIN --emit artifact --format json-v1 -o PROGRAM --force`,
then `../install/bin/joy run-artifact PROGRAM --input INPUT -o OUTPUT --force --budget 20000000 --frames 65536`.
The linked CLI receipt records the exact paths, commands, binary digest and outputs
at the source revisions above. Complete output bytes equal the independent model.

| Fixture | Charged reductions | Allocated nodes | Peak frames | Program container bytes |
|---|---:|---:|---:|---:|
| 517 source bytes, validate/update/append | 989077 | 69155 | 1208 | 404832 |
| 129 pair-valued sequence elements, validate/update/append | 249304 | 32887 | 1158 | 268462 |

Both pass with all three observed execution caps set exactly. Lowering each cap
individually by one rejects and preserves the old output even with `--force`.
An insufficient collection visit allowance and nonzero unused byte also reject
without publication. The malformed-byte fixture leaves later operations valid,
so a subsequent bounds error cannot mask broken padding admission.

Reported arena/frame/worker reservations are not peak RSS measurements.
Production Joy JOB1/RES1 admission is the next SH1 delivery.

## Seed frontend findings carried into SH3

- A typed function ending in literal `assert(false)` is normalized into a unit
  return and rejected. Adding a typed fallback is rejected as unreachable.
  Collection validators use `assert_eq(0, 1)` followed by a typed fallback for
  impossible exhaustion. SH3 must make halting/return analysis consistent and
  preserve name resolution when determining whether a call is a builtin.
- `if used == ZERO { } else { ... }` can parse the empty braces after the
  identifier as a struct initializer. The source uses
  `if (used == ZERO) == false { ... }`. SH3 must add a parser regression and
  remove this ambiguity or specify the required parentheses consistently.
