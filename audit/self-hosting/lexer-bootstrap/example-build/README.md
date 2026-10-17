# S1 example build evidence

All 21 example targets built successfully with zero Rust warnings. This closes
the example compilation slice omitted by the separately recorded selected-target
test commands. No example was executed and no test pass is added: the S1 gate
still records 1,197 unique passing tests.

The [receipt](receipt.json) records the actual clean isolated checkout revision
`4bbe399c85a5c5ef50e40791cc5f8da79e929071`, exact command and tool identities:

```sh
CARGO_TARGET_DIR=../target-lexer cargo build --release --locked --offline --examples
```

The cwd was `/Users/master/cyber/.worktrees/selfhost-0.4-full-bootstrap/trident-lexer`.
The command exited zero; [build.log](build.log) preserves its exact bytes.
Cargo reported a 1.27-second build using the existing `target-lexer` cache.
This is a local build check, not a clean bootstrap or platform reproduction.

Before building, the prebuilt inventory checker rechecked all 94 S1 source files
against inventory SHA256
`d35d263c7f9f27cbe7ea760a34393105ae8140dc6ab84d484151b6fe04960571`.
All 370,544 source bytes also match their committed blobs at
`77213171d39b88c5f41221912251cc4813ac2b11`. The runner checked source hashes,
627 tracked Rust/Trident/Cargo build inputs, inventory, tool binaries and clean
worktree status before and after the command. Retained Cargo metadata lists all
21 examples with no required-feature exclusions.

[measurement-files.tar.gz](measurement-files.tar.gz) preserves all 13 original
measurement files: the full command receipt, 94-source and build-input hashes,
Cargo metadata, inventory-check output, tool/version paths and raw logs. Every
archive member was byte-compared with its original; per-file lengths and SHA256
values are in the concise receipt. The original files remain under
`../measurements/lexer-example-build/`.

The exact [runner](run.py) was invoked from the main Trident checkout with:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -W error audit/self-hosting/lexer-bootstrap/example-build/run.py --worktree ../trident-lexer --output ../measurements/lexer-example-build --inventory audit/self-hosting/lexer-frame-chunks/inventory.json --checker ../target-probe/release/examples/selfhost_inventory
```

Reproduction requires a fresh output directory. The measured runner SHA256 is
`24ebfec1fdd2221a5fca6667698cb06a78b3f6ebf304c5619aa8ef72564b9fc6`.
Production sources, the feature map and earlier gate receipts are unchanged.
