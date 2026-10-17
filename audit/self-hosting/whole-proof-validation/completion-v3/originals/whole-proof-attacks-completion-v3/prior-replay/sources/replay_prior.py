import hashlib, json, sys, time, traceback
from pathlib import Path
from prior_cases import review
ROOT = Path(__file__).resolve().parent
BASE = ROOT.parent
output = ROOT / "prior-replay"
output.mkdir()
report = {"status": "running", "command": [sys.executable, *sys.argv], "started_ns": time.time_ns(), "sources": {name: hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in ("prior_cases.py", "replay_prior.py")}}
try:
    for number in (1, 2):
        proof = BASE / f"whole-proof/attempts/c{number}-selfbuild-1/proof.joysc"
        verifier = BASE / f"whole-proof/attempts/c{number}-fresh-verification-1"
        result = json.loads((verifier/"stdout").read_text())["verification"]
        actual = review(BASE, number, proof, verifier, result)
        (output/f"c{number}.json").write_text(json.dumps(actual, indent=2)+"\n")
    report["status"] = "passed"
except BaseException:
    report.update(status="failed", error=traceback.format_exc())
    raise
finally:
    report["ended_ns"] = time.time_ns()
    (output/"receipt.json").write_text(json.dumps(report, indent=2)+"\n")
print(report["status"])
