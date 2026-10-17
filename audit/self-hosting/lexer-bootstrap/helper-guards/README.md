# Audit checks require enabled assertions

The local archive collector and the C2/C3 archive verifiers now reject optimized
Python before reading or writing evidence. Those scripts use assertions for
identity and corpus checks; `python -O` or `PYTHONOPTIMIZE` would disable them.
The executed bootstrap and corpus launchers already required enabled assertions.

`before.json` binds the exact three helpers from commit
`989eaee` and their retained original source archives. The subsequent source
change adds an explicit `sys.flags.optimize` guard. Existing compiler receipts,
artifacts, source snapshots, logs and verification results stay byte-identical.

The command at base revision `989eaee`, with final helper identities recorded
in its receipt, was:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 audit/self-hosting/lexer-bootstrap/helper-guards/check.py --output audit/self-hosting/lexer-bootstrap/helper-guards/results
```

All nine subprocess checks passed: each helper validates the real archived
evidence normally, and exits unsuccessfully before producing output under both
`-O` and `PYTHONOPTIMIZE=2`. `results/receipt.json` retains the exact commands,
environment changes, exits, helper hashes and unchanged input hashes. Compressed
stdout/stderr preserve their original bytes. These are audit invocation checks;
they add no compiler tests or native-platform acceptance.

Rechecks require a fresh output directory and the original local measurement
inputs consumed by `collect.py`. The two corpus verifiers can separately check
their retained evidence and Git history. The checker itself also rejects
optimized Python.
