# Actual C2 semantic corpus

Measured on 2026-09-27 from launch revision
`e38658985b569b935675086f9eb328326d222374`. All six unchanged semantic
corpora passed with the compiler emitted by the successful full C1(S) run:

| Corpus script | Observations | Commands |
| --- | ---: | ---: |
| `run-native-compiler.py` | 402 | 1197 |
| `check-guest-constant-linking.py` | 31 | 140 |
| `check-guest-function-imports.py` | 32 | 148 |
| `check-guest-type-imports.py` | 24 | 110 |
| `check-guest-intrinsics.py` | 37 | 157 |
| `check-generated-compiler-profile.py` | 21 | 64 |
| Total | 547 | 1816 |

Each script exited zero and recorded `status: passed`. Expected negative
commands retain their original nonzero outcomes and assertions. The main
corpus took 1361.745549167 seconds; the per-corpus command receipts record
the other elapsed times and exact invocations. Two scripts ran concurrently.

The supplied compiler SHA-256 is
`fe0390b92257edf58686e50571160fc7985b0883ede116c6dcfd0a9f7820b5d0`,
particle `84710d31df5911098b614ee0455d7a199bfe5349e57a0f97e031d52dd533eeb3`.
The pinned Joy SHA-256 is
`506f665b0567cf8d7d669f152153b72dbbbd4520e926a4f47955d2f0bef487b8`.
`inputs.json` binds the producer receipt, compiler, runtime, executed harness
sources and all 94 compiler source files. The launcher verified those 94
source files and the compiler/runtime before and after the complete six-corpus batch.

The launch command, from the isolated Trident checkout, was:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 ../measurements/c2-corpus/run.py
```

The exact launcher is retained as `run.py.gz`. It supplies `--compiler` to
each script, selects `--time-ms 60000` for the main corpus as in its previous
full source-capacity receipt, and preserves all existing vectors and resource
limits. No command builds a replacement compiler. The independent Rust
reference programs remain in the extended corpora with `reference_only:
true`; their raw artifacts are separate comparison oracles. There are 119
such reference builds across the five extended corpora.

Evidence was collected with:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 ../measurements/c2-corpus/collect.py --input ../measurements/c2-corpus --output audit/self-hosting/c2-corpus
```

The exact collector is retained as `collect.py.gz`. `validation.json` records
the original and compressed hashes, verifies provided-artifact bindings,
and compares each case's source and expected result with the previous
source-capacity receipts. The `.json.gz` and `.log.gz` files preserve raw
bytes without whitespace normalization. `generated-artifacts.tar.gz`
retains all 109 generated-profile files, checked against that receipt's
per-file hashes. `summary.json` records all six script exits and final
identity checks.

This evidence covers the actual C2 semantic corpora. Fixed-point equality,
platform builds, graph/capacity component fixtures, proof execution and the
separately measured 64KiB source frame boundary have their own gates.
