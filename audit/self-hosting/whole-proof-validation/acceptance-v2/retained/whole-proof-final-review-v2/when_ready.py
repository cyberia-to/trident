"""Invoke the reviewed final evidence checker after all actual SH8 gates pass."""
import hashlib
import json
from pathlib import Path
import signal
import subprocess
import sys
import time
import traceback

ROOT = Path(__file__).resolve().parent
BASE = ROOT.parent
CHECKER = "503bb17b57ec2141788f1536b1750e875b5d1f8b108751e40e30d58ab2658c9c"
REVIEW = "0cb51c3c43f7de218d885f9d67dadd64ea1b0c81d63e96e108cabd4e79b5155e"


def identity(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def cancel(signum, _frame):
    raise InterruptedError("final evidence scheduler signal: " + str(signum))


def main():
    signal.signal(signal.SIGTERM, cancel)
    output = ROOT / "orchestration"
    output.mkdir()
    report = dict(status="waiting", started_ns=time.time_ns(), checker_sha256=CHECKER,
                  independent_review_sha256=REVIEW, runtime_timeout_seconds=3600,
                  readiness_timeout_seconds=24 * 3600)
    child = None
    paths = [BASE / f"whole-proof/attempts/c{n}-{stage}-1/receipt.json"
             for n in (1, 2) for stage in ("selfbuild", "fresh-verification")]
    paths += [BASE / "whole-proof-attacks-parallel-v2/run-1/receipt.json",
              BASE / "whole-proof-corpus-v2/orchestration/receipt.json"]

    def save():
        temp = output / "receipt.next.json"
        temp.write_text(json.dumps(report, indent=2) + "\n")
        temp.replace(output / "receipt.json")

    try:
        while True:
            states = {}
            for path in paths:
                try:
                    states[str(path)] = json.loads(path.read_text())["status"]
                except (FileNotFoundError, json.JSONDecodeError):
                    states[str(path)] = "waiting"
            report["observed_statuses"] = states
            save()
            if any(status not in ("waiting", "prepared", "running", "passed") for status in states.values()):
                raise ValueError("an actual SH8 gate failed; final acceptance checker was not launched")
            if all(status == "passed" for status in states.values()):
                break
            if time.time_ns() - report["started_ns"] > 24 * 3600 * 10**9:
                raise TimeoutError("SH8 evidence readiness deadline")
            time.sleep(30)
        if identity(ROOT / "check.py") != CHECKER or identity(BASE / "whole-proof-final-review-v2-independent/final-review.json") != REVIEW:
            raise ValueError("independently reviewed checker identity changed")
        command = [sys.executable, "-B", str(ROOT / "check.py"), "--base", str(BASE),
                   "--output", str(output / "final-review.json")]
        with (output / "stdout").open("xb") as out, (output / "stderr").open("xb") as err:
            child = subprocess.Popen(command, stdout=out, stderr=err, start_new_session=True)
            report.update(status="running", command=command, pid=child.pid)
            save()
            code = child.wait(timeout=3600)
        report["exit_code"] = code
        if code or json.loads((output / "final-review.json").read_text())["status"] != "passed":
            raise ValueError("actual final evidence review did not pass")
        report["status"] = "passed"
    except BaseException:
        if child and child.poll() is None:
            child.terminate()
            child.wait(timeout=30)
        report.update(status="failed", error=traceback.format_exc())
        raise
    finally:
        report["ended_ns"] = time.time_ns()
        save()


if __name__ == "__main__":
    main()
