# Native bootstrap phase run 36382085561

Status: running; SH6 remains open. This run is attempt 1 at Trident
`57491633fbccb58ae44dca2da438ee31430be1bc`. Its acceptance requires twelve
original producers, twenty-four original native corpus jobs and the original
successful aggregate. Earlier runs supply none of these phases.

## Retained producer snapshot

The `producer-009/` snapshot independently restores and verifies these nine
completed producer components. Each original GitHub job is successful; every
producer receipt remains `produced`, which establishes the self-build phase.
Corpus jobs and final matrix acceptance remain separate.

| Native target | Repeat | Original artifact |
|---|---:|---:|
| macOS ARM64 | 1 | 10955342077 |
| macOS ARM64 | 2 | 10955337661 |
| Linux ARM64 | 1 | 10954908883 |
| Linux ARM64 | 2 | 10955242264 |
| Linux x64 | 1 | 10956472096 |
| Windows ARM64 | 1 | 10955053500 |
| Windows ARM64 | 2 | 10955530212 |
| Windows x64 | 1 | 10955124998 |
| Windows x64 | 2 | 10955297375 |

`producer-009/retention.json` binds the complete nine-producer replay and links
the unchanged eight-producer snapshot, which retains the earlier seven. Together they index original command logs, direct GitHub
job responses, expected-input contract and root replay. The source-bound
commands in `commands/replay-phase-producers-009.py.gz` invoke the retained
archive tool, compare every restored file byte for byte with its original ZIP,
and execute the exact frozen producer checker. The final36 validator's pinned
helpers additionally bind original ZIP/API/store identities, successful distinct
jobs, both S1 steps and every non-time execution field. This component check
does not invoke or replace the final36 acceptance gate.

The nine restored trees contain 5346 files and 627897023 raw bytes. All
producers emit the same complete C2 and C3, SHA256
`76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8`.
Their source inventory, JOB1 bytes, compiler identities and all non-time worker
counters match the original S1 references. Both native Cargo builds per
producer have zero warnings. Original elapsed times are retained unchanged.
The earlier seven-producer snapshot and its four-producer replay remain under
`producer-007/`.

## Restore and finish acceptance

The producer store snapshot has index SHA256
`b3ddb6e387768046c0237ff6e0c9ab00017a35687a8c671f19c119c6e10ccdd1`:

```sh
python3 -B audit/self-hosting/bootstrap-results/archive-tool/archive.py restore \
  --store audit/self-hosting/bootstrap-results/run-36382085561/phase-stores/producer \
  --output /absolute/fresh/producer-results \
  --index-sha256 b3ddb6e387768046c0237ff6e0c9ab00017a35687a8c671f19c119c6e10ccdd1
```

Later imports advance the store index. The compressed original index remains
in `producer-009/producer-index.json.gz`; use this commit's store snapshot to
replay the command above. Store metadata retains the original API bytes and
ZIP digest. Original ZIP containers remain immutable in the local download
directory recorded by the replay; the store preserves all expanded files.

Remaining artifacts are retained in separate `producer`, `c2` and `c3` stores,
each limited to twelve entries. The original aggregate is retained separately.
Once all results exist, run the [prepared final validator](final-validation/README.md)
against the complete restored matrix and original GitHub verdict. Pending or
failed phases keep SH6 open.
