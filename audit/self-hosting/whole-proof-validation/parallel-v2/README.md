# Parallel complete-certificate diagnostics

This delivery retains the reviewed second scheduling profile. Full SH8 acceptance
remains open. Its two generation suites are running in the isolated measurement
directory recorded by [files.json](files.json); this archive does not claim their
completion or a release verdict.

The [plan](retained/whole-proof-attacks-parallel-v2/plan.json) keeps two controls
and all twenty-three rejection cases per generation. The independently verified
original certificate is an explicit existing control; a fresh verifier must also
accept the byte-identical reconstructed certificate. Every changed request and
certificate still needs its own expected diagnostic and protected-output check.
The production compiler, Joy, proof format, semantic cases and helper stay fixed.

The earlier single-generation schedule was deliberately interrupted after its
complete index succeeded and reconstructed-certificate construction had started.
[Transition evidence](retained/whole-proof-attacks-parallel-v2/transition.json)
retains the actual failed receipts and classifies the partial file. The partial
remains untouched and charged to the same shared 26 GiB disk cap. No successful
negative result is inherited from that interrupted schedule.

The coordinator reserves both possible full mutations and bounded output/log
buckets before launching either generation. Each generation holds at most one
mutation. Its helper file ceiling is the original certificate size plus 32 MiB,
below the original helper limit. Original helper/verifier CPU, wall-time, RSS and
proof limits remain unchanged. The physical free-space floor stays 8 GiB; launch
also retains both mutation ceilings plus 10 GiB headroom. Combined sampled RSS is
bounded at 12 GiB. Canonical/output and metadata buckets are independently bounded.

A native child registers its process-group identity before exec. The coordinator
continues accounting for that group after parent or group-leader exit. Shared
resource failure stops both suites, preserves unfinished payloads, and records
bounded termination plus a final group-empty observation. Reclamation requires
the registered generation and retained successful verification evidence.

[Independent review](retained/whole-proof-attacks-parallel-v2/independent-review.json)
binds the exact source manifest and copied original inputs. Eight isolated tests
in the [test receipt](retained/whole-tooling-delivery-review/parallel-v2-test-receipt.json)
exercise real subprocesses, capped logs, semantic rejection, shared disk accounting,
caller-bound reclamation, and termination of both orphaned and leaderless native
groups. The original failing test invocation is retained: its temporary path
needed canonicalization across macOS `/var` and `/private/var` aliases.

[Delivery source equivalence](delivery/test-inheritance.json) binds the complete
previously run Rust workspace result to identical source blobs at the named
revisions. That inherited result is separate from the freshly executed Python
subprocess tests and the fresh delivery `cargo check`. Large certificates and
prepared input copies remain external immutable files with recorded identities.
