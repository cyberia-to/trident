# Whole self-build certificate attacks

Status: prepared design; no whole certificate has been consumed. Execution waits
for root review and an explicit successful-proof signal. This directory owns all
alternate inputs, helpers, temporary mutations and receipts. The running
`whole-proof` family and the installed production family are read-only inputs.

The targets are the exact accepted complete C1 and C2 self-builds: 94 modules and
370,544 source bytes. Every original proof remains untouched. The root's fresh
original verification remains separate evidence. This suite adds an original
verification control and a correctly rebuilt, byte-identical transport-chain
control for each complete proof.

## Fixed resources

The helper accepts regular files only. Original input cap: 24 GiB wire. Decoded
cap: 96 GiB. Semantic-record cap: 12 billion. Result NOXDAG cap: 16 MiB, 196,608
nouns, depth 4,096. Each transport payload and decoded frame is at most 65,536
bytes. Noun regeneration uses the existing bounded Nox arena constructor and
canonical codec, with no formula evaluator calls.

A single index pass per proof validates the complete public transport and scans
fixed-size semantic records. It retains only the first eligible continuation,
first cache generation, terminal coordinates and complete bounded output noun.
The index binds the exact original file identity and records decoded byte offsets.
Indexing does not check or replace the production semantic verifier.

Mutation construction preserves compressed payload bytes for untouched frames.
It validates original frame digests/order and reconstructs the complete chain
under the selected context. Only frames intersecting changed semantic bytes are
decompressed and recompressed. Output changes regenerate the bounded terminal
suffix. This avoids repeatedly expanding billions of unchanged semantic records.
Original and rebuilt-chain controls must have identical full file SHA256 values.

One whole-proof mutation at a time, across both generations. No full payload is
kept in RAM. A filesystem lock prevents concurrent suite writers. The temporary
directory holds at most one mutation, bounded by 24 GiB plus 65,536 bytes; actual
CLI verification retains the root's 24 GiB wire allowance. No automatic cap
increase. Directory disk guard: 26 GiB; free disk floor: 8 GiB. Before creating a
mutation, require original file size plus 2 GiB free space above that floor.
The exact mutation identity, recipe, helper/source identity and all command logs
are retained before deleting a successfully recorded temporary mutation. Failed
unclassified attempts keep their payload and stop the suite for review.

Index/mutation commands have 1,800-second outer wall and CPU limits and 1 GiB
sampled process-group RSS. Alternate JOB packing has 120-second outer wall/CPU
and 2 GiB sampled RSS. Fresh production verification keeps host time 7,200 seconds,
outer wall/CPU 7,500 seconds, sampled process-group RSS 6 GiB, decoded 96 GiB,
records 12 billion, steps 16 billion and 262,144 cache slots. One-second samples
record wall time, process-group RSS, owned bytes and free disk. Each command gets
a new process group, a hard per-file limit and SIGTERM followed by SIGKILL after
ten seconds if needed. All verifier commands use the exact production Joy binary
and empty PATH. Baseline proof and compiler/JOB identities are checked before and
after execution. All time/RSS/disk measurements remain host observations.

## Inputs prepared before certificate readiness

Copy exact complete manifests and all source bytes into owned variant directories.
For each target generation, use the same complete source graph and pack alternate
JOB1 files with the installed production `pack-job`, without compiling sources:

1. Compiler: use the other accepted compiler and pack the otherwise unchanged job
   against that compiler. Both compiler particles and JOB particles then differ.
2. Entry source: change the first `as_u32(0)` to `as_u32(1)` in `native_compiler`.
3. Dependency source: change the `digit` upper bound `as_u32(58)` to `as_u32(59)`
   in `std.compiler.nox.ascii`.
4. Configuration: add the cfg flag `whole_proof_changed`.
5. JOB limit: lower requested reductions from 20,000,000,000 to 19,999,999,999.

Source edits preserve byte lengths and valid syntax. Full package admission must
succeed, with all other manifest fields retained. Pack results retain module,
source, options and limits identities. The compiler/JOB cases bind an explicitly
different whole request; they do not claim a different request was executed.

## Per-generation execution matrix

Run two positive controls: original certificate and complete rebuilt original
chain. Compare all production verifier semantic/compiler-response fields and
the extracted C2/C3 ART1 identity against the accepted whole-build receipt.

For each of the five alternate requests, first verify the original certificate
against changed expected compiler/JOB coordinates (expect context mismatch), then
rebuild the entire chain under the alternate expected context while retaining
the original semantic records (expect semantic key failure).

Rebuild complete valid chains for these semantic attacks:

- first non-root Enter with different object/formula: replace formula ID by object ID;
- first Reuse: toggle a generation byte;
- terminal charge: add one;
- regenerate a reachable output atom and all canonical noun identities;
- swap unequal children in one reachable output pair and regenerate canonically;
- repeat each valid output change with terminal particle also rebound to its new root;
- remove the semantic terminal while publishing a complete transport terminator.

Output regeneration validates the original noun, constructs the changed reachable
DAG, encodes it, then decodes and re-encodes to identical canonical bytes in a fresh
arena. Unchanged terminal particle must reject at output identity binding. A
terminal changed to the valid altered root must reject at the checked semantic
terminal. These are full self-build outputs, never replacement toy programs.

Framing attacks preserve their deliberate failure: drop the first data frame,
swap the first two data frames, omit the final transport frame, truncate the last
byte, and append one trailing byte. Required rejection is before successful
publication. Every negative starts with a protected destination; success stdout
must be empty, exit must be one, the expected error class must match and the
protected destination must remain byte-identical. Unexpected success stops the
suite and retains all evidence.

Final expected count per generation: 2 accepted controls and 23 rejected attacks
(10 binding, 8 semantic, 5 framing). Preparation and helper fixture tests remain
labelled preparation; full-proof results are recorded only after actual execution.
