# Isolated completion and reclamation regressions

Run with an explicit Python 3.14 executable:

```
python3 -B -W error -m unittest discover -s tests -p 'test_*.py' -v
```

These tests never launch the real full-proof coordinator, consume a whole
certificate, or invoke the reclamation action. Each writable fixture lives in a
fresh temporary directory. The cleanup-receipt regression calls coordinator
logic only with its first gate read and cleanup function forced to fail, and
asserts that `Popen` was never called.

- `test_parallel_completion.py` adapts all eight previously exercised v2
  process/accounting cases to v3. It adds charging of every retained v1/v2 scope,
  exhaustion by retained bytes, and preservation of the original failed receipt
  when cleanup also fails. Real tiny Python children exercise exact rejection,
  bounded stdout overflow, a reparented native group and a leaderless group.
- `test_prior_cases.py` checks original producer/fresh-verifier eligibility,
  exact proof/caller result binding, all nine distinct prior names, exclusion of
  the failed semantic case, unchanged protected output, exact diagnostic argv,
  construction context and error class, and the immutable checker source pin.
  `prior_fixture.py` supplies synthetic byte-sized receipt trees. It executes the
  real pinned checker's validators while replacing only its source-pin map and
  expected output-artifact hash with explicit fixture identities. These fixtures
  are unit inputs, never production proof evidence.
- `test_binding_context.py` preserves the eight focused context-derivation
  regressions already exercised with the actual accepted SH7 fixture. Source
  authentication remains the production admission's responsibility.
- `test_case_matrix.py` directly exercises the pure terminal validator in both
  generation modules: exact nine-plus-fourteen admission and rejection of
  missing, duplicated, unknown, misplaced or reordered cases and missing control
  or incorrect prior status.
- `test_reclamation.py` calls only `unlink_exact` on tiny owned files. It checks
  changed state/bytes, symbolic and additional hard links, replacement/mutation
  during the durable callback, callback failure, and successful removal of only
  the exact file after that callback. Injected directory-sync failures confirm
  preservation before unlink and truthful removal accounting after unlink.
- `test_reclamation_v2.py` repeats those exact-unlink cases against the separate
  corrected action. `test_reclamation_process_identity.py` checks actual old
  process refusal, synthetic reused PID/group and leaderless cases, rounding and
  malformed-time ambiguity. The v2 action requires every overlapping process to
  have a UTC birth strictly later than the final empty shutdown plus two seconds.
  No process is signalled by process classification.

The first prior-fixture run is retained as `prior-first.*`: its test-only import
loader omitted `create_module`, causing thirteen fixture errors before any
admission test ran. The corrected loader's `prior-second.*` records fourteen
passing tests. Original production source and failed whole-proof receipts were
not changed by these tests.
