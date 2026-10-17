# Native self-hosting integration into 0.4

The accepted compiler implementation, portable source preparation, Joy boundary
checks and accepted kit are merged into `release/0.4`. `retention.json` records
all seven feature merges: Joy PR22/23, Trident PR112/113/114 and Trisha PR18/19.
Each merge was checked against its exact reviewed head, current integration
branch, computed merge tree and both actual parents. Master, tags and package
publication are outside this delivery.

The original native acceptance remains bound to Trident `57491633` and CI
`36382085561`; see [the accepted matrix](../run-36382085561/README.md).
These integration checks do not rerun the compiler matrix or change its pins.
The [accepted kit](https://github.com/cyberia-to/trisha/blob/c66c2da3da0d5b1da55f09be533a4d664d585bb2/audit/selfhost-kit/accepted/README.md)
retains its own actual runtime checks and independent reviews.

## Commands and exact scope

`merges/` retains the raw GitHub and Git commands, responses and merge receipts.
The original preflight refusals are preserved: PR23 initially exposed an old PR
base snapshot, and PR114 initially had unknown mergeability. Neither refusal
performed a merge. The corrected driver checks the current branch reference,
verifies the PR base snapshot is its ancestor, and compares the actual merge's
parents and complete tree with the expected result.

Trident commit `6bc98d3c` integrates the accepted `99c8a44f` implementation into
the evidence branch. Its only conflict resolutions selected the current work
plan and the exact reviewed upstream self-build guide. Compiler, runtime,
helper and workflow implementation bytes match that accepted upstream tree.
The retained local checks are:

- `python3 -B -W error scripts/test_prepare_selfhost_source.py -v`: 21 passing
  helper cases on the resolved tree committed as `6bc98d3c`.
- `actionlint .github/workflows/selfhost-bootstrap.yml .github/workflows/selfhost-source-transport.yml`:
  success on the same resolved tree, using the isolated
  `selfhost-0.4-full-bootstrap/actionlint-bin/actionlint` executable.
- Isolated `cargo install --path . --locked --offline --force`, with absolute
  `--root`, separate `CARGO_TARGET_DIR` and the prefix on `PATH`: success with
  zero warnings after acceptance commit `761e233a` and integration `6bc98d3c`.

`kit-postinstall/receipt.json.gz` records every command and environment of the
Rust 1.89 installation at Trisha `c66c2da3`. Vendor preparation, local dependency
metadata, installation and installed version checks all passed with zero
warnings and an unchanged isolated source family. This validates installation;
the existing full runtime/proof gates retain their original revisions.

`review/` contains the root review of merge parents/trees, documentation hashes,
kit identity and postinstall bytes. The independent native-acceptance and kit
reviews remain in their original audits. The retention contains 186 exact raw
files, compressed individually; every stored file was compared with its
original bytes. It adds no new platform execution or proof acceptance.

SH6 is closed for the declared frozen S1 subset. Full distribution platform
gates, Rust frontend/tooling parity and SH7/SH8 compilation proofs retain their
separate acceptance criteria.
