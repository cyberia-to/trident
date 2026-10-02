"""One reviewed post-stop transition for one completely classified temporary."""
import argparse
import contextlib
import datetime
import calendar
import re
import fcntl
import hashlib
import json
import os
from pathlib import Path
import signal
import stat
import subprocess
import sys
import time
import traceback

ROOT = Path(__file__).resolve().parent
BASE = ROOT.parent
TARGET = "whole-proof-attacks-v2-c2/whole-c2/certificate-rebound-job-limit.joysc"
EXPECTED = dict(bytes=10569174820, sha256="2f6ce311471e969b92d2d23fda33077bfe7bd3895fe2d415c8e6e0c08b94a4ae")


def require(value, message):
    if not value:
        raise ValueError(message)


def identity(path):
    require(stat.S_ISREG(path.lstat().st_mode), "regular retained file")
    with path.open("rb") as stream:
        return dict(bytes=path.stat().st_size, sha256=hashlib.file_digest(stream, "sha256").hexdigest())


def state(value):
    return dict(device=value.st_dev, inode=value.st_ino, bytes=value.st_size,
                mtime_ns=value.st_mtime_ns, ctime_ns=value.st_ctime_ns)


def load(path):
    return json.loads(path.read_text())


def sync_directory(path):
    descriptor = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def save(path, value):
    with path.open("w") as stream:
        json.dump(value, stream, indent=2)
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())
    sync_directory(path.parent)


def unlink_exact(path, expected_state, expected_identity, before_unlink, after_unlink=None):
    """Hold the classified inode open and refuse replacements or changed bytes."""
    require(path.resolve() == path and stat.S_ISREG(path.lstat().st_mode), "exact regular owned path")
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW), "rb") as stream:
        require(state(os.fstat(stream.fileno())) == expected_state, "classified inode/state changed")
        require(os.fstat(stream.fileno()).st_nlink == 1, "unique owned temporary link")
        measured = dict(bytes=expected_state["bytes"], sha256=hashlib.file_digest(stream, "sha256").hexdigest())
        require(measured == expected_identity, "classified temporary bytes changed")
        require(state(os.fstat(stream.fileno())) == state(path.lstat()) == expected_state,
                "temporary changed during complete hash")
        before_unlink(measured)
        require(state(os.fstat(stream.fileno())) == state(path.lstat()) == expected_state,
                "temporary changed after durable classification")
        path.unlink()
        if after_unlink is not None:
            after_unlink()
        sync_directory(path.parent)
        require(not path.exists() and os.fstat(stream.fileno()).st_nlink == 0,
                "only the held classified inode was unlinked")
    return measured


ROUNDING_MARGIN_NS = 2_000_000_000
MONTHS = {name: i for i, name in enumerate(
    ("Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"), 1)}


def birth_ns(value):
    """Parse ps lstart in explicit C locale and UTC, with no locale inference."""
    match = re.fullmatch(r"(Mon|Tue|Wed|Thu|Fri|Sat|Sun) ([A-Z][a-z]{2}) +([0-9]{1,2}) "
                         r"([0-9]{2}):([0-9]{2}):([0-9]{2}) ([0-9]{4})", value)
    require(match is not None, "ambiguous process birth timestamp: " + value)
    weekday, month, day, hour, minute, second, year = match.groups()
    require(month in MONTHS, "unknown process birth month")
    try:
        date = datetime.datetime(int(year), MONTHS[month], int(day), int(hour),
                                 int(minute), int(second), tzinfo=datetime.timezone.utc)
    except ValueError as error:
        raise ValueError("invalid process birth date") from error
    require(("Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun")[date.weekday()] == weekday,
            "inconsistent process birth weekday")
    return calendar.timegm(date.utctimetuple()) * 1_000_000_000


def classify_processes(raw, groups, shutdown_ns, observed_ns):
    """Only births strictly beyond the empty-group shutdown plus 2s are reuse."""
    require(type(shutdown_ns) is int and shutdown_ns > 0 and
            type(observed_ns) is int and observed_ns >= shutdown_ns,
            "invalid shutdown/snapshot time ordering")
    require(groups and all(type(g) is int and g > 0 for g in groups), "original positive process IDs")
    seen, overlaps = set(), []
    for line in raw.decode("ascii", errors="strict").splitlines():
        parts = line.strip().split(maxsplit=2)
        require(len(parts) == 3 and all(p.isascii() and p.isdecimal() for p in parts[:2]),
                "malformed process snapshot row")
        pid, pgid = map(int, parts[:2])
        require(pid not in seen, "duplicate process snapshot PID")
        seen.add(pid)
        started = birth_ns(parts[2])
        if pid in groups or pgid in groups:
            overlaps.append(dict(pid=pid, pgid=pgid, birth_utc=parts[2], birth_ns=started, raw=line))
    require(seen, "empty process snapshot")
    blocked = [r for r in overlaps if not shutdown_ns + ROUNDING_MARGIN_NS < r["birth_ns"] <= observed_ns]
    require(not blocked, "live prior owner or ambiguous birth: " + json.dumps(blocked, sort_keys=True))
    return dict(snapshot_rows=len(seen), overlapping_rows=overlaps, reused_rows=overlaps,
                rounding_margin_ns=ROUNDING_MARGIN_NS, shutdown_ns=shutdown_ns,
                observed_ns=observed_ns, no_live_prior_owners=True)


