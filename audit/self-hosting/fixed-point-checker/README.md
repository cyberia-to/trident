# Fixed-point checker validation

This validates the checker and its source metadata guard. No completed C2/C3
pair was available for this receipt. Semantic corpus acceptance and full
self-host acceptance remain separate required gates.

[identities.json](identities.json) records the Trident revision, exact commands,
checker/test source hashes and evidence hashes. All code was local uncommitted
work at collection time. The [unit log](unit-tests.log) records fourteen passing
tests using synthetic consistency receipts and existing Joy artifact fixtures.
They execute no compiler. The tests cover supplied-artifact chaining, exact byte
equality, tampering, source hashes and inventory rejection, path confinement,
options/LIM1 equality, publication identity, NoTrace/gas guards, header framing,
failed checker invocation and preservation of existing output.

The real [snapshot-only check](snapshot-only.json) verifies all 94 modules and
369707 source bytes retained by the running compacting-closure probe. Each copy
matches its declared SHA256. Copies reconstructed at canonical relative paths
then pass the prebuilt inventory tool's `--check`, which compares the complete
inventory including BLAKE3 source identities. The metadata tool's SHA256 is
unchanged before/after. Its exact argv, output, source SHA set and input receipt
identity are retained. The original closure receipt remains untouched and was
still `running` when read; this check makes no statement about its execution.

Reproduce from the recorded Trident worktree, choosing a fresh output path:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s audit/self-hosting -p test_selfhost_fixed_point.py -v
PYTHONDONTWRITEBYTECODE=1 python3 audit/self-hosting/fixed-point-checker/snapshot-only.py --receipt ../measurements/compacting-closure.json --inventory-checker ../target-root/release/examples/selfhost_inventory --output /absolute/path/to/new-snapshot-check.json
```

Once two successful program-publication receipts exist, run:

```sh
python3 audit/self-hosting/check-selfhost-fixed-point.py --first /path/to/c1-to-c2.json --second /path/to/c2-to-c3.json --inventory-checker /path/to/prebuilt/selfhost_inventory --joy /path/to/recorded/joy --output /path/to/new-fixed-point-check.json
```

The current checker additionally [repacks JOB1 from verified source bytes](../fixed-point-job-binding/README.md)
using the explicitly supplied Joy binary, whose SHA256 must match both runs.
The metadata-only measurements above remain historical evidence of their named
scope. No compiler is built or executed by either check. Repacking validates the
compiler input and binds the exact sources, options and limits to the saved job.
Recorded Joy execution still supplies the output claim; semantic correctness,
corpus acceptance and execution proofs remain separate gates.
