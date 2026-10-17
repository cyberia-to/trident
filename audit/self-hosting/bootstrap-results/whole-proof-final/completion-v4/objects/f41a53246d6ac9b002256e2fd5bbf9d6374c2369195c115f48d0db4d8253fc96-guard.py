"""Bound one owned diagnostic command and retain its complete receipt."""
import hashlib
import json
import os
from pathlib import Path
import resource
import selectors
import sys
import shutil
import signal
import stat
import subprocess
import time
import traceback

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT.parent / "whole-proof-attacks-completion-v4"))
import resources as shared
import ownership
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
    shared.write(path, value)


def owned_bytes():
    return sum(p.stat().st_size for p in ROOT.rglob("*")
               if p.is_file() and "target-helper" not in p.relative_to(ROOT).parts)


def stop(child, binding):
    return ownership.stop_child(child, binding)


def run(name, argv, kind, inputs, expected_exit=0, metadata=None):
    if Path(name).name != name or name in (".", ".."):
        raise ValueError("one owned attempt name")
    generation = shared.admitted(ROOT)
    caps = dict(CAPS[kind])
    if kind == "helper":
        caps["file"] = min(caps["file"], shared.PLAN["proofs"][str(generation)]["bytes"] +
                           shared.PLAN["mutant_growth_ceiling_bytes"])
    argv = list(map(str, argv))
    shared.authorize_native(generation, name, argv, kind)
    if not Path(argv[0]).is_absolute():
        raise ValueError("absolute executable")
    paths = list(dict.fromkeys([argv[0], *map(str, inputs)]))
    before = {p: identity(p) for p in paths}
    if shutil.disk_usage(ROOT).free < 8 * GIB:
        raise ValueError("free disk floor")
    directory = ROOT / "attempts" / name
    directory.mkdir(parents=True, exist_ok=False)
    report = dict(schema="trident/whole-proof-attack-command/v2", status="prepared",
                  argv=argv, cwd=str(directory), environment={"PATH": ""},
                  caps=caps, sampled_scope_disk_cap=26 * GIB, free_floor=8 * GIB,
                  sample_interval_seconds=1, driver=identity(__file__),
                  shared_profile=identity(shared.ROOT / "plan.json"),
                  shared_driver=identity(shared.__file__),
                  admission=identity(shared.ROOT / "admission.json"),
                  per_stream_log_bytes=shared.PLAN["per_stream_log_bytes"],
                  inputs_before=before, expected_exit=expected_exit, metadata=metadata,
                  started_ns=time.time_ns(), sampled_peak_rss_bytes=0)
    def save():
        document(directory / "receipt.json", report)
    def limits():
        resource.setrlimit(resource.RLIMIT_FSIZE, (caps["file"], caps["file"]))
        resource.setrlimit(resource.RLIMIT_CPU, (caps["cpu"], caps["cpu"]))
        shared.register_native(name, argv)
    child = None
    binding = None
    started = time.monotonic()
    save()
    try:
        with (directory / "stdout").open("xb") as stdout, (directory / "stderr").open("xb") as stderr, (directory / "resources.jsonl").open("x") as samples:
            child = subprocess.Popen(argv, cwd=directory, env=dict(os.environ, PATH=""),
                                     stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                     start_new_session=True, preexec_fn=limits)
            registration = shared.ROOT / "native-processes" / (name + ".json")
            binding = json.loads(registration.read_text())
            report.update(status="running", pid=child.pid, process_binding=binding)
            save()
            selector = selectors.DefaultSelector()
            for pipe, dest in ((child.stdout, stdout), (child.stderr, stderr)):
                os.set_blocking(pipe.fileno(), False)
                selector.register(pipe, selectors.EVENT_READ, dest)
            next_sample = 0
            while child.poll() is None or selector.get_map():
                for key, _ in selector.select(timeout=0.2):
                    chunk = os.read(key.fileobj.fileno(), 65536)
                    if not chunk:
                        selector.unregister(key.fileobj)
                        key.fileobj.close()
                        continue
                    remaining = shared.PLAN["per_stream_log_bytes"] - key.data.tell()
                    key.data.write(chunk[:max(0, remaining)])
                    key.data.flush()
                    if len(chunk) > remaining:
                        report["resource_stop"] = "log-bytes"
                        shared.fail("log-bytes", {"attempt": name, "stream": key.data.name})
                        report["cleanup"] = stop(child, binding)
                now = time.monotonic()
                if now < next_sample:
                    continue
                next_sample = now + 1
                observed = ownership.observe(binding)
                if observed["status"] == "unbound":
                    raise ValueError("native process birth changed")
                members = [dict(pid=row["pid"], rss_bytes=row["rss_kib"] * 1024,
                                started=row["started"], state=row["state"])
                           for row in observed["members"]]
                sample = dict(time_ns=time.time_ns(), elapsed=now - started,
                              processes=members, rss_bytes=sum(p["rss_bytes"] for p in members),
                              shared_stop=shared.stop_requested())
                report["sampled_peak_rss_bytes"] = max(report["sampled_peak_rss_bytes"], sample["rss_bytes"])
                report["latest_sample"] = sample
                samples.write(json.dumps(sample) + "\n")
                samples.flush()
                reason = ("wall" if sample["elapsed"] > caps["wall"] else
                          "rss" if sample["rss_bytes"] > caps["rss"] else
                          "shared-stop" if sample["shared_stop"] else None)
                if reason:
                    report["resource_stop"] = reason
                    shared.fail(reason, {"attempt": name})
                    report["cleanup"] = stop(child, binding)
                    break
                save()
            selector.close()
            report.update(exit_code=child.wait(timeout=5), elapsed_seconds=time.monotonic() - started)
            final_group = ownership.observe(binding)
            report["final_group"] = final_group
            if final_group["status"] != "empty":
                report["cleanup"] = stop(child, binding)
                raise ValueError("native command left a nonempty group")
        report["status"] = "passed" if report["exit_code"] == expected_exit and "resource_stop" not in report else "failed"
    except BaseException:
        report.update(status="failed", error=traceback.format_exc())
        try:
            report["cleanup"] = stop(child, binding)
        except BaseException:
            report["cleanup_error"] = traceback.format_exc()
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
