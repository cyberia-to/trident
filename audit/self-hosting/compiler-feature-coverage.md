# Compiler inventory to semantic evidence

This is a coverage map for the frozen production-intended source at
`77213171d39b88c5f41221912251cc4813ac2b11`. It maps all 51 inventory feature
kinds to existing positive and rejection cases. The SH3 feature-evidence
criterion is met for this documented subset after review against the source,
canonical subset and named tests. This does not establish SH4/SH5 or C2 corpus
acceptance. The current frozen-source Rust gate passed.

The exact [inventory](lexer-frame-chunks/inventory.json) contains 94 modules,
484 functions, 370,544 source bytes and 8,731 lines. Its SHA256 is
`d35d263c7f9f27cbe7ea760a34393105ae8140dc6ab84d484151b6fe04960571`.
The inventory covers all parsed declarations in the transitive canonical
source closure; it does not infer types, resolve calls or establish reachability.
It was independently rechecked without a build using the following metadata-only
command at the same source revision; the checker binary hash is in the map.

```sh
../target-probe/release/examples/selfhost_inventory --root . --entry compiler/nox/main.tri --output audit/self-hosting/lexer-frame-chunks/inventory.json --check
```

The [machine map](compiler-feature-coverage.json) binds all 94 actual source
SHA256 values, their declared inventory BLAKE3 values, 65 named Rust test
references, eight historical installed receipts and 145 selected installed
observations. Counts below come from that inventory and the named receipts,
not a new compiler run. SHA256 checks detect drift; this checker does not
recompute BLAKE3 or revalidate historical artifact bytes.

## What the map covers

Each feature key has its exact count and at least one semantic group. Each
group has positive and rejection Rust tests plus named installed observations.
The map holds complete test names, paths and source hashes; the following
are representative installed case names, qualified by receipt group.

Only the lexer differs from the [archived S0 map](body-scale/feature-coverage-s0.json).
All 51 feature kinds and intrinsic declarations remain unchanged. The added
nested loop and direct token records use already mapped loop mutation/return,
qualified constructors, typed constants and checked conversions. Two additional
Rust component tests cover token equivalence and bounded trivia scanning;
they do not add semantic observations to the historical installed corpus.

| Group | Inventory constructs | Positive example | Rejection example |
|---|---|---|---|
| Locals | named/inferred/mutable bindings, variables, assignment | `full:typed-local`, `mutable-snapshot`, `shadow` | `full:unknown`, `immutable-write` |
| Arithmetic | integer literals, Field, binary `+`/`*` | `full:precedence`, `maximum` | `full:overflow`, `scalar-addition-type` |
| Control | Bool, equality, tails, expressions, if/return | `full:terminal-nested`, `early-return` | `full:unselected-type`, `missing-return` |
| Calls | function declarations and calls | `full:function-nested`, `unused-function` | `full:call-arity`, `call-recursive` |
| U32 | U32, masks and unsigned comparison | `full:scalar-mask`, `scalar-comparison` | `full:scalar-comparison-type` |
| Loops | literal range loops without `bounded` annotation | `full:loop-count-5000`, `loop-return` | `full:loop-dynamic-unsupported`, `loop-empty-type` |
| Noun | whole structured values and entry ABI | `full:noun-input`, `noun-swap` | `full:noun-entry-parameter`, `noun-write-type` |
| Digest | identity and dynamic indexed reads | `full:digest-identity`, `digest-u32-index` | `full:digest-wrong-index` |
| Tuples | tuples, typed/destructured bindings and tuple assignment | `full:tuple-nested-types`, `tuple-swap` | `full:tuple-immutable-target` |
| Records | nominal declarations/values, fields and persistent writes | `full:record-nested`, `record-write-nested` | `full:record-nominal-mismatch` |
| Arrays | literal-size Field arrays and indexed reads | `full:array-field-u32`, `array-loop` | `full:array-length-type`, `array-element-write` |
| Constants | final typed constants and imported literal provenance | `constants:foreign-index`, `final-binding` | `constants:replaced-invalid` |
| Modules | program/module headers, direct imports, public visibility | `callable:private-helper`, `types:full-short` | `types:transitive-type-hidden` |
| Contracts | requires/ensures lexical metadata | `full:attribute-token-metadata` | `full:attribute-unbalanced` |
| Purity | declaration-local source-name check | `full:attribute-pure-helper` | `full:attribute-untaken-error` |
| Intrinsics | intrinsic metadata and declaration-only functions | `intrinsic:complete-noun-values` | `intrinsic:wrong-parameter`, `intrinsic-with-body` |

