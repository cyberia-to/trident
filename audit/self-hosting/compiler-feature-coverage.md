# Compiler inventory to semantic evidence

This is a coverage map for the frozen production-intended source at
`b991d901e6585a40bedd0e0a3d4382c2ad3d89c1`. It maps all 51 inventory feature
kinds to existing positive and rejection cases. The SH3 feature-evidence
criterion is met for this documented subset after review against the source,
canonical subset and named tests. This does not establish SH4/SH5 or C2 corpus
acceptance. The current frozen-source Rust gate passed.

The exact [inventory](body-scale/inventory.json) contains 94 modules,
484 functions, 369,820 source bytes and 8,722 lines. Its SHA256 is
`c121df82df67e1672e83cfc30b94a9161cde551b1be1890ff1e075c070ef2ea9`.
The inventory covers all parsed declarations in the transitive canonical
source closure; it does not infer types, resolve calls or establish reachability.
It was independently rechecked without a build using the following metadata-only
command at the same source revision; the checker binary hash is in the map.

```sh
../target-root/release/examples/selfhost_inventory --root . --entry compiler/nox/main.tri --output audit/self-hosting/body-scale/inventory.json --check
```

The [machine map](compiler-feature-coverage.json) binds all 94 actual source
SHA256 values, their declared inventory BLAKE3 values, 63 named Rust test
references, eight historical installed receipts and 145 selected installed
observations. Counts below come from that inventory and the named receipts,
not a new compiler run. SHA256 checks detect drift; this checker does not
recompute BLAKE3 or revalidate historical artifact bytes.

## What the map covers

Each feature key has its exact count and at least one semantic group. Each
group has positive and rejection Rust tests plus named installed observations.
The map holds complete test names, paths and source hashes; the following
are representative installed case names, qualified by receipt group.

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
are labelled separately. The map's strict log check requires all 188 tests in
that binary and all eight `native_intrinsic_abi` tests to complete unfiltered,
with the selected names passing. It does not infer the exit status of the
whole Cargo command. The [body-scale gate receipt](body-scale/gates.json) records exit 0 for
`CARGO_TARGET_DIR=../target-root cargo test --release --locked --offline -- --test-threads=4`
at the same source revision: 45 suites, 1,195 passed, five ignored, zero failed
and zero Rust warnings. The map binds that receipt plus compressed and
decompressed log hashes. The strict check finds all 63 mapped tests passing.

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
No completed run of these suites using an actual self-produced C2 is retained
here. Provided-compiler routing work is separate from successful execution.

## Explicit limits and pending work

- No broad feature key lacks passing positive/rejection evidence. This meets
  the SH3 inventory criterion for the captured subset; full C1(S) is required
  by SH5 separately. Syntax keys still collapse inferred
  types, nominal owner identities, dotted variable/place paths and interactions.
  They do not prove every source occurrence compiles. The separate
  [complete C1(S) run](body-scale/README.md) now succeeds on this exact source.
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
python3 audit/self-hosting/compiler-feature-coverage.py --rust-log audit/self-hosting/body-scale/full-tests.log.gz --require-rust-pass
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

At `b991d901e6585a40bedd0e0a3d4382c2ad3d89c1`, the third command passed all
12 focused guards; [the log](compiler-feature-coverage.validation.log) is retained.
They reject missing features/cases, changed source/test/receipt hashes, a negative
case substituted for positive execution, missing/newly-called known-only
intrinsics, and absent/incomplete/filtered Rust logs. Synthetic log data tests
parsing only and is never claimed as compiler execution evidence.
