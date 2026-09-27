# First full-source attempt with bounded compaction

Result: budget rejection; C2 was not produced. The full
[receipt](compacting-closure.json) records the exact commands, frozen source
files, manifest, host flags, installed binary/ART1 hashes and failure diagnostic.
[Build inputs](compacting-probe-inputs.json) retain the local revisions and
uncommitted file hashes used by this exploratory worker. Its binary predates
the final nox Enter-only guard clarification and invalid-external-Order
accounting correction; see nox's `audit/sequential-compaction/` for that delta.
These observations are not relabelled as a run of the later committed binary.

The complete frozen source is the `scaled` source committed as `713f457`:
94 modules, 486 functions, 369707 bytes. C1 SHA256 is
`67fd57be4b610d4716fc550c28fba9034782e003a21188c95f12bc03d85c92d3`.
Joy packs the source as JOB1 and executes C1 with compiler profiles (1,1).
No source text is rewritten, no host language stage completes the compilation,
and the output path remains absent after failure.

Declared limits: 10000000000 reductions, 1000000000 cumulative allocations,
3145728 resident nodes, 10000000000 collection-work units, 65536 evaluator
frames, 16777216 validation visits and a 3600000-ms cooperative deadline.
The exact run command and elapsed duration are in the last command receipt.
It returns `execution budget exhausted`; failed charged gas and peak frames
are unavailable from that diagnostic and are not inferred from checkpoints.

| Reported allocation/collection counter | Value |
|---|---:|
| pinned nodes | 177215 |
| cumulative fresh allocations | 122586717 |
| final resident nodes | 845259 |
| peak resident nodes | 3145728 |
| reclaimed nodes | 121741458 |
| completed collections | 42 |
| collection work | 1026471527 |
| scratch bytes | 29360128 |
| evaluator checkpoints | 8243399959 |

The memory bound holds through this failed attempt. The next measurement
localizes module-body checking, function planning and emission with the saved
diagnostic prefix, while preserving this full-compiler rejection. Enlarging a
budget alone would not establish usability, C2/C3 equality or the independent
supplied-compiler semantic corpus.
