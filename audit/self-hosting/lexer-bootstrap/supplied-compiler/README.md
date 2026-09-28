# Supplied-C2 frozen-source preparation and self-build

This local rehearsal prepared all 94 frozen S1 modules (370544 bytes), packed
them twice from relocated source trees, then ran the supplied C2 once with the
third installed Rust-1.89-built Joy. C3 is exactly the supplied C2: 9691488 bytes,
SHA-256 `76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8`.
The package remains explicitly `rehearsal`. This supplies no accepted-kit
positive, replacement native matrix, corpus verdict or SH7/SH8 proof claim.

## Source and runtime binding

Helper/source unit: `98c5897aafde1072c692e0c1373d30f40f917e0d`, based on
`57491633fbccb58ae44dca2da438ee31430be1bc`. Helper SHA-256:
`4f2381880f7da77265ed562cb99bc0e7bcad4c5f7df28f8c62bf7a37f55c3711`.
All 94 source bytes and all 627 tracked Rust, Trident and Cargo inputs equal
that base. See retained `preparation/portability/source-integrity.json`.
A standalone matching Trident checkout/source archive suffices; preparation
also passed on a directory containing only the 94 source files, relocated to
a path containing spaces and Unicode. Coordinated archive packaging is not a
consumer dependency. The runtime path needs Python, Joy and verified C2/kit
metadata plus those source files, with no Rust toolchain or Trisha/Neptune
installation.

The guide was adapted from existing Trident guide commit
`82fdd1fd45fd14f3ac8da60a53d9e1a3e675c18e`; kit sample paths replace audit links
absent at base 57491633. Later guide wording and a WinError-1314-only symlink
test skip do not change the helper or guest inputs. Capture records retain
actual dirty-source identities at execution rather than relabeling them with
a later documentation commit.

Installed Joy: 5233200 bytes, SHA-256
`4035bca898e760666206f445c939c784ea248bff4f82c2c4f0a75254c3c48885`.
The retained `runtime-provenance/` records bind the original third candidate's
committed source closure, Rust 1.89 and exact binary. The source Trident is
57491633; Joy is `adb424023ae11a2264567d21171c3c2b3aa06f64`, with Nox
`172811b7746cdcd6ab198a3ed976dc6c19f55d5b` and Zheng
`633e5ba980db94ca10d9d6b67cf24d9d3aef63a7`. The executable remains external at
the exact path/length/hash in `receipt.json`; its raw bytes are not duplicated.

## Observed commands and limits

The full argv, cwd, environment, exit status and raw stdout/stderr are retained
for five preparation commands: default rejection of the actual rehearsal kit
(exit 1), two explicit rehearsal preparations and two `joy pack-job` calls
(exit 0). Both 6762002-byte JOB1 files have SHA-256
`3474e5583e7b3ac36bdd526435bb2ae584691774a009e29ca02407c22589f13d`.
The source contents, package manifest, admission packages and JOB1 bytes are
identical across relocation. No source parser or compiler is invoked by the
preparer. The exact manifest is in both retained prepared trees.

The subsequent single command was the guide's `joy run-artifact compiler.dag
--input job.dag --emit program --output c3.dag`, with absolute input/output
paths recorded in `execution/receipt.json`, and these unchanged explicit flags:

```text
--arena-nodes 1000000000 --budget 20000000000 --frames 65536
--time-ms 7200000 --validation-visits 16777216
--resident-nodes 3145728 --collection-work 10000000000
```

Packing and execution used an empty executable PATH and absolute Python/Joy
paths; Cargo, rustc and Trident were unavailable on PATH. `/bin/ps` sampled
RSS every 30 seconds without participating in compilation. There was one
guest run, no retry, host seed fallback or limit change. Source bytes, C2,
JOB1 and Joy were checked before and after execution.

Joy returned exit 0 and compiler-job success after 1323775018 us.
It charged 9777538159 reductions,
allocated 162296944 cumulative nodes and
reached 9254 frames. Sampled process-tree
RSS peaked at 438848 KiB; this is a sampled observation,
not an RSS bound. C3 equals C2 byte for byte. Admission compiler/job particles
match execution program/input particles, and every non-time execution field
matches the original S1 C2-to-C3 record. Only `elapsed_micros` is excluded.
The historical record used a 3600000 ms host deadline; this command used the
published 7200000 ms deadline. The compiler/job/runtime counters are otherwise
compared exactly. Both raw receipts remain retained.

## Local checks and retained history

There are 21 distinct helper guards and 46 existing bootstrap routing guards.
The 21 helper guards passed in ordinary and optimized Python; repeated runs
are not additional distinct tests. The final local portability repetition
also passed all 21 with zero skips on this Mac. It does not claim execution
on Windows. Symlink creation skips only explicit Windows privilege error
1314; all unexpected errors remain failures. The actual Mac case-insensitive
path guard executed. Actionlint and diff checks passed. Postcommit isolated
`cargo install --path . --locked --offline --force` and version check passed
with zero warnings; its nine clean source identities, exact argv and raw
logs are retained under `postinstall/`. No Rust or compiler source changed.

The original preparation run and first final capture are preserved separately
from corrected checks. Review led to physical case-alias protection and helper
identity binding before the final packing/run. Later corrections preserved
existing reference-section topology and removed an English-only missing-file
assertion. Independent review and root cheap-pack review are retained as raw
receipts; their timestamps/source hashes are not rewritten as later runs.

The final documentation/audit commit intentionally uses `[skip ci]` to avoid
rerunning the unchanged 36-phase matrix for transport-only changes. This is
not a green CI result for that head. Original matrix acceptance belongs to
its own original receipts and pinned 57491633 input; see the
[self-hosting ledger](../../../self-hosting-progress.md). Source unit 98c5897 added three exact workflow `paths-ignore` entries for
the preparer, its test and its fixture; changing the workflow file itself
would still trigger that workflow without the explicit skip marker. No
repository Actions settings, required checks or branch protection were
altered. GitHub documents the HEAD-marker behavior in [Skipping workflow runs](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/skip-workflow-runs).

## Retention and offline verification

`evidence.tar.gz` retains 284 named raw files totaling 44102285
bytes; identical bytes use ordinary tar hardlinks. Gzip/tar timestamps and
owners are zero. `files-*.json` lists each original path, raw SHA-256 and size.
Every archived member was byte-compared with its original. The original
measurement files remain unchanged. Raw log whitespace was not normalized.

```sh
python3 verify.py
```

This checks every retained byte plus receipt/input/output/particle/counter
bindings without running a compiler. It is an evidence check, not a fresh
guest execution or acceptance authority. The executable is not needed for
this verification. To repeat the actual guest run, use the unchanged prepared
compiler/JOB1 and an independently verified installed Joy with the published
flags, fresh destinations and retained exit/log evidence.
