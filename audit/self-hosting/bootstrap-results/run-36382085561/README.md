# Native bootstrap phase run 36382085561

Status: running; SH6 remains open. This is attempt 1 at Trident
`57491633fbccb58ae44dca2da438ee31430be1bc`. Acceptance requires twelve
original producers, twenty-four original native corpus jobs and the original
successful aggregate. Earlier runs supply none of these phases.

## Retained component snapshot

`components-021/retention.json` binds eleven independently verified producers
and ten independently verified corpus phases. All twenty-one original GitHub jobs
succeeded. The one remaining producer, fourteen remaining corpus jobs and final
aggregate are required before matrix acceptance.

| Native producer | Repeat | Original artifact |
|---|---:|---:|
| macOS ARM64 | 1 | 10955342077 |
| macOS ARM64 | 2 | 10955337661 |
| Windows ARM64 | 1 | 10955053500 |
| Windows ARM64 | 2 | 10955530212 |
| Linux ARM64 | 1 | 10954908883 |
| Linux ARM64 | 2 | 10955242264 |
| macOS Intel | 2 | 10956721881 |
| Windows x64 | 1 | 10955124998 |
| Windows x64 | 2 | 10955297375 |
| Linux x64 | 1 | 10956472096 |
| Linux x64 | 2 | 10956667089 |

The eleven restored producer trees contain 6534 files and 768681381 raw bytes.
All emit complete C2 and C3 with SHA256
`76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8`.
Their source inventory, JOB1 bytes, compiler identities and every non-time
worker counter match the original S1 references. Both native Cargo builds per
producer have zero warnings. Original elapsed times are preserved unchanged.
Producer receipts retain status `produced`; corpus acceptance is separate.

Five native repetitions have both original C2 and C3 corpus phases verified:
both Linux ARM64 repetitions, both Windows ARM64 repetitions and macOS ARM64
repetition 1. The original artifact IDs are retained in the store indices and
root replay. Each generation executed all six corpora: 547 observations and
1816 commands, including independent reference oracles. Their ten restored
trees contain 33900 files and 322912076 raw bytes.

The exact commands in the compressed `commands/` drivers restore each store,
compare every restored file byte for byte with its original ZIP, and execute
the frozen producer/corpus checker. Pinned final36 helper functions bind
ZIP/API/store identities and successful distinct jobs. Corpus replays also
compare each copied producer receipt, manifest, selected compiler and native Joy
byte for byte with the separately verified original producer. These component
checks do not invoke or replace final36 acceptance.

The twenty-one artifact trees total 40434 files and 1091593457 raw bytes; this sums
per-artifact trees, including their shared copies. New raw logs, direct job
responses, imports, current indices and root replays are retained in
`components-021/`. It binds `components-015/` by digest; that earlier snapshot
retains the producer replay and links all preceding snapshots. The intermediate
six-corpus replay and its indices also remain retained.

## Restore and finish acceptance

These are the exact store indices for this snapshot:

| Store | Entries | Index SHA256 |
|---|---:|---|
| `producer` | 11 | `a917a935f945d9082d74757fc743890e6cfc81ecdf03c46d3b26aba9c0c2e297` |
| `c2` | 5 | `568aea9f1681c1f42b4b9735b9ede1f9a9674e78457539b364f1cd066749800e` |
| `c3` | 5 | `c893ee3d6c907710252460f8f9964de23cd8583ad814321a66f8bcdd459b2295` |

For example, restore the producer trees with:

```sh
python3 -B audit/self-hosting/bootstrap-results/archive-tool/archive.py restore \
  --store audit/self-hosting/bootstrap-results/run-36382085561/phase-stores/producer \
  --output /absolute/fresh/producer-results \
  --index-sha256 a917a935f945d9082d74757fc743890e6cfc81ecdf03c46d3b26aba9c0c2e297
```

Use each role's store and hash, with a separate fresh destination. Later imports
advance the indices; this commit's stores and the compressed indices under
`components-021/` identify the exact replay snapshot. Store metadata preserves
the original API bytes and ZIP digest. Original ZIP containers remain immutable
in the local download directory recorded by the replay; the stores preserve
all expanded files.

Each store is limited to twelve entries. Retain the original aggregate
separately. Once all results exist, run the
[prepared final validator](final-validation/README.md) against the complete
restored matrix and original GitHub verdict. Pending or failed phases keep SH6
open.
