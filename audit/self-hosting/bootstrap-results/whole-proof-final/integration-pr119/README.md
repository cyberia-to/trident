# Reviewed tooling integration into release/0.4

PR119 merged reviewed head `e1b825c76cdc7f3aa6a167d43f1a818d4e35ce50` onto `8da6f8f6efb3eafb4697f3ce7da74e059d18e02e` as `28c982c0ff4fa9572f63778688a2763e0ed2878e`. The independently calculated and actual merged trees both equal `131b52d5f724128afd435043c477ba82355ee2bb`. The incoming self-hosted-compilation guide is preserved.

The exact-head review binds unchanged product source/dependency/workflow trees, the separate tooling/evidence diff, retained source reviews and fresh-prefix Rust1.89 installation. Push, PR creation and all merge command outcomes remain exact originals. Both the reviewed head and merged base installed with zero warnings and the same Trident/LSP byte identities. Commands, revisions, durations and binary hashes remain in their original receipts.

A new feature branch, `docs/0.4-full-selfbuild-acceptance`, starts from that accepted merge in the existing isolated worktree. Only the incoming guide changed at the switch; all four archive maps pinned by the prepared final collector stayed byte-identical. Live proof/controller sources remain unchanged.

This integration accepts diagnostic tooling and retained results. SH8 remains open for the complete original negative suite and final checker. Owner version/default-branch/tag/publication actions remain separate.

`retained-files.json` maps exact originals to SHA256 objects. Run `python3 -B -W error verify-retained.py --originals` to check retained bytes without executing their code.
