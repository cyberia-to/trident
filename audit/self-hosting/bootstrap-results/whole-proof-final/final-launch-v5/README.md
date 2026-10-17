# V5 final acceptance watcher preparation

This packet retains the external watcher source
d56faf9f89f6cb4ecad6b13a23c7cc596b3857dc11cd675c7ddf477dff3564e8 and its tests
f67e10db3d42483bde4055f50b26870a7c2d375d4979623420eb9ae1b5f38c06.
The [independent review](objects/c88253e59eb430e5a22d0680e2350134b20561d093b93720ca9dc1b7fc56bddb)
binds these exact sources. Preparation was archived from the Trident helper
worktree after checker archive revision 3379a23f; no watcher was launched.

The watcher requires all ten original positive, corpus and actual V5 completion
receipts to pass. Two observations must have identical receipt bytes before its
single checker launch. Failed or interrupted outcomes block. Readiness is
bounded to 86400 seconds, polling every 30 seconds; the checker keeps its
3600-second runtime limit. A fresh output directory prevents automatic reuse.

The exact checker source manifest is
9c56357c798c6e3e7727c2f55991e66c69b243b88340f7a334461f56fbdc4584,
with root review
eeaba901de02584018eb0d4b5f4ceac978c738349b0d4ff2468a9f58f2cf4897.
The external watcher is separately bound by this review and its outer launch
receipt. It is intentionally outside the already frozen checker manifest.

The original test command, independently repeated, was:

    python3.14 -B -W error -m unittest -v test_when_ready

All six tests passed. The independent review driver also passed six mock-only
main lifecycle probes, including single successful launch, source or receipt
changes, readiness timeout, nonzero exit and TERM/KILL timeout cleanup.
Mocked subprocess calls never executed the actual checker. Cleanup was reviewed
for this exact read-only checker, which spawns no workload children.

The original V3 watcher, narrow source diff, root test logs and independent test
logs are retained as original bytes. The diff has one exact object-path
blank-at-EOL exception for its unchanged context-line prefixes. All objects use
-text so native checkout preserves their byte identities.

[retained-files.json](retained-files.json) maps 15 original files
(52,107 bytes) to 10 unique SHA256 objects
(39,043 bytes). Original experiment paths identify the measured
family; the object archive is not a relocated executable checkout. Validate it
with:

    python3 -B -W error verify-retained.py

Use --originals only where the original local files remain available. This
packet records source review and offline tests. Actual watcher execution,
full V5 acceptance and SH8 closure remain separate pending evidence.
