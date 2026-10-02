# Explicit Git metadata for the extracted-compiler corpus

The original complete C3 corpus attempt failed in its sixth helper, before that
helper executed a compiler. Its first five child commands succeeded. The failing
helper resolved the metadata command `git rev-parse HEAD` through an empty `PATH`.
The full original directory is retained in
[the failure archive](original-corpus-v1-failed.tar.gz), with every file bound by
[the inventory](original-files.json). Its actual fixture revision was
`e57f2b4c6c6b7128e1cbbf80a77a0ccfcc8d33d7`; the receipt inside the archive binds
the exact verified C3, production Joy and all commands.

Fixture commit `2f1a575a6fbc0704c268f1ae21667830a3c996df` adds an explicit
`--git` / `TRIDENT_AUDIT_GIT` executable to the generated-profile helper. The
replacement wrapper resolves that metadata executable before starting the corpus,
binds its bytes as a sixth immutable input, and passes its absolute path through
`TRIDENT_AUDIT_GIT`. Every Joy command retains an empty `PATH`. Compiler selection,
source fixtures, expected observations and resource limits are unchanged.

The [independent review](prepared/whole-proof-corpus-v2/independent-review.json)
binds all replacement wrapper and helper sources. The original bootstrap driver,
compiler-selection logic and five other corpus helpers remain byte-identical.
The new wrapper runs the complete corpus for each actual proof-extracted C3/C2
in a distinct evidence directory. This prepared-tool delivery records no final
replacement corpus verdict.

Run `python3 -B audit/self-hosting/whole-proof-validation/corpus-v2/check_delivery.py`
from the repository to verify every original archive member and every prepared
source against its recorded identity. The checker also verifies the actual
original failure and its fixture revision. Large successful new corpus results
will be retained separately when their gates finish.
