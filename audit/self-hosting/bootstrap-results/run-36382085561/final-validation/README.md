# Final phase-matrix validation

Status: passed on the complete original run and a fresh three-store restore.
The [accepted evidence](../accepted/retention.json) binds the actual commands,
original aggregate and final run response. `preparation/` retains the earlier
guards and original diagnostics separately from this actual acceptance.

The validator is specific to run `36382085561`, attempt `1`, Trident
`57491633fbccb58ae44dca2da438ee31430be1bc`. It requires the twelve original
producer archives and twenty-four original corpus archives, retained through
three separately pinned archive stores (`producer`, `c2`, `c3`). Each store
contains exactly twelve target/repetition artifacts. The aggregate archive is
retained separately.

Acceptance binds each store index to its manifest, original ZIP and original
API metadata, then compares every restored file with its original ZIP bytes.
The frozen phase implementation validates the native tool, generation,
producer, fixtures, source pins and resource profile. Both self-builds must
match the original S1 compiler, source inventory, JOB and all execution fields
except `elapsed_micros`. Each phase requires its own successful original job
and raw job log; the aggregate has a separate job identity.

The local replay runs the exact frozen phase CLI with a local origin. A second
comparison explicitly requires the original CI origin. Both must bind the same
36 receipt hashes as the original successful CI aggregate. The original final
run must also be successful. An incomplete or failed native run stays open.

## Guard results and replay

`preparation/guards.json` records commands, source hashes and input hashes at
root receipt revision `2a5cb70` plus the two exact uncommitted Python files.
There are 19 distinct guards, each passing under ordinary and optimized Python
with warnings promoted to errors. The optimized run repeats the same guards.
The original S1 receipts are measured local receipts, bound to the hashes in
the frozen expected-input contract. Tiny archive/job fixtures exercise rejection
paths and never stand in for successful CI evidence.

Independent review identified and fixed a missing store-manifest binding and
missing distinct-job binding before acceptance. The real S1 regression also
exposed the draft's incorrect elapsed-field name. Its original failing output
and draft validator are retained under `preparation/elapsed-field*`; the final
guard logs and source hashes describe the corrected implementation.

To repeat the guard checks, run from the Trident checkout. Use a fresh temporary
directory for the decompressed inputs:

```sh
python3 - <<'PY'
import gzip
from pathlib import Path
import tempfile

base = Path('audit/self-hosting/bootstrap-results/run-36382085561/final-validation')
output = Path(tempfile.mkdtemp(prefix='sh6-phase-guards-'))
for part in ('inputs', 'references'):
    for source in (base / 'preparation' / part).rglob('*.gz'):
        destination = output / part / source.relative_to(base / 'preparation' / part).with_suffix('')
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(gzip.decompress(source.read_bytes()))
print(output)
PY
```

Pass the printed directory's `inputs` and `references` subdirectories to
`test-final-matrix.py --inputs ... --references ... -v`, first with
`python3 -B -W error`, then with `python3 -B -O -W error`.

## Actual acceptance inputs

`verify-final-matrix.py --help` lists required arguments. `--restored` is a fresh
directory containing the 36 original artifact directories; `--stores` contains
the three archive stores. `--indices` is a JSON mapping their role names to
final index SHA256 values, additionally pinned by `--indices-sha256`.

`--inputs` contains original `artifact-ID.json`, `artifact-ID.zip`,
`job-ID-direct.json` and `job-ID-api.log` downloads. `--runner-directory` holds
the three frozen implementation files. `--expected` is the exact retained
expected-input contract. `--aggregate-id`, `--aggregate-job` and `--final-run`
identify the original aggregate artifact, its direct job response and the final
run response. `--output` must be fresh and separate from every input. A final
acceptance report can be produced only after these actual results exist.
