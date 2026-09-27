# Native function ordering at compiler scale

Plan recorded before implementation at Trident `b453e3444bc5a8ac9c4e7f78910aeab6c59e0581`.
The shared worktree has concurrent, separately owned collection and compiler work.

`module_function_order.sort` currently inserts every final callable into a
persistent sequence. Every shift repeats the complete `(owner, member)` byte
comparison and updates a persistent tree. Replace insertion with bounded,
bottom-up stable merging. Keep the existing comparison, declaration IDs,
capacity diagnostics, sorted prefix on capacity exhaustion and `next` value.
Use the source language's existing bounded loops; introduce no recursion.

Before changing the sorter, add a raw component fixture and an independent Rust
oracle. Exercise prefix-related owner names, equal keys, repeated IDs, odd run
lengths, empty inputs and exact/insufficient capacity. Measure the declaration
names, owner names and declaration order of the complete current source closure,
including exact source bytes. Retain a baseline compiled fixture to compare the
same input against both implementations. These are component measurements;
they do not establish complete C1 execution or C2/C3 acceptance.

Implementation: standalone ordering uses stable bottom-up merging. Packed-word
caching skips equal complete word suffixes while retaining exact span bounds.
The production package path additionally buckets roots by owner, restores their
original order, orders active owners with a sparse sixteen-bit tree keyed by
their admitted package indices, sorts members within each owner and concatenates
the groups. `function_order.sort_roots` performs the stable member merge using
the existing single-source member comparison. Production no longer reaches the
standalone lexical owner comparator or passes owner metadata into each member
comparison. The ordinary `function_order.sort` insertion path is unchanged.
A capacity failure still
returns the same sorted input prefix, diagnostic and processed count.

The rank invariant is established by
[JOB1 admission](../../reference/self-hosting-jobs.md): package module paths are
strictly ASCII-sorted and unique. `examples/selfhost_jobs/validate.rs` rejects
non-increasing module paths; `job.find` already relies on that ordering.
`module_graph.register_open` retains the admitted package index unchanged in
`module_store.Module.index`. The new `sort_ranked` entry consumes those trusted
indices. The existing `sort` entry retains complete lexical comparison.

## Component measurements

These are local combined-worktree measurements on macOS ARM64, not a complete
compiler acceptance receipt. The input source is frozen from
`b453e3444bc5a8ac9c4e7f78910aeab6c59e0581`: 94 modules and 455 declarations in seed
dependency/declaration order. The compact ordering probe keeps every complete
owner/function name and original declaration order, reconstructing only the name
spans in 3444 source bytes. It does not stand in for admission or compilation of
the exact 345791-byte source closure.

[identities.json](function-sort-scale/identities.json) binds every measured
component artifact, the frozen input revision and the final edited source files.
Components were compiled while separately owned collection/runtime changes were
in progress; the formula hashes identify the actual measured programs. The
collection-time heads are recorded there without claiming clean-source release
acceptance. Failed runs are retained.

Each ordering measurement runs this command with the component path from the
identity manifest; `TRIDENT_SORT_RANKED=1` selects a ranked artifact's input ABI:

```sh
TRIDENT_SORT_SOURCE_ROOT=/absolute/path/to/frozen-b453e34 \
TRIDENT_SORT_COMPONENT=/absolute/path/to/component.dag \
CARGO_TARGET_DIR=../target-pipeline-review \
cargo test --release --locked --offline --test native_function_sort -- --ignored --nocapture
```

| Component | Reductions | Lifetime nodes | Peak frames | Result |
| --- | ---: | ---: | ---: | --- |
| [Initial insertion](function-sort-scale/initial-before-compact.log) | 1000000000 allowance exhausted | 12218178 | 3004 | `Halt(0)`, no sorted output |
| [Refreshed insertion](function-sort-scale/refreshed-before-compact.log) | 1000000000 allowance insufficient | 12076930 | 3004 | `Halt(54)`, no sorted output |
| [Global lexical merge](function-sort-scale/merge-compact.log) | 334505284 | 4781708 | 3080 | Exact independent order |
| [Global ranked merge](function-sort-scale/ranked-compact.log) | 129872311 | 4347186 | 2917 | Exact independent order |
| [Owner groups with ranked order](function-sort-scale/grouped-strict.log) | 63417540 | 2513698 | 4609 | Exact independent order |
| [Specialized owner/member ordering](function-sort-scale/specialized-strict.log) | 48335366 | 2323067 | 4606 | Exact independent order |

