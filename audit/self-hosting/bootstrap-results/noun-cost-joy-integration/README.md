# Downstream regression after noun/Cost integration

The selected integration family passes the existing local Joy and Trident
regression suites. This is macOS ARM64 compatibility evidence for the Nox
observer and Zheng noun/Cost component. The frozen native SH6 matrix uses its
own earlier Nox/Zheng pins; its acceptance remains separate.

| Input | Revision |
|---|---|
| Trident | `57491633fbccb58ae44dca2da438ee31430be1bc` |
| Joy | `ec83bd8d85b20a8bd20d2d14b0f25aab0f75e9fe` |
| Nox | `172811b7746cdcd6ab198a3ed976dc6c19f55d5b` |
| Zheng | `633e5ba980db94ca10d9d6b67cf24d9d3aef63a7` |

The other five exact revisions, Git trees, resolved paths and manifests are in
[the original Joy receipt](raw/receipt.json.gz) and
[Trident receipt](raw/trident-receipt.json.gz). Four independent before/after
snapshots contain identical nine clean, detached repositories. Cargo resolved
all 24 local Joy packages inside that real sibling family. No source was edited
for these runs.

## Results

| Command, from its owning repository | Observed result |
|---|---|
| Joy `python3 scripts/check-soft3-boundary.py` | 145 packages, compiler features `[]`, no foreign VM/prover dependency |
| Joy `cargo check --workspace --all-targets --locked --offline` | Passed |
| Same Joy check with `--all-features` | Passed |
| Joy `cargo test --workspace --release --locked --offline` | 172 passed, zero failed/ignored; 27 target summaries |
| Trident `cargo check --all-targets --locked --offline` | Passed |
| Trident `cargo test --release --locked --offline -- --test-threads=4` | 1,197 passed, zero failed, five existing ignored; 46 target summaries |

Both complete test logs independently reproduce those totals. There are zero
Rust warnings. The Joy tests comprise 58 `cyber-joy` and 114 `joy-rs` cases.
Trident includes all 188 native-compiler cases and both lexer-frame cases.
The same test invocation builds 21 release examples; their source and binary
identities are retained, and they add no executed-test count. Check
configurations and empty doctest targets do not inflate these totals.

The five ignored diagnostics are unchanged: alias-suffix footprint, discarded
cast footprint, constant-export-row footprint, function-export footprint, and
native module sorting at full compiler declaration scale. Individual names,
package/target coverage and counting logic are in
[summary.json](raw/summary.json.gz), generated from the original Cargo metadata
and logs. This local integration reused one isolated target directory
sequentially for Joy and Trident; coordinated binary candidates use separate
workspace build directories under their existing release contract.

## Actual solver follow-up

The existing `formal_audit` test can return early when Z3 is unavailable. A
separate focused run therefore records the real Z3 executable, its version and
SHA-256, repeats that one existing test, and directly checks the good and bad
source programs through the compiled Trident CLI with `audit --z3 --json`.

The [accepted focused receipt](z3-verified/receipt.json.gz) records Z3 4.15.4,
SHA-256 `ae6c8df33db9c9ae9a80b6044e77cd66529a141d8b25f0620f1e89b409594f48`.
The good source returned `safe` with exit 0; the false postcondition returned
`unsafe` with exit 1. The focused test passed with zero warnings. It repeats
one of the 1,197 cases and adds zero unique cases.

The [first attempt](z3-first/receipt.json.gz) remains failed: its altered PATH
selected the other installed Rust distribution and replaced the CLI in the
earlier target directory, so its executable-stability check failed. Source
repositories and the original 55 evidence files stayed unchanged. The original
CLI was recovered by exact SHA from an existing `deps` file and copied into the
first attempt's evidence, preserving both observations. The corrected run used
the original absolute Homebrew Cargo/Rustc and a fresh target directory. Its
CLI reproduced the original full-suite binary exactly:
`4160422e299e72ceedcfef29be99a36408273eee94a0fe84d378ed122945de1e`.

## Retention and scope

[retention.json](retention.json) binds all 94 original files to their compressed
copies, with raw and compressed lengths and SHA-256 values. This includes the
original 55-file bundle and its manifest, plus both complete solver attempts
and their manifests. Every compressed member was reopened and byte-compared;
the original full-suite logs were independently recounted. The original
55-file manifest SHA-256 is
`d70817ddb14502b3ed09c0fdf6e9b5f4911c0341f143c54aad4064c6ec944d46`.

Commands, working directories, environment selections, timestamps and exact
source/binary identities remain in their original receipts. This validation
does not repeat the complete guest self-build, supply six-platform release
coverage, or close the dynamic compiler-proof milestones SH7/SH8.
