# Native bootstrap phase run 36382085561

Status: running; SH6 remains open. This is attempt 1 at Trident
`57491633fbccb58ae44dca2da438ee31430be1bc`. Acceptance requires twelve
original producers, twenty-four original native corpus jobs and the original
successful aggregate. Earlier runs supply none of these phases.

## Retained component snapshot

`components-027/retention.json` binds eleven independently verified producers
and sixteen independently verified corpus phases. All twenty-seven original GitHub jobs
succeeded. The one remaining producer, eight remaining corpus jobs and final
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

Eight native repetitions have both original C2 and C3 corpus phases verified:
both ARM64 repetitions on Linux, Windows and macOS, plus both Windows x64
repetitions. The original artifact IDs are retained in the store indices and
root replay. Each generation executed all six corpora: 547 observations and
1816 commands, including independent reference oracles. Their sixteen restored
trees contain 54240 files and 517862185 raw bytes.

The exact commands in the compressed `commands/` drivers restore each store,
compare every restored file byte for byte with its original ZIP, and execute
the frozen producer/corpus checker. Pinned final36 helper functions bind
ZIP/API/store identities and successful distinct jobs. Corpus replays also
compare each copied producer receipt, manifest, selected compiler and native Joy
byte for byte with the separately verified original producer. These component
checks do not invoke or replace final36 acceptance.

The twenty-seven artifact trees total 60774 files and 1286543566 raw bytes; this sums
per-artifact trees, including their shared copies. New raw logs, direct job
responses, imports, current indices and root replays are retained in
`components-027/`. It binds `components-025/` by digest; the linked snapshots
retain the producer replay and all preceding component checks and indices.

## Restore and finish acceptance

These are the exact store indices for this snapshot:

| Store | Entries | Index SHA256 |
|---|---:|---|
| `producer` | 11 | `a917a935f945d9082d74757fc743890e6cfc81ecdf03c46d3b26aba9c0c2e297` |
| `c2` | 8 | `c320b4c8387b905cc5d4402ebae34952cc36d672bdae435315b9e18324622b96` |
| `c3` | 8 | `6d74790ba0152e3e7f49949264d439b556cf359c523beee2910a81aad1e89122` |

For example, restore the producer trees with:

```sh
python3 -B audit/self-hosting/bootstrap-results/archive-tool/archive.py restore \
  --store audit/self-hosting/bootstrap-results/run-36382085561/phase-stores/producer \
  --output /absolute/fresh/producer-results \
  --index-sha256 a917a935f945d9082d74757fc743890e6cfc81ecdf03c46d3b26aba9c0c2e297
```

Use each role's store and hash, with a separate fresh destination. Later imports
advance the indices; this commit's stores and the compressed indices under
`components-027/` identify the exact replay snapshot. Store metadata preserves
the original API bytes and ZIP digest. Original ZIP containers remain immutable
in the local download directory recorded by the replay; the stores preserve
all expanded files.

Each store is limited to twelve entries. Retain the original aggregate
separately. Once all results exist, run the
[prepared final validator](final-validation/README.md) against the complete
restored matrix and original GitHub verdict. Pending or failed phases keep SH6
open.
