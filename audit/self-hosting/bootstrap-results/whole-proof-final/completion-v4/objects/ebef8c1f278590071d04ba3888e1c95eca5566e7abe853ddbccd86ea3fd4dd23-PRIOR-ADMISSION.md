# Historical case admission for the v4 continuation

This read-only adapter authenticates retained case-level evidence. Neither the
failed v2 suite nor the failed v3 suite becomes successful. It launches no proof
command, constructs no mutation and grants no deletion permission. The two
`/bin/ps` calls used by retained-file admission observe PID/PGID/birth only.

`prior_cases_v4.review(base, generation, proof, original_verifier, expected)`
returns schema `trident/prior-complete-proof-cases/v2` with
`status: passed-selected-cases`. `base` is the experiment family directory;
`proof` and `original_verifier` must be the original canonical successful paths;
`expected` is the complete independently read fresh-verifier `verification`
response. The pinned original admission independently binds that supplied
response to the actual producer and fresh verifier.

The result contains:

- `controls`: the two original v2 controls, preserving their original paths.
- `rejections`: exactly the original nine v2 rows followed by the three completed
  v3 rows `rebound-job-limit`, `continuation`, `generation`.
- `original_v2`: the complete unchanged result of the pinned original admission.
- `additional_v3`: the failed suite reference, exact three names/rows and original
  command receipt references, plus its authenticated copies of index/result.
- `index` and `result`: the original v2 `{path, bytes, sha256}` references. The v3
  copies are required to have the same complete bytes.
- `quiescence`: exact later historical absence evidence. The failed shutdown and
  its null child exit statuses are retained unchanged.

The original nine and additional three are replayed against exact source pins,
original compiler/JOB/helper/binary inputs, argv, profile, before/after input
identities, expected exits, required rejection classes, protected outputs, raw
stream hashes and raw resource samples. The three v3 cases additionally require
native retirement inside their actual command/coordinator intervals. The
corrected JOB-limit context derives from its original authenticated admission.
There is no source substitution with the faster v4 mutation helper.

`prior_cases_v4.admit_c2_cost(base, proof, original_verifier, expected)` returns
schema `trident/retained-cost-construction-admission/v1` and
`status: passed-construction-admission`. It performs the same historical replay
for C2, then checks its successful original `construct-cost` invocation, complete
helper result and the actual retained file. It retains the interrupted old
verifier as `failed`, exit `-15`, `shared-stop`; no old verdict is accepted.

Its `certificate` has `{path, bytes, sha256, stat}`. `stat` includes `device`,
`inode`, `bytes`, `mtime_ns`, `ctime_ns`, `birthtime_seconds`, `birthtime_ns`, `links`, and `mode`. Where the host exposes
only floating-point birthtime seconds, the nanosecond coordinate is derived
from that available value and both are retained.
The complete SHA is read through a held `O_NOFOLLOW` descriptor while requiring
one regular single-link inode and unchanged descriptor/path state. Historical
quiescence is replayed, and two current PID/PGID/birth observations bracket that
read. Overlapping reused numeric IDs must have been born strictly after the
historical empty observation plus its timestamp-rounding margin.

`construction`, `helper`, `index`, and `result` are `{path, bytes, sha256}`
references. `recipe` contains `mode: cost`, empty context, the exact certificate
identity, and an absolute original construction receipt path.
`interrupted_verification` identifies the original failed command and original
compiler/JOB/protected output. `expected_fresh_error` is
`semantic terminal: Claim`; `fresh_verification_required` is true;
`accepted_as_rejection` and `deletion_authorized` are false. A new successful
negative-verification receipt and separately reviewed ownership/retirement
transition are required before the controller may reclaim this temporary.

The original guard uses `time.monotonic()`. Its elapsed reading and the Unix
receipt timestamps can differ on this host; `clock_observation` reports both.
The historical command is not represented as completing within the same civil
clock duration. No numerical command/resource cap or semantic profile changes.

`replay_prior_v4.py FRESH_OUTPUT_DIRECTORY` is a read-only actual-evidence replay;
it retains exact source identities before/after and failed outcomes without
rewriting earlier attempts. `test_prior_cases_v4.py`, `test_prior_cost_v4.py`, and
`test_prior_quiescence_v4.py` use tiny synthetic receipt/file trees; they run no
native proof command and are not proof evidence. Their fixture-only pin
substitutions are restricted to tests. Original source families and all existing
proofs remain read-only.
