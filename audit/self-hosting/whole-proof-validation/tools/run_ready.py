"""Run reviewed whole-certificate checks only after actual producer/verifier success."""
import json
from pathlib import Path
import signal
import subprocess
import sys
import time
import traceback

from guard import document, identity, stop

ROOT = Path(__file__).resolve().parent
WHOLE = ROOT.parent / "whole-proof"


def load(path):
    try:
        return json.loads(path.read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        return None  # Running receipts are replaced by their owning producer.


def cancel(signum, _frame):
    raise InterruptedError("suite scheduler signal: " + str(signum))


def main():
    signal.signal(signal.SIGTERM, cancel)
    output = ROOT / "whole-orchestration"
    output.mkdir()
    review_path = ROOT.parent / "whole-proof-attacks-independent-review/receipt.json"
    review = load(review_path)
    if not review or review.get("status") != "passed-source-review":
        raise ValueError("independent exact-source review required")
    source = {name: identity(ROOT / name) for name in review["source"]}
    if source != review["source"]:
        raise ValueError("independent review source identities differ")
    report = dict(status="waiting", started_ns=time.time_ns(), source=source,
                  independent_review=identity(review_path), driver=identity(__file__), generations=[])
    child = None
    try:
        for generation in (2, 1):
            row = dict(generation=generation, status="waiting")
            report["generations"].append(row)
            while True:
                producer = load(WHOLE / f"attempts/c{generation}-selfbuild-1/receipt.json")
                verifier = load(WHOLE / f"attempts/c{generation}-fresh-verification-1/receipt.json")
                for name, receipt in (("producer", producer), ("verifier", verifier)):
                    if receipt and receipt["status"] not in ("prepared", "running", "passed"):
                        raise ValueError(f"c{generation} {name} failed; no suite launch")
                if producer and verifier and producer["status"] == verifier["status"] == "passed":
                    break
                if producer and time.time_ns() - producer["started_ns"] > 15500 * 10**9:
                    raise TimeoutError("producer plus fresh verifier readiness deadline")
                document(output / "receipt.json", report)
                time.sleep(5)
            if source != {name: identity(ROOT / name) for name in source}:
                raise ValueError("reviewed source changed before launch")
            command = [sys.executable, "-B", str(ROOT / "whole_suite.py"), "--generation", str(generation)]
            with (output / f"c{generation}.stdout").open("xb") as stdout, (output / f"c{generation}.stderr").open("xb") as stderr:
                child = subprocess.Popen(command, stdout=stdout, stderr=stderr, start_new_session=True)
                row.update(status="running", command=command, pid=child.pid, started_ns=time.time_ns())
                report["status"] = "running"
                document(output / "receipt.json", report)
                code = child.wait()
            row.update(exit_code=code, ended_ns=time.time_ns(), status="passed" if code == 0 else "failed")
            row["logs"] = {name: identity(output / f"c{generation}.{name}") for name in ("stdout", "stderr")}
            if code or load(ROOT / f"whole-c{generation}/receipt.json")["status"] != "passed":
                raise ValueError("whole suite failed; no subsequent generation")
            row["suite_receipt"] = identity(ROOT / f"whole-c{generation}/receipt.json")
            document(output / "receipt.json", report)
        report["status"] = "passed"
    except BaseException:
        stop(child)
        report.update(status="failed", error=traceback.format_exc())
        raise
    finally:
        report["ended_ns"] = time.time_ns()
        document(output / "receipt.json", report)


if __name__ == "__main__":
    main()
