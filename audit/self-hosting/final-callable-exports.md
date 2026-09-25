# Final callable ownership across seed compiler stages

Source: Trident `87f464f73c4a3e60c70002241cf7fc804bffccbf`. [Pinned validation](final-callable-exports-validation.json)
records commands, sibling revisions and local macOS ARM64 evidence.

Previously a replaced public declaration could retain an export after its final
binding became private or changed ABI/kind. Generic specialization selected an
earlier body; TIR, entry metadata and test enumeration could select different
declarations. Every consumer now selects the last active declaration per name,
retaining surviving source order. Earlier active ordinary bodies still typecheck;
generic bodies are checked when instantiated. Syntactic test discovery remains
available; execution follows the final active annotation and counts cfg exclusions.

The positional generic-call queue also depended on traversal order and skipped
bodies. The shared map now keys by function and original callee byte span. Explicit
and inferred calls use their checked instantiation even when comparison lowering
visits operands in another order. Direct TIR rejects missing bindings/unemitted
instances before emission. Nested raw generic calls require the prepared compiler
API. Concrete return metadata and body AST use canonical specialization, preserving
nominal owner constants and instantiated aggregate widths.

```sh
cargo test --release --locked --offline --workspace
python3 audit/self-hosting/check-callable-exports.py --joy ../install/bin/joy --trisha ../install/bin/trisha --trident ../install/bin/trident --output /tmp/final-callable-exports-cli.json
python3 audit/self-hosting/check-callable-tir.py --output /tmp/final-callable-tir-validation.json
```

All 1097 Trident, 122 Joy and 380 Trisha CPU tests pass with zero Rust warnings;
four existing Trisha tests remain ignored. All 133 fixture rows and 43 manual
baselines match the attributes receipt. The new compiler coverage comprises
12 unit/direct-TIR tests and seven integration tests.

The [installed CLI receipt](final-callable-exports-cli.json) records 75 commands
and 24 observations: 13 positive projects execute on Joy and Trisha, six rejected
projects preserve prior outputs, and five test-selection cases execute through
Trident's nox test entrypoint and Trisha. Joy has no test subcommand.
The [direct TIR receipt](final-callable-tir-validation.json) records 15 actual
Triton VM executions from checker-produced source-site bindings. It bypasses
PreparedProject's generic rewrite to test the direct builder's own boundary.

Post-source-commit installs reproduce all three executed binaries. The complete
C1 artifact is identical to the accepted attributes compiler; its existing full
JOB1/ART1 corpus remains applicable by exact artifact identity. That full corpus
was not rerun for this seed-only change. C1 particle:
`6ee70c5d760a2c01844ac738b08e3ce6977f5ca525d14f9b20829cf26e1e0048`.

All 88 formal audits remain UNKNOWN. Explicit import scopes, true guest modules,
full source/resource scale, generated profiles, C2/C3, six platforms and separate
native Zheng compiler proofs remain open. Noun stays 128K.