The three mandatory SH3 regressions are explicit observations: `typed-local`
executes `let x: Field = 7 x`; `function-nested` executes a two-parameter call;
`unused-function` executes main returning 9 beside helper returning 7.
These also appear in the mapped Rust locals/functions tests.

The map separately names module-graph ordering and original-source diagnostics,
complete bounded symbol names, ABI admission, once-only argument evaluation and
runtime traps. These supplement the feature keys, whose presence alone cannot
show those properties. A runtime trap is distinguished from source rejection.
The long-source Rust test compares emitted bytes and diagnostics; its scope is
compilation-only, not another emitted-program execution.

## Evidence lanes

Rust `tests/native_compiler.rs` builds C1 through the Rust native backend, runs
that artifact inside nox, then checks guest output. Positive tests selected for
the semantic groups execute emitted programs; some also compare the Rust seed
or independent formulas. Component tests run host-built guest fixtures and
are labelled separately. The strict log check requires all 188 tests in that binary, all eight
`native_intrinsic_abi` tests and both `native_lexer_frames` tests to complete
unfiltered, with all 65 selected names passing. The [complete split gate](lexer-frame-chunks/full-gate.json)
retains the actual commands: one invocation for the compiler and lexer targets,
one for the remaining library/binary/integration targets and one for doctests.
Their 46 primary targets contain 1,197 passing tests and five existing ignores,
with zero failures and Rust warnings. A separately identified lexer rerun adds
two passing executions after review of newline-at-chunk-end vectors; the
unique test count remains 1,197.

The checker derives target coverage from retained Cargo metadata, verifies
command selectors and terminal summaries, binds retained test-source revisions,
and checks each raw log plus their exact concatenation. The recorded execution
base remains `0fe4054` with explicit source hashes; committed source `7721317`
was subsequently verified against all 94 source blobs. The gate records both
identities. No single Cargo invocation is invented for the combined result.

The following installed receipts all used historical Rust-seeded C1 SHA256
`4aed7fc83be96156fcb65c3bbb369c192ad27f894ab78a030e3588056a66d112`
and Joy SHA256
`3408767cd09f1c4ede2c44e9d99d402810fc1dbdf8c4be6866f8b3d06c7e75f3`.
Their exact commands, sources and execution records are retained in each JSON.
The map binds receipt hashes and checks selected observations' compiler
particle, success/error status, emitted-program execution particle or preserved
previous output. It does not claim those old temporary artifact files survive.

| Runner | Receipt | Observations | Scope |
|---|---|---:|---|
| `run-native-compiler.py` | [full](source-capacity-full-cli.json) | 402 | Core semantics, runtime traps and boundary observations |
| `check-guest-constant-linking.py` | [constants](source-capacity-constants-cli.json) | 31 | Imported constants and provenance |
| `check-guest-function-imports.py` | [callables](source-capacity-callable-cli.json) | 32 | Imported functions, visibility and ordering |
| `check-guest-type-imports.py` | [types](source-capacity-types-cli.json) | 24 | Imported nominal types and complete values |
| `check-guest-intrinsics.py` | [intrinsics](source-capacity-intrinsic-cli.json) | 37 | Intrinsic resolution, ABI and lowering |
| `check-guest-module-graph.py` | [graph](source-capacity-graph-cli.json) | 12 | Graph component; no emitted program |
| `check-generated-compiler-profile.py` | [profile](source-capacity-profile-cli.json) | 21 | Generated bounded fixture compiler/profile transport; no complete C2 |
| `check-source-capacity.py` | [capacity](source-capacity-capacity-cli.json) | 7 | Source admission and diagnostics |

