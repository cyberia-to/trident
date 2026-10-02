# Generated-profile metadata with empty PATH

The original full C3 corpus attempt reached the generated-profile helper after
five corpora passed, then could not launch bare `git` under the intentionally
empty PATH. That failed attempt remains in `R2/whole-proof-corpus`; this change
does not turn it into a pass. Here `R2` means
`/Users/master/cyber/.worktrees/selfhost-0.4-finalization-20261002`.

The helper now accepts `--git /absolute/path/to/git`; otherwise it uses
`TRIDENT_AUDIT_GIT`, followed by ordinary PATH lookup when neither was supplied.
Explicit and environment values must be absolute executable file paths. The
selected executable path, SHA256 and actual metadata command arguments are
recorded. Its identity is checked before and after metadata capture and at
final verification. The process PATH stays unchanged for Joy and compiler work.

Validation ran from Trident base
`e57f2b4c` on the isolated `fix/0.4-corpus-metadata-git` branch. The
[complete revision and commands](validation.json) bind these results:

- `python3 -B -m unittest audit/self-hosting/test_generated_profile_metadata.py audit/self-hosting/test_compiler_selection.py`:
  21 checks passed: seven metadata checks and fourteen existing routing checks.
  Metadata checks launch actual Git with empty PATH; an explicit stop before
  guest execution preserves their narrow scope.
- `python3 -B -m unittest audit/self-hosting/test_bootstrap_runner.py audit/self-hosting/test_bootstrap_phases.py`:
  46 checks passed, including the expected rejected head/pin mismatch.
- [Actual `cargo check --locked --offline`](cargo-check.json) uses explicit
  native Rust 1.89.0 paths and the existing warm build cache.

The generated-profile exercise AST, after removing only its metadata capture,
is identical to the base revision. All 21 guest observations, fixtures,
Joy/compiler command semantics and resource bounds are unchanged. The bootstrap
orchestrator is unchanged. The source-impact check also binds the unchanged
Rust/Trident production paths to the separately retained complete workspace
run: 1231 passed, 0 failed, 5 existing ignored cases. That prior command's exact
receipt is identified in `validation.json`; this patch does not claim another
complete Rust-suite run or a completed whole C3 corpus rerun.

Root independently reviewed the helper and test delta and ran the same focused
21 checks successfully. Its supplied acknowledgement is recorded in
[source-review.json](source-review.json); raw output from that independent
invocation was not supplied. A fresh wrapper run, with immutable absolute Git
input and empty PATH, remains separate acceptance evidence.
