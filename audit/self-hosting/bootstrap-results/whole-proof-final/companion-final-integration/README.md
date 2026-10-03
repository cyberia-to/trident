# Accepted companion SH8 integration

[Joy PR31](https://github.com/cyberia-to/joy/pull/31) and
[Trisha PR28](https://github.com/cyberia-to/trisha/pull/28) merged the final
frozen-S1 acceptance links into `release/0.4`. Joy's accepted merge is
`65f6a2b7dba1a661adfaed1c37154ac0a825036d`; Trisha's is
`f6161e3f6455bd6b4ae8f6a77fa98ac052c1d505`.

The exact [original packet](packet/README.md) records the reviewed heads,
merge trees, authenticated published-file readback and installation commands.
Each reviewed head and accepted merge passed a fresh-prefix offline Rust 1.89
installation from its source-equivalent accepted archive path and warm cache.
All installations retained the accepted binary bytes without warnings,
compilation, source changes or target-size changes. Their receipts preserve
the distinction between the original Joy proof binary and the packaged Joy
binary. No native proof workload was repeated for these documentation changes.

The [delivery review](support/review.json) and
[independent root review](root-validation/review.json) retain the postmerge
checks. [copy-inventory.json](copy-inventory.json) binds every copied packet,
support and root-validation file to its exact original path, size and SHA256.
The inner packet and its maps remain byte-identical to the reviewed originals.
Run `python3 -B -W error packet/verify-retained.py --originals` to check retained
objects and their locally available originals; omit `--originals` for a
portable archive-only check.

The first root text-format probe raised an assertion when an original Markdown
proposal began with a link. Its [failed observation](root-validation/format-probe-failure.json)
is retained. The corrected review read that exact proposal and the two exact
Git commit outputs as plain text; all other files used the frozen strict
privacy check. The original proposal remains inert, and no collector source
or original bytes changed.

The [cross-repository integration record](../integration-final/README.md)
connects this packet to Trident's accepted SH8 evidence. Version, default-branch,
tag and public-release decisions retain the existing owner policy.
