# Local postcommit installation

Trident receipt commit `e329af70324615327c6c6574b6e22c2fc4732234` was installed
successfully into a separate prefix with a separate Cargo target directory.
The command exited 0 with zero Rust warnings; the installed CLI reports
`trident 0.3.0`. No guest compiler or CI bootstrap was rerun.

```sh
cargo install --path /Users/master/cyber/.worktrees/selfhost-0.4-full-bootstrap/trident-legacy-retention \
  --force --locked --offline \
  --root /Users/master/cyber/.worktrees/selfhost-0.4-full-bootstrap/install-legacy-retention \
  --target-dir /Users/master/cyber/.worktrees/selfhost-0.4-full-bootstrap/target-legacy-retention \
  --jobs 2
```

The retained receipt binds the exact command, 388 Trident production-source
hashes, nine sibling checkout identities/statuses before and after, both
installed binaries and the CLI version check. Sources and checkout states
remained unchanged. The sibling survey includes pre-existing Lens edits;
this is a local maintenance installation, not a clean-origin release candidate
or additional native CI acceptance.

Installed `trident` SHA-256:
`3d9381e40510a32e03032d31f3203ba53e771919bb19e9d89a779000ff71b030`.
Installed `trident-lsp` SHA-256:
`889455a17467881046057751a015afb2631a01752e311adad53eb3a7e79b626d`.
`files.json` binds the lossless raw logs, receipt and measured capture script.
