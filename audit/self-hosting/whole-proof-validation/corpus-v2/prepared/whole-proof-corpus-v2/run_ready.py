"""Wait for each real full-proof verification, then run its extracted-compiler corpus."""
import hashlib
import json
from pathlib import Path
import signal
import subprocess
import sys
import time
import traceback

ROOT = Path(__file__).resolve().parent
WHOLE = ROOT.parent / "whole-proof"
FIXTURES = ROOT.parent / "corpus-path-fix/trident"
REVISION = "2f1a575a6fbc0704c268f1ae21667830a3c996df"
DRIVER_SHA = "2e0679d7792a8d720d590d5e2f22299b912c654e196fae46f0599e63a448534c"


def load(path):
    try:
        return json.loads(path.read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        return None


def cancel(signum, _frame):
    raise InterruptedError("corpus scheduler signal: " + str(signum))


def main():
    signal.signal(signal.SIGTERM, cancel)
    output = ROOT / "orchestration"
    output.mkdir()
    report = dict(status="waiting", driver_sha256=DRIVER_SHA, fixture_revision=REVISION, generations=[])
    child = None
    try:
        review = load(ROOT / "independent-review.json")
        if not review or review["status"] != "passed-source-review":
            raise ValueError("independent exact-source review required")
        manifest = load(ROOT / "sources.json")
        if review["sources"] != manifest:
            raise ValueError("independent review must bind complete source manifest")
        for path, expected in manifest.items():
            data = Path(path).read_bytes()
            if dict(bytes=len(data), sha256=hashlib.sha256(data).hexdigest()) != expected:
                raise ValueError("reviewed corpus source changed: " + path)
        report["reviewed_sources"] = manifest
        report["independent_review_sha256"] = hashlib.sha256((ROOT / "independent-review.json").read_bytes()).hexdigest()
        for generation in (2, 1):
            row = dict(proof_generation=generation, compiler_generation=generation + 1, status="waiting")
            report["generations"].append(row)
            while True:
                producer = load(WHOLE / f"attempts/c{generation}-selfbuild-1/receipt.json")
                verified = load(WHOLE / f"attempts/c{generation}-fresh-verification-1/receipt.json")
                for receipt in (producer, verified):
                    if receipt and receipt["status"] not in ("prepared", "running", "passed"):
                        raise ValueError("producer or verifier failed; no corpus launch")
                if producer and verified and producer["status"] == verified["status"] == "passed":
                    break
                if producer and time.time_ns() - producer["started_ns"] > 15500 * 10**9:
                    raise TimeoutError("producer plus fresh verification deadline")
                (output / "receipt.json").write_text(json.dumps(report, indent=2) + "\n")
                time.sleep(5)
            if hashlib.sha256((ROOT / "run.py").read_bytes()).hexdigest() != DRIVER_SHA:
                raise ValueError("independently reviewed source changed")
            command = [sys.executable, "-B", str(ROOT / "run.py"), "--generation", str(generation),
                       "--fixtures", str(FIXTURES), "--revision", REVISION]
            with (output / f"c{generation + 1}.stdout").open("xb") as out, (output / f"c{generation + 1}.stderr").open("xb") as err:
                child = subprocess.Popen(command, stdout=out, stderr=err, start_new_session=True)
                row.update(status="running", command=command, pid=child.pid, started_ns=time.time_ns())
                report["status"] = "running"
                (output / "receipt.json").write_text(json.dumps(report, indent=2) + "\n")
                code = child.wait()
            row.update(exit_code=code, ended_ns=time.time_ns(), status="passed" if code == 0 else "failed")
            if code or load(ROOT / f"c{generation + 1}/receipt.json")["status"] != "passed":
                raise ValueError("extracted compiler corpus failed; no next corpus")
        report["status"] = "passed"
    except BaseException:
        if child and child.poll() is None:
            child.terminate()
            child.wait(timeout=60)
        report.update(status="failed", error=traceback.format_exc())
        raise
    finally:
        (output / "receipt.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
