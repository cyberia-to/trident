# Final frozen-S1 self-hosting integration

The [accepted SH8 evidence](../acceptance/README.md) and its companion status
updates are merged into `release/0.4`. The exact source heads, merge parents,
resulting trees and installation commands are retained in
[integration.json](integration.json) and the byte-addressed original records.

| Repository | Reviewed delivery | Accepted merge |
|---|---|---|
| Trident [PR120](https://github.com/cyberia-to/trident/pull/120) | `a6b12e584719b69b64a606c5c4a54d6f6c7ae929` | `942cb5584bc12a3753331829152fa53b83c81341` |
| Joy [PR31](https://github.com/cyberia-to/joy/pull/31) | `c5ededb34b4a9ea82b77c2cb540b81be818750d2` | `65f6a2b7dba1a661adfaed1c37154ac0a825036d` |
| Trisha [PR28](https://github.com/cyberia-to/trisha/pull/28) | `faca656ec1c6e5198433f97f59a2156262da006a` | `f6161e3f6455bd6b4ae8f6a77fa98ac052c1d505` |

Each merge required the exact reviewed head and current integration base,
a ready pull request, successful reported checks, an independently computed
merge tree, and matching final parents/tree and GitHub pull-request identity.
The retained command arrays identify the bounded merge driver and all Git/GitHub
operations. The documentation-only deliveries had no reported CI checks;
their original native acceptance remains attached to its measured revisions.

Trident's reviewed head and accepted merge each passed a fresh-prefix Rust 1.89
`cargo install --locked --offline --force` with zero warnings. Their exact
commands, environments, source revisions and raw logs are retained. Both
installations produced these same binaries:

| Binary | Bytes | SHA256 |
|---|---:|---|
| trident | 9088672 | `b2736abc9956a1c089c1ccaf5159590a5de19f673305d4387da50611f74d6dac` |
| trident-lsp | 6636944 | `c9901902fccc090c07ae7b369ff103989b24e7f2391e1bec343b2e0fdb0ec2bd` |

The commands use the validated warm target cache; native proof workloads were
not repeated for these reporting commits. Companion source-equivalence, exact
link readback and installation evidence are retained in the
[final companion archive](../companion-final-integration/README.md).
The original Joy proof binary and later packaged binary retain their
[explicit source bridge](../joy-source-bridge/README.md).

The [retained file map](retained-files.json) binds exact original metadata and
raw logs. Validate its objects with `python3 -B verify-retained.py`; append
`--originals` where the original local paths remain available. Directory-local
attributes preserve raw bytes, including intentional whitespace in Git output.
Large binary/proof bodies and private process observations retain their
separate accepted evidence locations.

SH0–SH8 are closed for the frozen S1 workload and declared public compiler
profile. The release integration branch, tested package versions and owner
publication decisions retain the scope in the
[release/version review](../../../../release-version-closure.md). Private or
succinct compiler proofs and semantic preservation remain separate roadmap
claims.
