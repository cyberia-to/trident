# Cross-repository evidence-link integration

Joy PR30 repaired three README links to immutable Cyber and Zheng files.
Trisha PR27 repaired its audit index link to the accepted Trident PR119 ledger.
The target Git blobs match the authenticated GitHub file responses. The exact
document diffs, commands, independent root review and merge outcomes are retained.
SH8 acceptance wording stayed pending.

| Repository | Reviewed head | Accepted merge | Matching reviewed/merged tree |
| --- | --- | --- | --- |
| Joy | `f008a14a6c5e66dca5cfb7acade924618ea5b4a8` | `89088bb27a2d6eaa63ec85aaa2eaec08028c28ad` | `7d1dae78d9d5b298ff0a2935c70476e744b4492f` |
| Trisha | `be137e7fa582198f13b2afe7f54d29a2aba4db48` | `c9f442c58005249d14e95fc81f125ff7f864ab07` | `e2e97a67f418673af52850a337777dc61e115d7f` |

Each reviewed head and accepted merge passed a fresh-prefix, offline Rust1.89
installation using source-equivalent accepted archive paths and existing warm
fingerprints. The copied drivers retain the 120-second deadline, 64-MiB target
growth bound, 8-GiB free-space floor and refusal of any compilation. All four
installs passed without warnings; warm target sizes and production inputs stayed
unchanged. Joy retains SHA256
`7b1aee370f6826db73f089054f261a3ef874ad54d2e259fbd26f7d8164ab6071`;
Trisha retains `fe069879a44b582a47069c5cd849b1eb3da4c75a037e72aa9d107742ecc8173f`.
These were installations from equivalent archived source paths, with the exact
new documentation commits checked by their wrappers.

Clean `docs/0.4-final-proof-acceptance` continuation branches were created from
the two accepted merges. No final-proof status edit was made. Their branch
receipts bind the clean states before and after, matching trees and exact heads.

This packet also retains the root's nine immutable-link HTTP checks and three
Trident fresh-prefix postinstalls: PR119 retention at `9e81bd187af63259940c273be8711a0fc34757f2`,
Joy source bridge at `ae47afd4c5f9579edf6d29c4f8fc2a61e6c0acb5`, and documentation
status at `6200ce5614f009bc9cb1cff1f5b0b5c8d51919d8`. Their original commands,
timings and unchanged Trident/LSP binary identities remain in the receipts.

`retained-files.json` maps selected original public bytes to SHA256 objects.
`references-only.json` binds existing source inventories and duplicate document
bodies without copying them again. The link receipts retain the immutable
repository/revision/path, byte identities and observed API outcomes. Executables,
proof bodies, full-host process snapshots and signed transport URLs are excluded.

Run `python3 -B -W error verify-retained.py --originals` where original paths are
available, or omit `--originals` to validate the archive alone. The verifier is
the unchanged `7084b3e31fff02d78533e48998ac2994afc1cbd2744a6586fe9335315ceae446`
implementation. Directory-local `* -text` preserves exact bytes; any whitespace
exception is scoped to an original raw object. This packet records documentation
integration and installation evidence, with no new SH8 or public-release verdict.
