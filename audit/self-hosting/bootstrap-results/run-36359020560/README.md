# Clean native bootstrap: run 36359020560

Partial acceptance: macOS ARM64, Linux ARM64 and both Windows architectures
pass, with two clean repetitions each. This receipt retains eight of the
twelve required repetitions; Linux x64, Intel macOS and the aggregate remain
required. SH6 stays open. No compilation proof is claimed.

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

## Linux ARM64

Both clean native Linux glibc repetitions pass with the same C1, actual C2/C3,
S1 inventories and JOB1 bytes as Windows and the local reference. Each C2 and
C3 passes all six corpora (547 observations / 1816 commands); both clean Cargo
builds per repetition have zero Rust warnings. All non-time execution fields
match the local S1 reports. The unchanged runner also compares all four
retained Windows/Linux reports successfully; this remains a partial matrix.

| Repetition | GitHub job | Artifact | C1→C2 worker seconds | C2→C3 worker seconds |
|---|---|---|---:|---:|
| 1 | `108732342692` | `10947959238` | 2304.321239 | 2045.023197 |
| 2 | `108732342674` | `10948841523` | 2303.901358 | 2053.309206 |

Original ZIP SHA-256 values are
`a4f6d29e1488640caa1a5b3674946a388be0731dbeb29cc89bb9f56dbe5822d6`
and `7d12850b510d0d1298ae6e0f36f03186975b8d038a69eb7883e02dc98c659c6d`
for repetitions 1 and 2. Each retains 7277 files: 99902961 and 99902951 raw
bytes respectively. Restored files matched the original ZIPs exactly and
passed the frozen runner again. `linux-arm/files.json` binds the original
command logs, direct GitHub job responses, worker measurements and independent
inspection, including the four-report comparison.

## macOS ARM64

Both clean native repetitions pass with the same S1 inventory, JOB1, C1 and
actual C2/C3 identities as the retained local reference. Each actual C2 and C3
passes all six corpora (547 observations / 1816 commands); both native Cargo
builds in each repetition have zero Rust warnings. All non-time execution
fields match the local S1 reports.

| Repetition | GitHub job | Artifact | C1→C2 worker seconds | C2→C3 worker seconds |
|---|---|---|---:|---:|
| 1 | `108732342677` | `10948746458` | 2363.548077 | 1849.651108 |
| 2 | `108732342696` | `10949130160` | 2391.814686 | 2032.208015 |

Original ZIP SHA-256 values are
`a9f051d06df289108c0d89f46a324441aa2f5c196c84c291091a546fb6a8203d`
and `64172d08f8cd4e589b1f8bca9f48c07d4abe51f96aa64d118fc381c196b62982`
for repetitions 1 and 2. Each retains 7277 files: 99905168 and 99905187 raw
bytes respectively. Restored file sets and bytes match the original ZIPs
exactly; the frozen runner and supplemental S1 checks pass on those restored
trees. `mac-arm/files.json` binds the raw command logs, direct GitHub job
responses and independent inspection.

## Windows ARM64

Both clean native repetitions pass with the same S1 inventory, JOB1, C1 and
actual C2/C3 identities as the retained local reference. Each actual C2 and C3
passes all six corpora (547 observations / 1816 commands); both native Cargo
builds in each repetition have zero Rust warnings. All non-time execution
fields match the local S1 reports.

| Repetition | GitHub job | Artifact | C1→C2 worker seconds | C2→C3 worker seconds |
|---|---|---|---:|---:|
| 1 | `108732342678` | `10948457612` | 2328.567972 | 2075.316887 |
| 2 | `108732342643` | `10949180243` | 2319.241064 | 2073.750926 |

Original ZIP SHA-256 values are
`c0d56d37c48eab5c806a0d5a2ae19ff13948c5a15b3be25d6bff08375c16d17b`
and `676d15b9009fbe0b9cb962da2b91a4e4c8ea621fc220304e29797ec0757ed10c`
for repetitions 1 and 2. Each retains 7277 files: 100045655 and 100045653 raw
bytes respectively. Restored file sets and bytes match the original ZIPs
exactly; the frozen runner and supplemental S1 checks pass on those restored
trees. `windows-arm/files.json` binds the raw command logs, direct GitHub job
responses and independent inspection.

## Retention and replay

`platform-store/` uses the reviewed [raw artifact store](../archive-tool/README.md).
Its current index SHA-256 is
`a051ca5805a7fa980d8911a24dcbd494b25f89bdcd5a93f32ccc79330b55e189`.
The Windows import commands report 7277 file members in each ZIP: 100042334 raw bytes
for repetition 1 and 100042030 for repetition 2. Restored file sets and bytes
matched both original ZIPs. The unchanged frozen runner then checked each
restored tree's complete manifest, compiler chain, fixed point and corpora.

All eight restored reports also pass the unchanged runner’s cross-platform
comparison. This is a partial comparison; the twelve-report matrix gate
remains required.

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
