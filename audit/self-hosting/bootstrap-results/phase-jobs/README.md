# Native bootstrap phases

This delivery separates each clean self-build producer from its C2 and C3
corpus jobs. The original run
[36359020560](https://github.com/cyberia-to/trident/actions/runs/36359020560)
at `c17bd0371c11746f46e20222c48cae2ab08be79d` completed all twelve C1→C2→C3
chains and fixed-point checks. Both Intel macOS repetitions then reached the
330-minute outer step limit during their C3 corpora. Ten repetitions completed
all corpora. The original failed aggregate and exact partial artifacts remain
in the [retained run](https://github.com/cyberia-to/trident/blob/19e35ee/audit/self-hosting/bootstrap-results/run-36359020560/README.md).

The implementation contract is
[SH6 in the canonical reference](../../../../reference/self-hosting.md#sh6-reproducible-bootstrap).
Each of twelve fresh producers exports its native Joy and actual C2/C3;
twenty-four native corpus jobs consume those exact bytes with explicit
generation and producer bindings. The aggregate requires all original phases
from one run/attempt/head. Existing execution profiles, corpus cases and quotas
are unchanged. Prior successful repetitions cannot fill new-run phases.

The phase core is commit `51e984cca290197ef90fa7e7393840626b319eca`.
`final-gates.json.gz` captures the preceding c17 base plus eight exact source
identities, unchanged throughout validation. The committed core preserves
those measured bytes. The three implementation hashes are recorded in every
phase receipt; source, target, fixture pins, compiler generation, original
producer receipt/manifest and exact Joy bytes are checked before acceptance.

## Validation

- The ordinary Python command in `final-gates.json.gz` passes 87 tests: legacy
  orchestration, the new phase boundaries and entrypoint routing, fixed-point
  checks and existing compiler-selection guards.
- Its optimized-Python boundary command passes the same 64 orchestration and
  fixed-point tests. This repeated subset adds no distinct tests.
- Actionlint and diff checking pass. The source and evidence received
  independent review. Wiring fixtures and mocked expensive hooks supply no
  guest execution or SH6 acceptance.

The phase guards reject missing/duplicate/mixed-run evidence, relabelled C2/C3
roles even with identical compiler bytes, changed Joy/compiler exports,
incomplete or substituted corpus wrappers, changed pins/native profile and
input/output overlap. Overlap checks run before creating output or work,
including a symlink-parent alias. Routing tests exercise the producer export
and both consumer generations while forbidding a consumer seed/self-build.

## Retained diagnostics

An early check still expected the previous single-job workflow; its original
failure is retained before updating that workflow assertion. An exploratory
85-test optimized-Python command also ran the existing assertion-based corpus
routing tests directly. It failed with one failure and nine errors. The same
23-test legacy subset at untouched c17 reproduces those exact named failures.
Those corpus scripts require ordinary Python; the bootstrap child adapter
explicitly rejects optimized Python and clears inherited `PYTHONOPTIMIZE`.
Their supported ordinary-Python gate passes. The final optimized boundary gate
tests the fail-closed orchestration without claiming an optimized corpus run.

All command arguments, working directories, actual exits, before/after source
identities and raw output bytes are retained. Earlier validation remains
identified by its own source capture. `files.json` binds stored and decoded
files. Native phase CI and original aggregate replay remain required before
SH6 closes. Compilation proof gates SH7/SH8 stay separate.
