# C2/C3 corpus results from full-certificate verification

Both proof-extracted compilers passed the complete measured corpus on macOS
arm64 with an empty runtime `PATH`. C2 and C3 each produced 547 observations
across six corpus helpers. The results come from fixture
`2f1a575a6fbc0704c268f1ae21667830a3c996df`, production Joy source
`6e0ec4d8440e2521df08f442d64f54e667044716`, and the actual invocations retained in
`corpus/{c2,c3,orchestration}/receipt.json` inside
[the complete archive](successful-corpus.tar.gz).

| Helper | C2 observations | C3 observations | Recorded commands per generation |
|---|---:|---:|---:|
| `run-native-compiler.py` | 402 | 402 | 1197 |
| `check-guest-constant-linking.py` | 31 | 31 | 140 |
| `check-guest-function-imports.py` | 32 | 32 | 148 |
| `check-guest-type-imports.py` | 24 | 24 | 110 |
| `check-guest-intrinsics.py` | 37 | 37 | 157 |
| `check-generated-compiler-profile.py` | 21 | 21 | 64 |
| Total | 547 | 547 | 1816 |

The original command is `run.py --generation 2 --fixtures <fixture> --revision
2f1a575a6fbc0704c268f1ae21667830a3c996df`, followed by the same command with
`--generation 1`. The argument selects the proof producer generation; its
fresh verifier extracted C3 and C2 respectively. Each wrapper then invoked
`bootstrap-runner.py --corpus-child <helper> --retain <directory> --joy <binary>
--compiler <fresh-verifier/compiler.dag> --output <report>`. Every complete
argument vector, working directory, stdout, stderr and reported observation
remains byte-exact in the archive. Expected rejection commands are included in
the command counts. `TRIDENT_AUDIT_GIT=/usr/bin/git` supplies only metadata Git;
Joy retains `PATH=""`. The per-helper wall ceiling remained 5400 seconds.

C2 and C3 ART1 files are byte-identical: 9,691,488 bytes, SHA-256
`76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8`.
Their compiler particle is
`2eea2ac5f611012877b4e7291a3a6f534aee7281bb358a0b8e5fabe2ac1f9fbe`.
The actual Joy executable is 5,635,776 bytes, SHA-256
`8f42591ece35f192ff6f2328a8360fe0f0959f48a173248b211cd0d8f4d984f9`.
These bytes, the two separate fresh-verification receipts, their outputs, the
producer receipts and the successful Rust 1.89 installation receipt are
included. The proof certificates themselves remain in the separate whole-proof
retention scope; their exact identities are bound by both producer and fresh
verifier receipts.

[The inventory](files.json) preserves all 6,699 files of the original completed
`whole-proof-corpus-v2` directory, including its orchestration and scheduler
outputs. Together with the supporting files and the twelve previously reviewed
source/review copies, the archive has 6,731 regular files and 61,907,247 logical
bytes. Its compressed size is 18,476,509 bytes and SHA-256 is
`c5c6b07d6ff6ba65e680fddb17297810a9b9d60222356b010b906c82d9d9177b`.
[The archive receipt](archive.json) binds these counts and the original paths.
The packager normalizes archive metadata and verifies source membership and
bytes again after writing. Original file content and path strings are retained.

Run from this repository:

```sh
python3 -B -W error audit/self-hosting/whole-proof-corpus-v2-result/check_delivery.py
python3 -B -W error -m unittest discover -s audit/self-hosting/whole-proof-corpus-v2-result -p test_delivery.py -v
```

The checker rehashes every member without extraction, checks exact inventory
membership, rejects duplicate/link/traversal members, checks producer-to-fresh
receipt identities, C2/C3 generation-specific input paths, the actual binary and
compiler bytes, all six source/input bindings, all 547 observations and command
counts, and the preserved empty runtime `PATH`. `--originals` additionally
compares every retained member to its original file and checks complete original
scope membership on this host. It replays evidence integrity and receipt
consistency; it does not execute Joy or independently verify the whole proofs.
The test suite alters semantic records and recomputes their stored identities,
so generation, compiler, count, command and result checks are exercised beyond
archive corruption detection.

The [original failed attempt and prepared replacement tools](../whole-proof-validation/corpus-v2/README.md)
remain intact. That attempt failed before the sixth helper executed a compiler
because bare metadata `git` could not resolve under an empty `PATH`. This result
uses the separately committed metadata-only fix and a complete new run for both
compilers. Historical helper scope strings in raw reports are preserved as
written. Optional oracle-coverage flags describe applicability, while assertion
flags report checks performed; they have distinct meanings in the source.

[Validation](validation.json) records exact packaging/replay/test commands,
source hashes and unchanged tracked repository state. Early checker-development
failures are retained beside the final successful logs: the first draft
incorrectly treated optional oracle-coverage flags as assertions. No corpus
result was rewritten. This delivery records the measured corpus gate only;
SH8 completion, adversarial full-proof acceptance and durable full-certificate
retention require their separate receipts.