def no_live_owners(output):
    prior = BASE / "whole-proof-attacks-parallel-v2"
    admission_path, shutdown_path = prior / "admission.json", prior / "shutdown.json"
    inputs = {str(p): identity(p) for p in (admission_path, shutdown_path)}
    admission, stopped = load(admission_path), load(shutdown_path)
    require(stopped["all_owned_groups_empty"] is True and stopped["remaining"] == [], "original bounded shutdown")
    shutdown = stopped["time_ns"]
    require(type(shutdown) is int and shutdown > 0, "exact shutdown timestamp")
    groups = {admission["pid"], *admission["children"].values()}
    for path in sorted((prior / "native-processes").glob("*.json")):
        retired = path.with_suffix(".retired")
        inputs.update({str(p): identity(p) for p in (path, retired)})
        record = load(path)
        require(record["coordinator"] == admission["pid"], "original native owner")
        retired_ns = int(retired.read_text().strip())
        require(0 < retired_ns <= shutdown, "native group was not retired before final empty shutdown")
        groups.add(record["pgid"])
    command = ["/bin/ps", "-axo", "pid=,pgid=,lstart="]
    environment = {"TZ": "UTC", "LC_ALL": "C", "LANG": "C"}
    result = subprocess.run(command, capture_output=True, timeout=15, env=dict(os.environ, **environment))
    observed = time.time_ns()
    (output / "processes.stdout").write_bytes(result.stdout)
    (output / "processes.stderr").write_bytes(result.stderr)
    require(result.returncode == 0, "actual process inventory")
    require(all(identity(Path(p)) == value for p, value in inputs.items()), "original owner records changed")
    classified = classify_processes(result.stdout, groups, shutdown, observed)
    return dict(command=command, environment=environment, checked_groups=sorted(groups),
                original_owner_records=inputs, stdout=identity(output / "processes.stdout"),
                stderr=identity(output / "processes.stderr"), **classified)


def execute(review_sha256):
    output = ROOT / "reclamation-action-v2"
    output.mkdir()
    sync_directory(ROOT)
    sync_directory(output)
    report = dict(schema="trident/classified-temporary-reclamation/v1", status="running",
                  command=[sys.executable, *sys.argv], cwd=os.getcwd(), started_ns=time.time_ns(), removed=False)
    def persist():
        save(output / "receipt.json", report)
    persist()
    try:
        plan_path, review_path = ROOT / "reclamation-plan.json", ROOT / "reclamation-review-v2.json"
        require(identity(review_path)["sha256"] == review_sha256, "exact independent transition review")
        review, plan = load(review_path), load(plan_path)
        require(review["status"] == "passed-reclamation-source-review" and review["sources"] == {
            Path(__file__).name: identity(Path(__file__)), "reclamation-plan.json": identity(plan_path)
        }, "review binds exact action source and plan")
        require(plan["target"] == TARGET and plan["identity"] == EXPECTED, "only the classified C2 temporary")
        packet = BASE / "whole-reclamation-review"
        require(identity(packet / "classification.json") == plan["classification"], "exact independent classification")
        classified = load(packet / "classification.json")
        require(classified["status"] == "passed-readonly-classification", "completed read-only classification")
        require(identity(packet / "comparison.json") == classified["comparison"], "complete original comparison")
        comparison = load(packet / "comparison.json")["comparison"]
        require(comparison["states_before"][1] == comparison["states_after"][1] == plan["state"], "classified inode")
        require(classified["classification"]["completed_mutant"] == EXPECTED, "classified complete bytes")
        require(classified["classification"]["original_suite_status"] == "failed", "original failure stays failed")
        for name, expected in classified["retained"].items():
            require(identity(BASE / name) == expected and identity(packet / "retained" / name) == expected,
                    "original failure evidence and retained copy unchanged: " + name)
        with contextlib.ExitStack() as locks:
            for name in ["whole-proof-attacks-parallel-v2/coordinator.lock",
                         "whole-proof-attacks-v2-c1/whole-suite.lock", "whole-proof-attacks-v2-c2/whole-suite.lock",
                         "whole-proof-attacks/whole-suite.lock"]:
                lease = locks.enter_context((BASE / name).open("a+"))
                fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
            report["quiescence"] = no_live_owners(output)
            for name, expected in plan["retained_partials"].items():
                require(identity(BASE / name) == expected, "unfinished partial unchanged before transition")
            report.update(classification=plan["classification"], review=identity(review_path),
                          source=identity(Path(__file__)), plan=identity(plan_path), target=TARGET)
            def before_unlink(measured):
                report.update(status="validated-before-unlink", complete_mutant_identity=measured)
                persist()
            def after_unlink():
                report["removed"] = True
            unlink_exact(BASE / TARGET, plan["state"], EXPECTED, before_unlink, after_unlink)
            report.update(removed=True, status="reclaimed-classified-temporary")
            persist()
            for name, expected in plan["retained_partials"].items():
                require(identity(BASE / name) == expected, "unfinished partial unchanged after transition")
            for name, expected in classified["retained"].items():
                require(identity(BASE / name) == expected, "original failed evidence changed")
            report.update(status="passed", original_failed_evidence_unchanged=True,
                          retained_partials=plan["retained_partials"])
    except BaseException:
        report.update(status="failed", error=traceback.format_exc())
        raise
    finally:
        report["ended_ns"] = time.time_ns()
        persist()
    print(report["status"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--review-sha256", required=True)
    args = parser.parse_args()
    def timeout(_signum, _frame):
        raise TimeoutError("transition wall allowance")
    signal.signal(signal.SIGALRM, timeout)
    signal.alarm(600)
    execute(args.review_sha256)


if __name__ == "__main__":
    main()
