# SH2: source compilation inside nox

SH2 is accepted at Trident source revision
`7684fd7e67d3d6610c42553088767844af379f36`, with Joy
`a3dd4c5c7c2f870f6632deace5b137a141796173` and the complete sibling pins in
[validation](sh2-native-compiler-validation.json).

`compiler/nox/main.tri` reads an admitted JOB1, selects the requested source,
validates UTF-8, tokenizes it, parses arithmetic with explicit operator/value
stacks and constructs the executable ART1. Joy executes this compiler and then
loads and executes its result. The complete JOB1 identity binds RES1; generated
program identity depends on the formula and declared profiles.
The [pilot contract](../../reference/self-hosting.md#native-arithmetic-pilot-contract)
defines its grammar, diagnostics, ownership and limits.

## Reproduce

From this repository, with the pinned sibling checkout and installed Joy:

```sh
python3 audit/self-hosting/run-native-compiler.py \
  --joy ../install/bin/joy --output /tmp/native-compiler-cli.json
```

The runner builds C1 once, then creates fresh source files and manifests. Every
subsequent compilation uses `pack-job` and `run-artifact`; the host packages
exact bytes, admits records and publishes complete outputs. The guest performs
the language work. Expected formulas and arithmetic outputs are independent
oracles used only after execution.

[Installed CLI evidence](native-compiler-cli.json) records all 60 commands and
22 observations, including both executions, particles, costs and diagnostics.
C1's particle is
`20f708f4075539e770536c210497d1231b7da4a1cf7e3026eab4264da0ef13f8`.
Its bytes remained identical after the committed rebuild and across all cases.

## Executed acceptance

The following measurements come from that runner on the source revision above:

| Source expression | Result | Compiler reductions | Compiler allocated nodes | Peak frames | Generated-program reductions |
|---|---:|---:|---:|---:|---:|
| `2+3*4` | 14 | 738619 | 31612 | 473 | 5 |
| `(2+3)*4` | 20 | 763382 | 31983 | 493 | 5 |
| A literal inside 64 parenthesis pairs | 1 | 3618126 | 116482 | 956 | 1 |

The independent Rust test corpus compares complete canonical formula artifacts,
Goldilocks arithmetic and Rust-seed execution on identical source. It covers
generated post-C1 expressions, precedence/associativity, leading zeroes, u64
range boundaries, exact whitespace, UTF-8 including comments/chunk boundaries,
reserved names, unused malformed modules, requested profiles, rejected syntax
and unsupported constructs. Package provenance, cfg and runtime limit changes
leave generated program bytes unchanged when the program is unchanged.

The CLI accepts the exact guest admission allowance of 202 visits and rejects
201 after host admission; it also accepts 738619 reductions, 473 frames and
31613 arena nodes, then rejects one less for each independent boundary.
The arena case calibrates the extra input atom introduced by changing its LIM1
value. Failures preserve the previously published program. The next parenthesis
after the pilot stack ceiling produces capacity diagnostic7.

[Gate commands and log digests](sh2-native-compiler-validation.json) record
921 Trident workspace tests and 120 Joy workspace tests with zero Rust warnings.
Trisha verifies 133 fixtures and 43 independent baselines; result/cycle rows
match the preceding delivery. New scalar library entry points have executed
differentials and are registered in the surface census.

## Remaining boundaries

The source ceiling is an algorithmic limit. The installed fixture with 4096
bytes exhausts the independent lifetime arena during guest collection admission,
before its first invalid UTF-8 byte is diagnosed. The fixture with 4097 bytes
returns capacity diagnostic7 before decoding the source collection. The receipt preserves
both observations; full compiler-scale memory belongs to SH4. Earlier failed
drafts and their first errors are retained in the validation record.

Formal audits of the entry and native stage libraries remain UNKNOWN. These
tests establish the specified SH2 pipeline; self-compilation, a complete compiler
closure, fixed-point equality, the six-platform matrix and native Zheng proofs
remain SH3–SH8. Noun's roadmap temperature remains 128K because its 64K milestone
also requires the full generator and self-build. This is local development
evidence on the recorded macOS host.
