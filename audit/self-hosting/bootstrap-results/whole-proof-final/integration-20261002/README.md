# Accepted companion integration — 2026-10-02

This packet retains the actual `release/0.4` integrations and their reviewed
fresh-prefix installations. It preserves each original command, receipt and raw
stream as a SHA256 object. It adds no native proof run or SH8 acceptance.

| Delivery | Reviewed head | Merge | Exact integration receipt |
| --- | --- | --- | --- |
| Trisha PR25 | `5a8e386aa1228c6c37603811f1cfda219d281665` | `fbea3cef9a4139075e529c319ff75488ed5df625` | [receipt](objects/b8f6de1815f832e5dc7ef384f6777293cf9bbaf76ca2c169a6054c47a9a08dbd) |
| Trisha PR26 | `0b8d784fbc49bba55c37bcdee8851e35072ab413` | `9e1af832543a21def306dac84ff918cbb823653d` | [receipt](objects/90ea6991336664be3d57858f185aa802bc4b54fd29b25cb6214a7c45b548c08d) |
| Joy PR29 | `5a38ea8416c07df506c844a8ea2a95d33a93818a` | `15202f42240ba923398908ee0db62d32f1d655f8` | [receipt](objects/af696ec188b1d5cc859e77204f8721d060506f5cbafdf8be5af666dacaa877d7) |

Trisha PR25 delivers accepted complete certificate byte-equivalence retention.
Its remote body readback, local observations and earlier failed adopter keep
their original provenance; SH8 adversarial/final-checker acceptance is separate.
Trisha PR26 corrects current package/retention pointers and dates the historical
pre-release index. Joy PR29 records the implemented public dynamic compiler
profile and opt-in host ceiling in Unreleased notes. Product versions remain
unchanged. Default branches, tags and public promotion are outside these merges.

## Actual source-equivalent warm installations

Each command used the accepted archived source paths and existing Cargo target
after checking equality with the corresponding committed delivery inputs. The
fresh installation prefix received the exact observed binary; this does not
claim compilation from a relocated documentation checkout. Explicit native
Rust, Cargo and rustdoc 1.89 tools were authenticated. Original guards reject
compilation/warnings and enforce 120 seconds, a 10 GiB initial free-space check,
an 8 GiB runtime floor and at most 64 MiB target growth. Sources are checked again
afterward. The original drivers and exact absolute argv remain in the receipts.

| Delivery | Original receipt | Elapsed seconds | Installed SHA256 |
| --- | --- | ---: | --- |
| Trisha PR25 | [receipt](objects/e20bab6e82c310a5d0dea7eba50770f06f427b4642e1465212ba97b1aeb78ccc) | 0.6082377910497598 | `fe069879a44b582a47069c5cd849b1eb3da4c75a037e72aa9d107742ecc8173f` |
| Trisha PR26 | [receipt](objects/1707eed58a44ed12d977e6c5ba44bf5b9694014d955f4d83e012be4f8f12f467) | 0.7709584590047598 | `fe069879a44b582a47069c5cd849b1eb3da4c75a037e72aa9d107742ecc8173f` |
| Joy PR29 | [receipt](objects/b610c719918522d3a6333cc4cc971930e9e6123dbdc29b13b11d6b365eec17b1) | 0.2460536250146106 | `7b1aee370f6826db73f089054f261a3ef874ad54d2e259fbd26f7d8164ab6071` |

The [first Trisha PR25 attempt](objects/5a09456135574b8f6e47e41be60be8fff675739fe5c8b5154edd990b94fb37f8) remains failed: its original
Cargo PATH warning prevented guarded install acceptance. The separate corrected
attempt passed with its own original logs. No earlier status was overwritten.

The [first Joy root check](objects/6fd03846a8485b692bd8a65fa022d06ee0c63b2740c6b609cb3d5ec17fbc8593) incorrectly required exact target-directory size
equality. The actual target changed from 345000177 to 345000145 bytes, shrinking
by 32 bytes. The [accepted follow-up check](objects/6e43b7eef49f48eba6c22e9d6a2d3bf170b7751e084172427f566dd7035b3510) applies the reviewed growth bound and
rehashes the same original successful install. It is a corrected review, not a
second install. Both review outcomes are retained.

The Trisha docs packet also retains its superseded root-attribute proposal and
the failed assertion that two different base attribute blobs were equal. The
final directory-local rule preserves the 38-file byte-retention source binding;
the [root merge-tree review](objects/9c9bf606fa91c04df02436a48de5893865fbf69a592ce277762fa91dbd176e90) authenticates that result.

## Exact-byte archive and privacy

`retained-files.json` maps 148 originals (153814 bytes) to 85 unique objects.
`references-only.json` identifies source inventories and installed binaries
kept in their original locations or already accepted owner audits. Full proofs,
binary bodies, full-host process snapshots and signed URLs are excluded.
Only safe selected integration/install/review files are copied; the original
private evidence and failures remain in their owning lanes.

Validate from this directory without executing any retained original code:

```sh
python3 -B -W error verify-retained.py
python3 -B -W error verify-retained.py --originals
```

The second command additionally compares each object with its original local
file. All objects use directory-local `-text`; five exact raw diff objects
disable only blank-at-EOL checks for preserved context prefixes. The original
GitHub responses are observations tied to their recorded commands and times.
