# Native bootstrap phase run 36382085561

Status: running; SH6 remains open. This is attempt 1 at Trident
`57491633fbccb58ae44dca2da438ee31430be1bc`. Acceptance requires twelve
original producers, twenty-four original native corpus jobs and the original
successful aggregate. Earlier runs supply none of these phases.

## Retained component snapshot

`components-015/retention.json` binds eleven independently verified producers
and four independently verified corpus phases. All fifteen original GitHub jobs
succeeded. The one remaining producer, twenty remaining corpus jobs and final
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

Both Linux ARM64 repetitions have their original native C2 and C3 corpus phases
verified. Repetition 1 uses artifacts `10956384118` / `10955644380`;
repetition 2 uses `10956711787` / `10956826026` (C2 / C3). Each generation
executed all six corpora: 547 observations and 1816 commands, including
independent reference oracles. Their four restored trees contain 13560 files
and 131127504 raw bytes. These are two complete native repetitions.

The exact commands in the compressed `commands/` drivers restore each store,
compare every restored file byte for byte with its original ZIP, and execute
the frozen producer/corpus checker. Pinned final36 helper functions bind
ZIP/API/store identities and successful distinct jobs. Corpus replays also
compare each copied producer receipt, manifest, selected compiler and native Joy
byte for byte with the separately verified original producer. These component
checks do not invoke or replace final36 acceptance.

The fifteen artifact trees total 20094 files and 899808885 raw bytes; this sums
per-artifact trees, including their shared copies. New raw logs, direct job
responses, imports, current indices and root replays are retained in
`components-015/`. It binds the earlier `producer-009/` and `corpora-002/`
snapshots by digest; their earlier snapshots and original failures remain intact.

## Restore and finish acceptance

These are the exact store indices for this snapshot:

| Store | Entries | Index SHA256 |
|---|---:|---|
| `producer` | 11 | `a917a935f945d9082d74757fc743890e6cfc81ecdf03c46d3b26aba9c0c2e297` |
| `c2` | 2 | `2855ef9435ec770bde694e146ff80898421fa2d136f5bba730998546937081ff` |
| `c3` | 2 | `0cddc9fc28195aac82f26ac78fae60818a7ee1c55f6906ac0cabae75dc950852` |

For example, restore the producer trees with:

```sh
python3 -B audit/self-hosting/bootstrap-results/archive-tool/archive.py restore \
  --store audit/self-hosting/bootstrap-results/run-36382085561/phase-stores/producer \
  --output /absolute/fresh/producer-results \
  --index-sha256 a917a935f945d9082d74757fc743890e6cfc81ecdf03c46d3b26aba9c0c2e297
```

Use each role's store and hash, with a separate fresh destination. Later imports
advance the indices; this commit's stores and the compressed indices under
`components-015/` identify the exact replay snapshot. Store metadata preserves
the original API bytes and ZIP digest. Original ZIP containers remain immutable
in the local download directory recorded by the replay; the stores preserve
all expanded files.

Each store is limited to twelve entries. Retain the original aggregate
separately. Once all results exist, run the
[prepared final validator](final-validation/README.md) against the complete
restored matrix and original GitHub verdict. Pending or failed phases keep SH6
open.
