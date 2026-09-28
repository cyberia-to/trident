# Native bootstrap phase run 36382085561

Status: running; SH6 remains open. This is attempt 1 at Trident
`57491633fbccb58ae44dca2da438ee31430be1bc`. Acceptance requires twelve
original producers, twenty-four original native corpus jobs and the original
successful aggregate. Earlier runs supply none of these phases.

## Retained component snapshot

`components-034/retention.json` binds all twelve independently verified producers
and twenty-two independently verified corpus phases. All thirty-four original GitHub jobs
succeeded. Two remaining corpus jobs and the final aggregate are required
before matrix acceptance.

| Native producer | Repeat | Original artifact |
|---|---:|---:|
| macOS ARM64 | 1 | 10955342077 |
| macOS ARM64 | 2 | 10955337661 |
| Windows ARM64 | 1 | 10955053500 |
| Windows ARM64 | 2 | 10955530212 |
| Linux ARM64 | 1 | 10954908883 |
| Linux ARM64 | 2 | 10955242264 |
| macOS Intel | 1 | 10957689633 |
| macOS Intel | 2 | 10956721881 |
| Windows x64 | 1 | 10955124998 |
| Windows x64 | 2 | 10955297375 |
| Linux x64 | 1 | 10956472096 |
| Linux x64 | 2 | 10956667089 |

The twelve restored producer trees contain 7128 files and 838706734 raw bytes.
All emit complete C2 and C3 with SHA256
`76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8`.
Their source inventory, JOB1 bytes, compiler identities and every non-time
worker counter match the original S1 references. Both native Cargo builds per
producer have zero warnings. Original elapsed times are preserved unchanged.
Producer receipts retain status `produced`; corpus acceptance is separate.

Eleven native repetitions have both original C2 and C3 corpus phases verified:
both ARM64 repetitions on Linux, Windows and macOS, both Windows x64
repetitions, both Linux x64 repetitions and Intel macOS repeat 2. The original artifact IDs are retained in the store indices and
root replay. Each generation executed all six corpora: 547 observations and
1816 commands, including independent reference oracles. The twenty-two restored
corpus trees contain 74580 files and 717125460 raw bytes.

The exact commands in the compressed `commands/` drivers restore each store,
compare every restored file byte for byte with its original ZIP, and execute
the frozen producer/corpus checker. Pinned final36 helper functions bind
ZIP/API/store identities and successful distinct jobs. Corpus replays also
compare each copied producer receipt, manifest, selected compiler and native Joy
byte for byte with the separately verified original producer. These component
checks do not invoke or replace final36 acceptance.

The thirty-four artifact trees total 81708 files and 1555832194 raw bytes; this sums
per-artifact trees, including their shared copies. New raw logs, direct job
responses, imports, current indices and root replays are retained in
`components-034/`. It binds `components-031/` by digest; the linked snapshots
retain the producer replay and all preceding component checks and indices.

## Restore and finish acceptance

These are the exact store indices for this snapshot:

| Store | Entries | Index SHA256 |
|---|---:|---|
| `producer` | 12 | `51f791f67822bd9ebaf4ee77f04806d10dff592e64a7528dfc985b1d84c0c13f` |
| `c2` | 11 | `6bf96f78eecf1f1b5213cf831f66a8d9ea88d7dfee97f44b764b26cbc6d1477b` |
| `c3` | 11 | `c9440bf27044d665f5b191030c7ff21e8970ba0bf6764f8806209bfe4a9fbf6d` |

For example, restore the producer trees with:

```sh
python3 -B audit/self-hosting/bootstrap-results/archive-tool/archive.py restore \
  --store audit/self-hosting/bootstrap-results/run-36382085561/phase-stores/producer \
  --output /absolute/fresh/producer-results \
  --index-sha256 51f791f67822bd9ebaf4ee77f04806d10dff592e64a7528dfc985b1d84c0c13f
```

Use each role's store and hash, with a separate fresh destination. Later imports
advance the indices; this commit's stores and the compressed indices under
`components-034/` identify the exact replay snapshot. Store metadata preserves
the original API bytes and ZIP digest. Original ZIP containers remain immutable
in the local download directory recorded by the replay; the stores preserve
all expanded files.

Each store is limited to twelve entries. Retain the original aggregate
separately. Once all results exist, run the
[prepared final validator](final-validation/README.md) against the complete
restored matrix and original GitHub verdict. Pending or failed phases keep SH6
open.
