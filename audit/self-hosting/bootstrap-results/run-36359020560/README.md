# Clean native bootstrap: run 36359020560

Partial acceptance: both Windows x64 repetitions pass. This receipt retains
two of the twelve required native repetitions; SH6 remains open until all six
platforms and the aggregate pass. No compilation proof is claimed.

[GitHub run](https://github.com/cyberia-to/trident/actions/runs/36359020560),
attempt `1`, Trident head `c17bd0371c11746f46e20222c48cae2ab08be79d`.
All source pins, Rust `1.95.0`, runner and workflow identities are retained in
the original platform receipts and `windows-x64/expected.json.gz`.

## Windows x64

Both fresh native source/build trees executed `C1(S1) -> C2`, then actual
`C2(S1) -> C3`. Each generated compiler passed all six supplied-compiler
corpora: 547 observations and 1816 commands per compiler. The original raw
Rust programs remain separately labelled reference oracles. Both clean
native Cargo builds in each repetition have zero Rust warning lines.

The frozen S1 inventory, both actual JOB1 inputs, C1 and C2/C3 identities match
the retained local acceptance. All execution-report fields except worker time
also match that reference; this is an observed comparison, not a new platform
acceptance requirement.

| Repetition | GitHub job | Artifact | C1→C2 worker seconds | C2→C3 worker seconds |
|---|---|---|---:|---:|
| 1 | `108732342704` | `10948701623` | 2141.847055 | 1887.928953 |
| 2 | `108732342750` | `10947982650` | 2039.872016 | 1788.497252 |

Times are the actual Joy `execution.execution.elapsed_micros` counters from
the producer receipts. Command wall times, which include process startup and
exit, are retained separately in the independent inspection receipts. Every
executed command and its producer revision remains in the raw platform tree.

C1 SHA-256:
`5728e07a37e111f88166c471ad338f8947b74dfdc842ed1ffa9fa0886c005fda`.
Every actual C2 and C3 SHA-256:
`76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8`.

The two original ZIPs have SHA-256
`107fbeaabecb758e2368136f76e7e7812f53f75b772855ebb12ac5857eb24e82`
(repetition 1) and
`00b4e7d1b6a1bc46904333c2fae7a6312bfacdc71ef62808e293a9308b8c1d21`
(repetition 2). Downloads matched their original API sizes and digests before
parsing. Those immutable ZIP downloads remain in the measurement directory;
the durable store retains their complete uncompressed trees and exact API
metadata, with ZIP provenance.

## Retention and replay

`platform-store/` uses the reviewed [raw artifact store](../archive-tool/README.md).
Its current index SHA-256 is
`6ee39efcd6101916c460df18ac29761921f1ea1cd2ccdb9198e42bb5a6544693`.
The import commands report 7277 file members in each ZIP: 100042334 raw bytes
for repetition 1 and 100042030 for repetition 2. Restored file sets and bytes
matched both original ZIPs. The unchanged frozen runner then checked each
restored tree's complete manifest, compiler chain, fixed point and corpora.

Exact import/restore command arrays and output streams, intermediate index
snapshots, direct job API responses, raw job logs and validation results are
retained losslessly under `windows-x64/`. Its `files.json` binds their stored
and decoded identities. It also retains the initial failed import caused by
a missing parent directory; no artifact was published by that failed command.

`validate-platform.py` supplements the frozen runner with the declared
run/head/attempt, source pins and known local S1 input identities. It explicitly
reports `matrix_acceptance=false`. During review, an omitted reference-row
count check was found and repaired. The actual reference always contained both
rows. Historical source and the synthetic empty-reference false green are
retained under `windows-x64/guards/`; the repaired helper rejects all four
malformed reference inputs and passes the actual original and restored data.

To replay a retained platform, restore this store using its recorded index
hash, decompress the frozen runner and expected-input JSON retained here,
then invoke `validate-platform.py` with the restored platform directory,
those inputs, its explicit target/repetition and a fresh output file. Exact
argument arrays are retained in the validation receipts. The twelve-directory
`bootstrap-runner.py --matrix` invocation remains the final SH6 gate.
