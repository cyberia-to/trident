# Actual S1 C3 semantic acceptance

All six supplied-artifact corpora passed: 547 observations and 1,816 recorded
commands. These runs used the actual C3 emitted by the retained second build C2(S1),
SHA256 `76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8`,
particle `2eea2ac5f611012877b4e7291a3a6f534aee7281bb358a0b8e5fabe2ac1f9fbe`.
`binding.json` ties that exact file to `../fixed-point/c2-to-c3.json` (SHA256
`9479e0899f4aeeee089bfc0fd53ab0d9767fde8f50de8d5cbfff55cda515e8fa`)
and `../fixed-point/c3.dag.gz`. The compiler path in every receipt is the
second producer output, `lexer-v9-c2-to-c3-files-jb8je2q4/result.dag`; C2 was
not substituted despite the two artifacts having identical bytes.
All 94 captured source hashes match Git source
revision `77213171d39b88c5f41221912251cc4813ac2b11`.

The launch revision was `4bbe399c85a5c5ef50e40791cc5f8da79e929071` in
`trident-lexer`; that revision includes the source and full Rust gate evidence.
The pinned Joy binary is `install-compiler-work-budget/bin/joy`, SHA256
`506f665b0567cf8d7d669f152153b72dbbbd4520e926a4f47955d2f0bef487b8`,
from Joy `2878f4b17dfedf237c6110d7d411bb4824e65103`.
`inputs.json` preserves source and harness hashes, actual paths and launch state.

| Harness in `audit/self-hosting/` | Observations | Commands | Raw reference builds |
| --- | ---: | ---: | ---: |
| `run-native-compiler.py` | 402 | 1,197 | 0 |
| `check-guest-constant-linking.py` | 31 | 140 | 31 |
| `check-guest-function-imports.py` | 32 | 148 | 32 |
| `check-guest-type-imports.py` | 24 | 110 | 24 |
| `check-guest-intrinsics.py` | 37 | 157 | 28 |
| `check-generated-compiler-profile.py` | 21 | 64 | 4 |
| Total | 547 | 1,816 | 119 |

The exact six commands, working directories, start/end times and exit codes
are retained in `summary.json` and each `*-command.json`. Every harness used
`--joy` with the pinned binary, `--compiler` with the actual emitted C3 path,
and a fresh `--output` receipt. The main harness explicitly used
`--time-ms 60000`. The two-worker launcher is preserved as `run.py.gz`.
All compiler selections remained in provided-artifact mode. There were zero
compiler seed builds or replacements. The 119 independent Rust builds use
`--artifact-profile raw`, are explicitly reference-only, and never supply the
compiler or its emitted result.

The collector compared case order, source bytes, expected results and boundary
labels with the historical corpus receipts. `verify.py` additionally verified
all 1,695 `pack-job`/`run-artifact` resource-option sequences and 639 fixed JOB1
limit objects against those receipts. The existing adaptive boundary tests
recalibrate reductions, frames and arena nodes for the selected compiler;
the three exact-limit objects therefore differ from the earlier compiler.
Their exact and one-below tests are checked against this run's measured
baseline/calibration and preserve the prior output. No fixed quota was raised
and no corpus vector was removed.

The exact raw receipts and logs are deterministically compressed without text
normalization; `validation.json` records compressed and raw hashes. The
109-file `generated-artifacts.tar.gz` retains the generated-profile inputs,
JOB1 files and actual outputs, with its per-file hashes in
`generated-profile.json.gz`. `verify.py` checks every archive member, receipt
hash, producer binding, Git source blob, historical vector and resource limit.

The archival command was run from the Trident repository:

```sh
python3 ../measurements/lexer-v9-c3-corpus/collect.py --input ../measurements/lexer-v9-c3-corpus --output audit/self-hosting/lexer-bootstrap/c3-corpus
python3 audit/self-hosting/lexer-bootstrap/c3-corpus/verify.py
```

The collector is retained as `collect.py.gz`; its output directory must be
fresh. The verifier can be rerun against the retained evidence and Git history
without running a language stage.

This is actual local S1 C3 semantic acceptance. The completed C2→C3/fixed-point
validation is retained separately in `../fixed-point/`.
Clean bootstrap, platform execution, proofs, and separate component graph and
capacity checks have their own evidence. No completion claim for those gates
is made here.
