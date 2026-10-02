# Authenticated unchanged-frame digest reuse

The diagnostic mutation constructor previously hashed each unchanged frame twice.
Its checked reader now seals the frame bytes and retains their authenticated digest;
the writer reuses it only when its emitted header is identical. Rewritten headers
and newly encoded frames compute a new digest. The runtime verifier and proof
profile are unchanged.

Exact source, commands and source identities are retained here. `build.py` records
actual Rust 1.89: ten unit tests and the release build passed with warnings denied.
`fixture.py` records fourteen complete byte comparisons against original pilot
mutations, plus fourteen fresh verifications against frozen Joy 6e0ec4d; all results
and protected outputs agree. Full fixture certificate bodies remain in the original
local experiment, identified by the retained receipt. No whole-proof performance
claim follows from those bounded fixtures.

The independent review approves this helper only. The failed v3 whole schedule and
its cleanup error remain failed; a new schedule, admission and final independent
acceptance are separate work. All retained raw source and command bytes are listed
in `retained-files.json`; Cargo paths and original command directories describe the
original isolated experiment layout, not a new Cargo workspace inside this archive.
