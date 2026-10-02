# Frozen composite acceptance checker

This delivery preserves the final checker and its source reviews for the local
whole-compiler certificate experiment. The checker requires each generation's
two original controls, nine completed rejection cases from the failed v2 attempt,
and fourteen fresh completion cases. The complete producer, fresh verifier and
all 547 corpus observations per generation remain separate required evidence.

The [source manifest](originals/whole-proof-final-review-v3/sources.json) binds
the thirteen-file checker and watcher closure. It retains the original positive
and corpus checks and derives alternate certificate context from the admitted
JOB1 reductions and frame limits. The [independent review](originals/whole-final-checker-independent-review/independent-review.json)
and [root launch review](originals/whole-final-checker-root-launch-review.json)
refer to exactly that closure. The native proof profile remains production Joy
`6e0ec4d8440e2521df08f442d64f54e667044716`, executable SHA256
`8f42591ece35f192ff6f2328a8360fe0f0959f48a173248b211cd0d8f4d984f9`.

The retained source-manifest SHA256 is
`3bb3470f75daa39c263a0b36cc9b1b9f0a891b19669aaf6aa263fcebbad68e96`;
the independent-review SHA256 is
`4e7c65de21545a6efc1cb0f1aeb898bfad81c8ecb37540d4c122a7a04211a752`;
the root launch-review SHA256 is
`9e1fec7e10835702faf7f9f67c3a10544d7ac9a9de84737ad017f55ce530f134`.

The [independent replay receipt](originals/whole-final-checker-independent-review/final-replay-1/receipt.json)
records the exact Python command, source hashes, 35 passing offline tests,
73 checked source/runtime pins, ten prepared ART1/JOB context comparisons and
32 original completed-command resource replays. Its aggregate-resource and
reclamation checks read the actual retained historical evidence. These checks
preserve the original failed schedule and first refused reclamation attempt.

Historical source snapshots, preparations, offline test runs and initial
failures are retained alongside the final sources. Independent small fixtures
reproduced the original watcher cleanup-error problem and contradictory
schedule timestamps/final child status. Final source rejects those contradictions
and preserves the first failure before recording any cleanup error.

[files.json](files.json) maps every retained file to its explicit original path,
original identity, stored identity and encoding. The retainer reads every stored
file back and compares all decoded bytes with its unchanged original. Text
attributes preserve audit bytes across checkouts. Replay all stored evidence with:

```sh
python3 -B -W error audit/self-hosting/whole-proof-validation/acceptance-v3/check_delivery.py
```

Add `--originals` on the original measurement host to compare every source file
again. The exact retainer command and integration revision are recorded in the
manifest; its source is retained under `originals/final-integration/`.

The package contains frozen preparation and review evidence. Live watcher
orchestration and later acceptance results are excluded. The full-command host
process diagnostic remains local, and Python caches are excluded. This delivery
leaves the whole SH8 outcome pending and makes no durable whole-proof retention
or release publication claim.
