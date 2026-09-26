# Nominal bindings integrated with guest function imports

This supplements the [isolated seed evidence](nominal-bindings.md).
The tested integration is `285681d707a178d9ebd875e5b13753eb3f58f856`, merging isolated
state `5d2e3ed104e97b3eabf92762e6440076a17306f1` with release H `1d283fc9815cd0faa6653951de032a8d262f3a17`. Seed source `5d06645b7190a11b5bdab7ed8e3bad7af993c186`
and callable source `4acc73021497950154ec6b2e68e1e6f2536a2ba3` remain distinct provenance pins.
The [combined receipt](nominal-bindings-combined-validation.json) records exact
commands, logs, source snapshots, binaries and local macOS ARM64 evidence.

All 1165 Trident, 122 Joy and
380 Trisha tests pass without warnings; the
4 existing Trisha tests remain ignored. The 11 nominal
regressions pass alongside the function-import tests. All 133 baseline rows and
43 manual programs match H. All 111 formal audits
report UNKNOWN; this is not a native execution proof.

The [combined installed CLI run](nominal-bindings-combined-cli.json) contains
28 commands, 8 cases and 16 warrior observations. Joy and Trisha check complete
positive outputs and preserve existing artifact bytes on the five expected
nominal-layout rejections. The original receipt records the uncommitted merge
on base `5d2e3ed104e97b3eabf92762e6440076a17306f1`; every source snapshot matches the committed integration.
Clean post-commit installation reproduces all three combined binaries exactly.
Those binaries differ from the isolated binaries; neither receipt is relabelled.

The full 1195-command / 401-observation guest corpus and 39-command /
12-observation graph corpus were not rerun on the combined binaries. Their
[H execution evidence](guest-function-imports-validation.json) is reused because
the rebuilt C1 and graph artifacts are byte-identical to H, with unchanged guest
and runtime source trees and pinned sibling runtime libraries. The original
guest execution used Joy `3614adbb3ea1e45869b6b0843a6ea8809503d4e61bbbe372b4fa39ed66e5ef66` at compiler
source `4acc73021497950154ec6b2e68e1e6f2536a2ba3`. No combined execution is claimed for those commands.

Rebuilt C1 is `f39413a1c5abc217bd3f926fa8b6d6546c243678e7381c947dc7ebc74c044b04` (90563 DAG entries);
the graph fixture retains `7fff69be1bec5170a989c7ecd31c237e2907af3dce22d2f7fce31b00dc7ce74a`.
The real rebuild commands and artifact identities are in the combined receipt.

Imported types and intrinsics, generated compiler profiles, compiler-scale
closure, SH4, C2/C3, six CPU platforms and native Zheng gates remain open.
Noun stays 128K. Isolated reports and original receipts remain unchanged.
