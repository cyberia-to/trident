"""Bound one owned diagnostic command and retain its complete receipt."""
import hashlib
import json
import os
from pathlib import Path
import resource
import shutil
import signal
import stat
import subprocess
import time
import traceback

ROOT = Path(__file__).resolve().parent
GIB = 1024**3
CAPS = {
    "pack": dict(wall=120, cpu=120, rss=2 * GIB, file=32 * 1024**2),
    "helper": dict(wall=1800, cpu=1800, rss=GIB, file=24 * GIB + 65536),
    "verify": dict(wall=7500, cpu=7500, rss=6 * GIB, file=32 * 1024**2),
}


def identity(path):
    path = Path(path)
    mode = path.lstat().st_mode
    if not stat.S_ISREG(mode):
        raise ValueError("identity requires a regular file: " + str(path))
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    return {"bytes": path.stat().st_size, "sha256": digest}


def document(path, value):
    Path(path).write_text(json.dumps(value, indent=2) + "\n")


def owned_bytes():
    return sum(p.stat().st_size for p in ROOT.rglob("*")
               if p.is_file() and "target-helper" not in p.relative_to(ROOT).parts)


def stop(child):
    if child is None or child.poll() is not None:
        return
    try:
        os.killpg(child.pid, signal.SIGTERM)
    except ProcessLookupError:
        return
    try:
        child.wait(timeout=10)
    except subprocess.TimeoutExpired:
        try:
            os.killpg(child.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        child.wait()


def run(name, argv, kind, inputs, expected_exit=0, metadata=None):
    if Path(name).name != name or name in (".", ".."):
        raise ValueError("one owned attempt name")
    caps = CAPS[kind]
    argv = list(map(str, argv))
    if not Path(argv[0]).is_absolute():
        raise ValueError("absolute executable")
    paths = list(dict.fromkeys([argv[0], *map(str, inputs)]))
    before = {p: identity(p) for p in paths}
    if shutil.disk_usage(ROOT).free < 8 * GIB:
        raise ValueError("free disk floor")
    directory = ROOT / "attempts" / name
    directory.mkdir(parents=True, exist_ok=False)
    report = dict(schema="trident/whole-proof-attack-command/v1", status="prepared",
                  argv=argv, cwd=str(directory), environment={"PATH": ""},
                  caps=caps, sampled_scope_disk_cap=26 * GIB, free_floor=8 * GIB,
                  sample_interval_seconds=1, driver=identity(__file__),
                  inputs_before=before, expected_exit=expected_exit, metadata=metadata,
                  started_ns=time.time_ns(), sampled_peak_rss_bytes=0)
    def save():
        document(directory / "receipt.json", report)
    def limits():
        resource.setrlimit(resource.RLIMIT_FSIZE, (caps["file"], caps["file"]))
        resource.setrlimit(resource.RLIMIT_CPU, (caps["cpu"], caps["cpu"]))
    child = None
    started = time.monotonic()
    save()
    try:
        with (directory / "stdout").open("xb") as stdout, (directory / "stderr").open("xb") as stderr, (directory / "resources.jsonl").open("x") as samples:
            child = subprocess.Popen(argv, cwd=directory, env=dict(os.environ, PATH=""),
                                     stdout=stdout, stderr=stderr, start_new_session=True,
                                     preexec_fn=limits)
            report.update(status="running", pid=child.pid)
            save()
            while child.poll() is None:
                rows = subprocess.check_output(["/bin/ps", "-axo", "pid=,pgid=,rss="], text=True)
                members = [dict(pid=p, rss_bytes=rss * 1024)
                           for p, group, rss in (map(int, row.split()) for row in rows.splitlines())
                           if group == child.pid]
                sample = dict(time_ns=time.time_ns(), elapsed=time.monotonic() - started,
                              processes=members, rss_bytes=sum(p["rss_bytes"] for p in members),
                              owned_bytes=owned_bytes(), free_bytes=shutil.disk_usage(ROOT).free)
                report["sampled_peak_rss_bytes"] = max(report["sampled_peak_rss_bytes"], sample["rss_bytes"])
                report["latest_sample"] = sample
                samples.write(json.dumps(sample) + "\n")
                samples.flush()
                reason = ("wall" if sample["elapsed"] > caps["wall"] else
                          "rss" if sample["rss_bytes"] > caps["rss"] else
                          "owned-disk" if sample["owned_bytes"] > 26 * GIB else
                          "free-disk" if sample["free_bytes"] < 8 * GIB else None)
                if reason:
                    report["resource_stop"] = reason
                    stop(child)
                    break
                save()
                try:
                    child.wait(timeout=1)
                except subprocess.TimeoutExpired:
                    pass
            report.update(exit_code=child.wait(), elapsed_seconds=time.monotonic() - started)
        report["status"] = "passed" if report["exit_code"] == expected_exit and "resource_stop" not in report else "failed"
    except BaseException:
        stop(child)
        report.update(status="failed", error=traceback.format_exc())
        raise
    finally:
        try:
            report["inputs_after"] = {p: identity(p) for p in paths}
            if report["inputs_after"] != before:
                report["status"] = "input-changed"
        except BaseException:
            report.update(status="input-check-failed", input_error=traceback.format_exc())
        report.update(ended_ns=time.time_ns(), files={p.name: identity(p) for p in directory.iterdir()
                      if p.is_file() and p.name != "receipt.json"})
        save()
    if report["status"] != "passed":
        raise RuntimeError("diagnostic command failed: " + str(directory / "receipt.json"))
    return report