These counts are observations with different scopes, not a count of independent
semantic programs. The main 402 observations exclude the other seven runners.
These historical C1 receipts keep their original identities. The earlier S0
actual [C2 corpus](c2-corpus/README.md), [C3 corpus](c3-corpus/README.md) and
[fixed point](fixed-point/README.md) are separate completed measurements.
The current S1 scanner source requires its own bootstrap and supplied-compiler
evidence; this feature map does not infer that acceptance from S0.

## Explicit limits and pending work

- No broad feature key lacks passing positive/rejection evidence. This meets
  the SH3 inventory criterion for the captured subset; full C1(S) is required
  by SH5 separately. Syntax keys still collapse inferred
  types, nominal owner identities, dotted variable/place paths and interactions.
  They do not prove every source occurrence compiles. The separate
  [earlier complete C1(S) run](body-scale/README.md) describes S0. The
  [progress ledger](../self-hosting-progress.md) tracks S1 acceptance separately.
  Historical barriers remain documented in the
  [frontier](full-bootstrap-frontier.md) and [compacting run](full-bootstrap-compacting/README.md).
- `requires`/`ensures` mean balanced lexical metadata in this native subset.
  Tests intentionally allow false or ill-typed predicate text and preserve
  executable artifacts. They prove neither predicate enforcement nor SH7 proofs.
  `pure` checks documented source-member names locally, without transitive proof.
- All 15 actual VM intrinsic declarations are mapped individually. Exact VM
  source tests execute ten lowerable identities. `split`, `field_add`,
  `field_mul`, `neg` and `inv` have unused-declaration acceptance and reachable
  rejection cases. There are no syntactic calls with those final member names in this
  inventory; this is not a general semantic reachability proof. ABI component
  tests check every registered signature and malformed parameter/return shapes.
- `loop.no_explicit_bound` records absence of a `bounded` annotation. The source
  loops use literal ranges; their large execution cost remains a distinct gate.
  Field arrays with literal extents are the captured array subset. Generics,
  cfg/test attributes, XField and public/secret IO declarations are absent from
  this inventory; negative cases do not establish positive support for them.
- Run the relevant distinct semantic runners with the actual C2 published
  by the complete C1(S) run and bind
  the compiler's start/end hashes. Complete C2(S), compare canonical C2/C3 bytes
  and particles, then repeat and exercise CI. The fixed-point checker and
  supplied-compiler runner options prepare those gates; they do not close them.

## Rechecking and guard tests

From the Trident repository root:

```sh
python3 audit/self-hosting/compiler-feature-coverage.py
python3 audit/self-hosting/compiler-feature-coverage.py --rust-log audit/self-hosting/lexer-frame-chunks/full-rust.log.gz --require-rust-pass
PYTHONDONTWRITEBYTECODE=1 python3 -W error audit/self-hosting/compiler-feature-coverage.test.py
```

The [retained strict check](compiler-feature-coverage.validation.json) records
all selected suites passing against the immutable compressed full-gate log.
The first command checks mapping integrity only. The second requires completed
Rust suite evidence; SH4/SH5, C2 acceptance and fixed point remain separate. There is no build,
compiler invocation, fallback, artifact mutation or receipt overwrite. A changed
inventory or current source/test hash fails until its coverage is reviewed.
`--inventory PATH` supports an independently generated future inventory; it must
match the reviewed hash. Do not merely refresh that hash to bypass new features.

With the above S1 source bindings, the third command passed all 28 focused
guards; [the log](compiler-feature-coverage.validation.log) is retained.
They reject missing features/cases, changed source/test/receipt hashes, a negative
case substituted for positive execution, missing/newly-called known-only
intrinsics, absent/incomplete/filtered Rust logs, missing or duplicate primary
targets, failed subcommands, changed log bytes, unbound source changes, absent
committed-source verification and reruns incorrectly counted as unique tests.
Synthetic log data tests parsing only; actual gate logs establish execution.
