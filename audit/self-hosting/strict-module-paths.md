# Complete qualified paths and visible check diagnostics

Source: Trident `752dc1103119c8d0fe9514a26e7cffc0f7b9d487`; implementation commits `8812c92` and
`ff8b45b`. [Pinned receipt](strict-module-paths-validation.json) records exact
commands, sibling revisions and local macOS ARM64 results.

Every consumed dot now requires an identifier. Imports, calls, fields, constructor
paths and named types reject incomplete paths at the unexpected token, including
EOF. Valid comments and whitespace between path components retain original spans.
The grammar now explicitly reflects already-supported dotted module owners.

`trident check` reports returned compiler diagnostics on stderr before exiting 1.
Acceptance exposed an earlier silent failure at the discovery/parse boundary;
a process-level regression checks both syntax and semantic diagnostics.

```sh
cargo test --release --locked --offline --workspace
python3 audit/self-hosting/check-strict-module-paths.py --trident ../install/bin/trident --joy ../install/bin/joy --trisha ../install/bin/trisha --output /tmp/strict-module-paths-cli.json
```

All 1123 Trident, 122 Joy and 380 Trisha CPU tests pass with zero Rust warnings;
four existing Trisha tests remain ignored. All 133 baseline rows and 43 manual
programs remain unchanged. All 92 formal audits remain UNKNOWN.

The [installed corpus](strict-module-paths-cli.json) executes 35 commands with 12
observations: the valid dotted/commented program returns 14 on both nox and Triton;
ten malformed sources are rejected by check and both warriors. Every rejected
build preserves the previous output file. C1 particle remains
`681c613b8952777de42675bb03941c0af13d80ce255d23a30b2cff6480a0d260` and its complete artifact matches the accepted
[1192-command guest corpus](guest-package-cli.json). That corpus is reused by
byte identity; it was not rerun for this seed/parser/CLI-only delivery.

All-three installs reproduce the tested binaries after source and integration
commits. Guest linking, full compiler language/resources, generated compiler
profiles, C2/C3, six-platform and native Zheng proof gates remain open.
Noun stays 128K; no temperature gate is newly closed.
