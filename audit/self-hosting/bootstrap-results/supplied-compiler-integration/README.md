# Supplied compiler integration

Trident [PR113](https://github.com/cyberia-to/trident/pull/113), delivery
`a7b74dabc2fa38cdb031e97535d979a766ec1ea0`, prepares all frozen S1 source
bytes for execution by an installed Joy. The helper implementation is
`98c5897aafde1072c692e0c1373d30f40f917e0d`; its source and observed commands
are bound in the [retained supplied-compiler audit](https://github.com/cyberia-to/trident/blob/a7b74dabc2fa38cdb031e97535d979a766ec1ea0/audit/self-hosting/lexer-bootstrap/supplied-compiler/README.md).

The actual `joy run-artifact compiler.dag --input job.dag --emit program`
command processed all 94 modules with the published full compiler limits.
It exited successfully after 1323775018 microseconds and emitted C3 exactly
equal to C2. Every non-time execution field matches the original S1 reference.
The empty command path contained no Rust toolchain or seed compiler. Absolute
Joy was the archived Rust 1.89 build with SHA256
`4035bca898e760666206f445c939c784ea248bff4f82c2c4f0a75254c3c48885`.

The input kit was explicitly a historical rehearsal. This result establishes
the supplied-compiler execution path. Original native matrix acceptance and
production kit assembly retain their separate gates; compilation proofs remain
SH7/SH8 work.

`root-full-source-review.json` records the root's independent comparison of
all 284 archived members against their original files, plus the complete
delivery index. The portable `verify.py` command passed. The delivery also
retains a separate independent reviewer report checking source Git blobs,
runtime identities, exact C2/JOB/C3 bytes, particles and execution fields.

`postdelivery-install.json` records the actual isolated `cargo install --path
. --root <isolated-prefix> --locked --offline --force` after the delivery
commit. Its original stdout/stderr are copied here; installation passed with
zero warnings. This install is a host packaging check and was separate from
the completed guest self-build. `files.json` binds each exact original copy.
