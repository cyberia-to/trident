**One compiler, two proof runtimes.** Trident 0.3 separates the language compiler from execution and proving: **Trisha handles Triton and Neptune; Joy handles nox and Zheng**. The three tools form one coordinated release, with corrected program semantics and proofs that bind the computation to its public result.

1. **The compiler and warriors have clear responsibilities.** Trident keeps parsing, type checking, typed IR, nox lowering and the shared extension interfaces. Triton code generation, costs, Neptune libraries and all 43 hand-written Triton baselines now belong to Trisha. This removes target-specific assumptions from the common compiler and puts each runtime's implementation and tests with its owner.
2. **Build, run, prove and verify use the same target selection.** Explicit targets, project settings and the nox default resolve consistently. Trident discovers installed warriors, checks their capabilities and preserves program identity when delegating. This closes cases where a command could select the wrong backend or lose options across the process boundary.
3. **Compiled programs preserve their intended values.** Fixes cover early returns, selected branches, variable shadowing, simultaneous assignment, aggregate layouts, dynamic array bounds and spilled RAM values. Entry signatures enforce input arity and Bool/U32 ranges, including inside proofs. A successful execution or proof must describe the program the source actually expressed.
4. **nox verification binds computation, inputs, output and state.** Joy derives the Zheng execution relation from the canonical program and checks the public result against it. Public execution certificates, private execution proofs and authenticated state reads have distinct formats and explicit guarantees. Earlier trace statements are no longer accepted as proof of execution or output.
5. **Triton and Neptune use the warrior's proof pipeline.** Trisha upgrades to Triton 7, verifies complete expected claims in recursive proofs and validates compiled-lock transaction intents before authenticated submission to Neptune. This keeps VM-specific proof and network rules out of the language compiler.
6. **Formal auditing distinguishes complete checks from unsupported analysis.** Obligations are isolated by function and variable identity, counterexamples and solver errors fail the check, and incomplete analysis reports `UNKNOWN`. This prevents a partial analysis from being presented as evidence that a whole program is correct.
7. **The toolchain is packaged for native use.** The release targets macOS, Linux and Windows on both ARM64 and x64. Every archive contains `trident`, `trident-lsp`, `trisha` and `joy`; runtime resources are embedded, so execution does not depend on the development checkout. The source archive captures the exact locked dependency closure.

**Install:** extract the archive for your OS and CPU, then add `cyber-tools/bin` to PATH. Keep the four executables together. These packages coordinate **Trident 0.3.0, Trisha 0.3.0 and Joy 0.5.0**. Windows binaries statically link the MSVC runtime. Source builds still need Cargo access to the locked upstream dependencies.

<!-- RELEASE_DOWNLOADS -->
| Platform | ARM64 | x64 |
|---|---|---|
| macOS | [Download](https://github.com/cyberia-to/trident/releases/download/v0.3.0/cyber-tools-aarch64-apple-darwin.tar.gz) | [Download](https://github.com/cyberia-to/trident/releases/download/v0.3.0/cyber-tools-x86_64-apple-darwin.tar.gz) |
| Linux (glibc) | [Download](https://github.com/cyberia-to/trident/releases/download/v0.3.0/cyber-tools-aarch64-unknown-linux-gnu.tar.gz) | [Download](https://github.com/cyberia-to/trident/releases/download/v0.3.0/cyber-tools-x86_64-unknown-linux-gnu.tar.gz) |
| Windows | [Download](https://github.com/cyberia-to/trident/releases/download/v0.3.0/cyber-tools-aarch64-pc-windows-msvc.zip) | [Download](https://github.com/cyberia-to/trident/releases/download/v0.3.0/cyber-tools-x86_64-pc-windows-msvc.zip) |
<!-- /RELEASE_DOWNLOADS -->

**Upgrade:** compiler API **3** replaces APIs 1 and 2; update the compiler and warriors together. Regenerate older development proofs and state certificates. Triton baselines and target-specific tooling now live in Trisha. Z3 remains a separate optional dependency for `trident audit --z3`.

**Scope:** this is the default CPU release. Public Joy certificates disclose their execution witness; private proofs hide private inputs, while state tables remain public and bounded. Full GPU proving, dynamic nox continuations and live database integration remain separate work. Neptune wallet operations require the upstream CLI; transaction submission requires the configured authenticated gateway/node.

<!-- RELEASE_VALIDATION -->
**Validation:** all six native targets pass the CPU suites, all 133 execution fixtures, installed proof/certificate smoke and native process/file probes. Every producer's corpus verifies on every consumer: **36 platform pairs, 1,692 checks**, including rejected mutations. The final macOS ARM64 binaries additionally generated and verified **198 fresh baseline proofs**, covering all 43 hand-written programs; this full proof run was measured once on the dedicated 48 GiB worker. The exact released Linux ARM64 client passed admission and rejection checks against the isolated pinned Neptune node.

See [native builds](https://github.com/cyberia-to/trisha/actions/runs/35116488433), [cross-platform verification](https://github.com/cyberia-to/trisha/actions/runs/35125655070), the [validation summary](https://github.com/cyberia-to/trident/releases/download/v0.3.0/release-validation.json), [complete logs, receipts and proof corpora](https://github.com/cyberia-to/trident/releases/download/v0.3.0/release-validation.tar.gz), and [SHA-256 checksums](https://github.com/cyberia-to/trident/releases/download/v0.3.0/SHA256SUMS). These records bind validation to the released source and binary hashes.
<!-- /RELEASE_VALIDATION -->

Source commit: `531e93c4a08bd715a8e8ec61d2089ba7768c6d39`.
