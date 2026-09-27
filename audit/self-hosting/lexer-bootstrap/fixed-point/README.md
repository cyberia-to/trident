# Current S1 reaches an exact compiler fixed point

Actual C2 compiled the complete frozen S1 source into C3 through Joy/nox.
The [comparison receipt](comparison.json) passed exact executable byte equality,
producer-chain checks, the complete source inventory and canonical repacking of
both executed JOB1 files. C2 and C3 share SHA256
`76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8`,
particle `2eea2ac5f611012877b4e7291a3a6f534aee7281bb358a0b8e5fabe2ac1f9fbe`
and 9,691,488 canonical bytes. No instruction or metadata was normalized away.

The source is Trident `77213171d39b88c5f41221912251cc4813ac2b11`:
94 modules, 484 functions and 370,544 source bytes. The [first build](../README.md)
retains C1(S1)→C2. The [second receipt](c2-to-c3.json) records actual C2(S1)→C3
with unchanged source/options/limits and pinned Joy SHA256
`506f665b0567cf8d7d669f152153b72dbbbd4520e926a4f47955d2f0bef487b8`.
It invokes no host compiler stage. Rust constructs the initial C1 seed only.

The actual second-build command, run from the isolated `trident-lexer`
checkout at `4bbe399c85a5c5ef50e40791cc5f8da79e929071`, was:

```sh
CARGO_TARGET_DIR=../target-probe PYTHONDONTWRITEBYTECODE=1 python3 audit/self-hosting/probe-native-closure.py --joy ../install-compiler-work-budget/bin/joy --compiler ../measurements/lexer-v9-c1-to-c2-files-astfqc_x/result.dag --inventory ../measurements/lexer-v9-inventory.json --output ../measurements/lexer-v9-c2-to-c3.json --budget 20000000000 --arena-nodes 1000000000 --time-ms 3600000 --validation-visits 16777216 --resident-nodes 3145728 --collection-work 10000000000 --emit program
```

| Second full build measurement | Value |
|---|---:|
| Successful charged reductions | 9,777,538,159 |
| Worker elapsed microseconds | 1,251,723,053 |
| Cumulative fresh allocations | 162,296,944 |
| Peak resident nodes | 3,145,728 |
| Final resident nodes | 2,061,972 |
| Reclaimed nodes | 160,234,972 |
| Collections | 56 |
| Collection work | 1,370,915,937 |
| Peak evaluator frames | 9,254 |

These values come from the successful second execution in `c2-to-c3.json`.
The first and second executions retain distinct artifact directories. C3 is
archived from the second producer's output, despite its byte equality with C2.
Every second source copy and the manifest match both the first-build archive
and the committed source exactly.

```sh
PYTHONDONTWRITEBYTECODE=1 python3 audit/self-hosting/check-selfhost-fixed-point.py --first ../measurements/lexer-v9-c1-to-c2.json --second ../measurements/lexer-v9-c2-to-c3.json --inventory-checker ../target-probe/release/examples/selfhost_inventory --joy ../install-compiler-work-budget/bin/joy --output ../measurements/lexer-v9-fixed-point.json
```

[Artifact identities](artifacts.json) retain the exact checker command, source,
receipts, raw checker log, second JOB1 and independently produced C3. Deterministic
gzip preserves the original bytes. The existing `../sources.tar.gz` retains both
identical source snapshots; original measurement paths stay unchanged.

This receipt establishes the local byte fixed point. Actual C3 semantic corpus
acceptance and clean repetitions on all six native platforms remain distinct
SH6 checks. Native compilation proofs remain SH7/SH8 work.
