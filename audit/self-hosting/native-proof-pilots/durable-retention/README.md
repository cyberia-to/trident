# Durable SH7 evidence retention

The complete [accepted SH7 pilot archive](../README.md) is retained as ordered
assets on Trisha's existing draft `389977897`, `candidate-20260916.1`.
All parts and the manifest passed server-digest checks and independent downloads.
Ordered reconstruction reproduced the original complete archive. The final
observed release state remained unpublished and its candidate Git tag absent.
SH8 still requires the complete self-build certificate acceptance gate.

This delivery follows Trident acceptance commit
`bbcd7e455af1a92c6f2bb971dd3470325fad922f`; it copies completed evidence without
rerunning compilation proofs or changing proof bounds. The original proof
commands used Joy `6e0ec4d8440e2521df08f442d64f54e667044716`, as recorded in the
[acceptance audit](../README.md). [Provenance](provenance.json) records the
original path and exact identity of every copied file. Absolute paths and
chronological wording inside original receipts remain unchanged.

## Archive and transport

The complete archive is `sh7-native-c2-complete-evidence-20261002.tar.gz`,
602191451 bytes, SHA256
`fa0f99785b4f499067683949a4126a1e4837558fbd421c0c4d61fedc39000d61`.
The [remote manifest](remote-parts-manifest.json) specifies all offsets,
part lengths, SHA256 values, asset IDs and API URLs. Four parts contain
134217728 bytes each; the last contains 65320539 bytes.

The uploaded manifest is asset
[604791641](https://api.github.com/repos/cyberia-to/trisha/releases/assets/604791641),
4763 bytes, SHA256
`9bc033f4dffb8e248a55b445db884ca035886705923f2c7a596daa45914febac`.
Draft access requires repository authorization. The API asset route retained
in each manifest entry supports authenticated downloads; publication was not
part of this transport.

[The actual transport receipt](transport-receipt.json) records every command,
its output hashes and all six asset identities. The source-reviewed runner
used at most four concurrent transfers and a fixed 7200-second transport
deadline per command. These are transfer limits; proof limits stayed fixed.
[Independent complete replay](transport-review.json) checked all 64 exact
commands, 128 transport outputs and 11 preparation logs, rehashed each local
and downloaded part, and reconstructed the full original archive.

[Root's separate replay](root-review/receipt.json) passes the same complete
checks; [its command](root-review/command.json) names the exact source and
archive paths. The [first root invocation](root-review/initial-invocation.json)
used a wrong archive basename and was refused before payload verification.
The corrected invocation changed no source or evidence.

[transport-audit.tar.gz](transport-audit.tar.gz) retains 235 original small
files in 138492 bytes, SHA256
`9db8e0f08be770423748ca5d376a02be261a22c62d209902a4b2eb92db81de47`.
[Its manifest](transport-audit-manifest.json) names every archived member;
[the packaging result](package-command-result.json) records the exact command.
It includes the reviewed transport/checker sources, complete raw API logs,
original stop metadata and failed single-file upload receipt, part manifest
and independent check receipts. Large original/split/downloaded payloads
remain outside this compact package and are explicitly mapped to the remote
assets. Every member was re-read and hashed again during this delivery.

## Actual acceptance-commit installations

Both [first](postinstall/attempt-1/receipt.json) and
[corrected](postinstall/attempt-2/receipt.json) receipts are the original
postcommit installations of `bbcd7e455af1a92c6f2bb971dd3470325fad922f`.
Their recorded `cargo install --path . --root … --locked --offline --force`
commands used actual Rust 1.89.0 on `aarch64-apple-darwin`, explicit compiler
paths and separate fresh installation prefixes.

The first command exited 0 but failed the zero-warning gate because the new
installation directory was absent from PATH. Its [original warning](postinstall/attempt-1/install.stderr)
is retained. The second added that prefix to PATH and passed with no warnings;
its [original log](postinstall/attempt-2/install.stderr) shows the reused warm
Cargo target. This is a clean source checkout and fresh installation prefix,
with a reused compilation cache. Both attempts produced the same Trident
binary SHA256 `9c96e52e0548e64835599f72d9b32be4f54e81daeaa0e6f06659ac7be12429e6`.
The receipts retain both installed binary identities; binaries are not copied
into this small audit delivery.

## Local retention check

From the repository root:

```sh
python3 -B audit/self-hosting/native-proof-pilots/durable-retention/check_retention.py
```

This verifies every copied identity and compact-archive member, ordered remote
manifest bindings, the independent replay receipts and both install outcomes.
It performs no new network download or proof execution. The original complete
replay command and source remain preserved in the compact package.

[Follow-up validation](delivery/README.md) retains the complete successful
Rust workspace run, earlier interrupted attempts, source identities and
documentation checks for this audit-only delivery.
