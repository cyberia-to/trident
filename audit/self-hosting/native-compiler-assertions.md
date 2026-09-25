# Assertions and resolved halting on nox

Source: Trident `eca4ea8c70331f7efd118740655c7e6b8dfb5397`. [Pinned validation](native-compiler-assertions-validation.json)
records commands, sibling revisions and local macOS ARM64 observations.

C1 accepts typed `assert(Bool)` and `assert_eq(Field,Field)` with Unit results.
Success returns atom zero; failure traps through InvZero in the emitted program.
Arguments execute once, left to right. Final ordinary functions retain callable
precedence, including forward and replaced declarations; locals and constants
do not capture the callable namespace.

Only a resolved builtin assert with literal false supplies halting coverage.
Grouped false is preserved. Direct statements, terminal tails and explicit
returns share that rule, including nonterminal branch and loop tails. Ordinary
functions, computed false conditions, unequal assert_eq and let initializers do
not establish coverage. Direct unreachable continuations diagnose, while IF/loop
parents retain the seed's conservative defensive continuation behavior.

The added compiler code initially crossed an existing allocation gate. Block
emission now constructs only its selected tail continuation, and coverage avoids
scanning earlier statements after a fully checked terminal tail covers the block.
All statements and both terminal arms are still type checked before that shortcut.
The original function-depth fixture/source/196608 arena and all public JOB quotas
are retained. Temporary larger-arena diagnostics were removed before acceptance.

```sh
cargo test --release --locked --offline --workspace
python3 audit/self-hosting/run-native-compiler.py --joy ../install/bin/joy --output /tmp/native-guest-assertions-cli.json
```

All 1072 Trident, 122 Joy and 380 Trisha CPU tests pass with zero Rust warnings;
four existing Trisha cases remain ignored. All 133 fixture rows and 43 manual
baselines match the constants delivery. Six new tests exercise typed Unit values,
all result types, final callable shadowing, conservative halting, ordered traps,
once-only subtraction traces, exact caps and independent complete artifact depth.
The native raw Rust seed supplies differential execution and rejection evidence.

The [installed corpus](native-assertions-cli.json) contains 1067
commands and 360 observations. All 160 prior
positive ART1 identities remain identical. Twelve new positive source packages
compile through fresh JOB1 and execute through Joy. Twelve rejected programs
preserve existing outputs; ten well-typed failing programs compile successfully
and then trap without replacing prior runtime output. The CLI started on the
eventual source commit's unchanged production tree. Post-commit installs reproduce
all three binaries and the complete executed C1 artifact. C1 particle:
`604c1ed3729172eacca029982792d99963b07740049ef2af86f3e32ddee17aee`.

Costs from pinned CLI receipts at old source `0051cd5`
and new source `eca4ea8` are charged reductions and lifetime allocations:

| Case | Reductions old → new | Allocated nodes old → new |
|---|---:|---:|
| precedence | 937649 → 937329 | 102389 → 103023 |
| stack64 | 3693287 → 3692967 | 184832 → 185465 |
| body-chunks | 4002744 → 3969185 | 192419 → 192428 |
| record-wide32 | 19670520 → 19669469 | 472965 → 473580 |
| record-typed-call | 2362999 → 2362436 | 130798 → 131413 |
| record-nested | 2714021 → 2713701 | 132564 → 133197 |
| record-long-type | 17198983 → 17198663 | 351789 → 352421 |

Valid wide-record sources, combined long names and the 4096-whitespace workload
retain explicit SH4 resource boundaries. All 84 formal audits are UNKNOWN and
establish no proof. Function attributes, true imports, full SH3/SH4, generated
profiles, C2/C3, six platforms and native Zheng compiler proofs remain open.
Noun temperature stays 128K.