The last row also passes an explicitly smaller component allowance:

```sh
TRIDENT_SORT_RANKED=1 TRIDENT_SORT_SMALL_ARENA=1 TRIDENT_SORT_BUDGET=100000000 \
TRIDENT_SORT_SOURCE_ROOT=/absolute/path/to/frozen-b453e34 \
TRIDENT_SORT_COMPONENT=/absolute/path/to/specialized-ranked.dag \
CARGO_TARGET_DIR=../target-pipeline-review \
cargo test --release --locked --offline --test native_function_sort -- --ignored --nocapture
```

That run enforces 3145728 arena nodes and 65536 evaluator frames. The preceding
grouped implementation also passed those allowances with the same counters as
its larger-arena run. Neither insertion baseline completed under its allowance,
so no insertion speed ratio is reported. The refreshed baseline recompiles the
old sorter using current collection helpers; its halt retains 54 reductions,
which cannot pay the next operation's cost.

The exact-source versions of the initial insertion and global merge probes both
exhaust 12582912 nodes before returning any ordering, with peak 1842 frames:
[before](function-sort-scale/exact-before.log),
[after](function-sort-scale/exact-after.log). They retain a 1000000000 reduction
allowance. These failures remain source-admission/runtime evidence; they establish
no complete compiler-scale source support. Set `TRIDENT_SORT_EXACT_SOURCES=1`
to retain exact source bytes in the current probe.

## Verification and reproduction

The ordinary test checks complete pair ordering, duplicate-key stability,
repeated IDs, empty/odd sequences, exact and insufficient capacity, all byte
alignments and a reached owner at package index 65535. The Rust oracle uses
stable tuple ordering independently of the guest algorithm. The scale test is
explicitly ignored in ordinary runs because it is a separately budgeted component
measurement; the commands above execute it.

```sh
CARGO_TARGET_DIR=../target-pipeline-review cargo check --locked --offline --test native_function_sort
CARGO_TARGET_DIR=../target-pipeline-review cargo test --release --locked --offline --test native_function_sort -- --nocapture
CARGO_TARGET_DIR=../target-pipeline-review cargo test --release --locked --offline --test native_compiler imported_function_artifacts_ignore_declaration_discovery_and_unused_definitions -- --nocapture
CARGO_TARGET_DIR=../target-pipeline-review cargo test --release --locked --offline --test native_compiler native_constants_keep_previous_default_arena_and_artifacts -- --nocapture
CARGO_TARGET_DIR=../target-pipeline-review cargo test --release --locked --offline --test native_compiler locals_cross_parser_and_emitter_chunks -- --nocapture
```

The focused check is [warning-free](function-sort-scale/check.log). The end-to-end
source-level pipeline regression preserves generated artifacts under reordered
declarations and unused definitions ([log](function-sort-scale/import-artifacts.log)).
The [ordinary sort run](function-sort-scale/semantics.log) passes both tests; the
separate scale test is ignored there and passes when explicitly selected above.
The [saved insertion fixture](function-sort-scale/reference-semantics.log) passes
the same empty, single, stable-pair, zero-cap and prefix-cap oracle cases.

An intermediate grouped implementation made the combined compiler too large
for existing default-arena tests: the parent's retained
[failure log](function-sort-scale/default-constants-grouped-failure.log) observes
the body-chunk case using 196920 nodes against its unchanged 196608-node limit.
It runs the same constants test command above with the parent's `target-probe`
directory, during uncommitted combined-source work after lexer rollback.
Specializing production
owner/member ordering restored both cited gates. The
[constants regression](function-sort-scale/default-constants.log) passes with
196252 nodes for body chunks and 194673 nodes for stack64. The
[locals regression](function-sort-scale/default-locals.log) passes for seven,
eight and nine assignments with 176929, 187610 and 196252 nodes respectively.
These counters are local combined-source observations, not isolated sorter
allocation deltas. No existing quota was raised.

Full owner gates and complete C1/C2/C3 acceptance belong to the parent
integration task.

`TRIDENT_SORT_SAVE_COMPONENT=/path/to/new.dag` retains a freshly compiled fixture.
For a reproducible insertion comparison against the current collection helpers,
extract `lib/std/compiler/nox/module_function_order.tri` from the frozen input
revision into a temporary file and set `TRIDENT_SORT_REFERENCE_SOURCE` to that
file while compiling the unranked fixture. This override is confined to the
diagnostic component; it never substitutes compiler output in a self-build.
