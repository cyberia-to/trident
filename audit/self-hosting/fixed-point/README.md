# The complete compiler reaches C2 = C3

Actual C2 compiled the same complete source into C3 through Joy/nox. The
canonical artifacts are byte-for-byte identical. The [comparison](comparison.json)
passed source inventory checks, canonical repacking of both actually executed
JOB1 files, producer-chain checks, and exact executable byte/particle comparison.
The [C2 semantic corpus](../c2-corpus/README.md) also passed all six runners.
This establishes a usable self-produced compiler and its byte fixed point on
the reference host. Full SH4/SH6 acceptance still requires the valid 64 KiB
scanner repair, actual C3 corpus and clean six-platform reproduction.

The frozen source is Trident `b991d901e6585a40bedd0e0a3d4382c2ad3d89c1`:
94 modules, 484 functions, 369,820 bytes. The
[first build](../body-scale/README.md) records the Rust seed and C1(S) → C2.
The [second receipt](c2-to-c3.json) records actual C2(S) → C3, with the same
Joy SHA256 `506f665b0567cf8d7d669f152153b72dbbbd4520e926a4f47955d2f0bef487b8`,
source/options/limits and NoTrace profile. It invokes no host compiler stage.

From the isolated Trident checkout, the second command was:

```sh
CARGO_TARGET_DIR=../target-probe python3 audit/self-hosting/probe-native-closure.py --joy ../install-compiler-work-budget/bin/joy --compiler ../measurements/body-scale-closure-20b-files-wnfaub83/result.dag --inventory ../measurements/body-scale-inventory.json --output ../measurements/c2-to-c3-20b.json --budget 20000000000 --arena-nodes 1000000000 --time-ms 3600000 --validation-visits 16777216 --resident-nodes 3145728 --collection-work 10000000000 --emit program
```

| Second full build measurement | Value |
|---|---:|
| successful charged reductions | 9,771,339,293 |
| worker elapsed microseconds | 1,324,851,112 |
| cumulative fresh allocations | 162,260,313 |
| peak resident nodes | 3,145,728 |
| final resident nodes | 2,041,574 |
| reclaimed nodes | 160,218,739 |
| collections | 56 |
| collection work | 1,370,939,038 |
| peak evaluator frames | 9,262 |
| canonical C3 bytes | 9,681,676 |

These numbers come from that successful execution and the exact revisions
bound in the first-build receipt. C2 and C3 share SHA256
`fe0390b92257edf58686e50571160fc7985b0883ede116c6dcfd0a9f7820b5d0`
and particle `84710d31df5911098b614ee0455d7a199bfe5349e57a0f97e031d52dd533eeb3`.
The comparison removes no instructions or metadata. C1 remains different,
as permitted by the bootstrap contract.

```sh
PYTHONDONTWRITEBYTECODE=1 python3 audit/self-hosting/check-selfhost-fixed-point.py --first ../measurements/body-scale-closure-20b.json --second ../measurements/c2-to-c3-20b.json --inventory-checker ../target-probe/release/examples/selfhost_inventory --joy ../install-compiler-work-budget/bin/joy --output ../measurements/body-scale-fixed-point.json
```

[Artifact identities](artifacts.json) bind this checker command/source and all
retained bytes. Deterministic gzip archives preserve the actual second JOB1 and
C3. Every second-build source file and its manifest were byte-compared with the
first-build copies; the existing source archive therefore retains both exact
snapshots. Historical paths in receipts remain unchanged. Rechecks write fresh
receipt paths and require the recorded files and prebuilt tools.

This is local execution evidence. Compilation proofs remain SH7/SH8 work;
the [source-size frame failure](../source-scale-compacting/README.md) remains
visible and is being repaired separately.
