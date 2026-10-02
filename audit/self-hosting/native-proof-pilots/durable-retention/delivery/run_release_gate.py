"""Bound the complete unchanged Rust workspace test set and retain every output."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import time
import traceback


def identity(path):
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    return dict(bytes=path.stat().st_size, sha256=digest)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--environment-receipt", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    previous = json.loads(args.environment_receipt.read_text())
    env = dict(os.environ)
    for key in list(env):
        if key in ("RUSTC", "RUSTDOC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "RUSTFLAGS",
                   "RUSTDOCFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_ENCODED_RUSTDOCFLAGS",
                   "CARGO_BUILD_TARGET", "RUSTUP_TOOLCHAIN") or key.startswith("CARGO_TARGET_"):
            env.pop(key)
    env.update(previous["environment"])
    cargo = Path(env["RUSTC"]).with_name("cargo")
    versions = {name: subprocess.check_output([str(cargo.with_name(name)), "-vV"], env=env, text=True)
                for name in ("cargo", "rustc")}
    if not versions["rustc"].startswith("rustc 1.89.0 ") or "host: aarch64-apple-darwin" not in versions["rustc"]:
        raise ValueError("actual native Rust 1.89 required")
    args.output.mkdir()
    argv = [str(cargo), "test", "--release", "--workspace", "--locked", "--offline", "--", "--test-threads", "8"]
    report = dict(schema="trident/sh7-retention-release-test/v2", status="prepared", argv=argv,
        cwd=str(args.repo), environment=previous["environment"], versions=versions,
        base_commit=subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=args.repo, text=True).strip(),
        driver=identity(Path(__file__)), environment_receipt=identity(args.environment_receipt),
        wall_seconds=1800, sampled_group_rss_limit_bytes=4 * 1024**3, sample_interval_seconds=2,
        scope="Full unchanged workspace coverage; existing target directory; 8 test threads. "
              "Earlier partial attempts retained separately. Rust validation bounds only; proof caps unchanged.")

    def save():
        temporary = args.output / "receipt.pending.json"
        temporary.write_text(json.dumps(report, indent=2) + "\n")
        temporary.replace(args.output / "receipt.json")

    child = None

    def stop():
        if child is None or child.poll() is not None:
            return
        try:
            os.killpg(child.pid, signal.SIGTERM)
            child.wait(timeout=10)
        except ProcessLookupError:
            pass
        except subprocess.TimeoutExpired:
            os.killpg(child.pid, signal.SIGKILL)
            child.wait()

    def interrupted(signum, _frame):
        raise InterruptedError("owned runner signal " + str(signum))

    signal.signal(signal.SIGTERM, interrupted)
    started = time.monotonic()
    report["started_ns"] = time.time_ns()
    save()
    try:
        with (args.output / "stdout").open("xb") as stdout, (args.output / "stderr").open("xb") as stderr, \
                (args.output / "resources.jsonl").open("x") as samples:
            child = subprocess.Popen(argv, cwd=args.repo, env=env, stdout=stdout, stderr=stderr, start_new_session=True)
            report.update(status="running", pid=child.pid, sampled_peak_group_rss_bytes=0)
            save()
            while child.poll() is None:
                rows = subprocess.check_output(["/bin/ps", "-axo", "pid=,pgid=,rss="], text=True)
                members = [dict(pid=pid, rss_bytes=rss * 1024)
                    for pid, group, rss in (map(int, row.split()) for row in rows.splitlines()) if group == child.pid]
                rss = sum(row["rss_bytes"] for row in members)
                sample = dict(elapsed_seconds=time.monotonic() - started, time_ns=time.time_ns(), members=members, rss_bytes=rss)
                samples.write(json.dumps(sample) + "\n")
                samples.flush()
                report["sampled_peak_group_rss_bytes"] = max(report["sampled_peak_group_rss_bytes"], rss)
                if sample["elapsed_seconds"] > 1800 or rss > report["sampled_group_rss_limit_bytes"]:
                    report["resource_stop"] = "wall" if sample["elapsed_seconds"] > 1800 else "rss"
                    stop()
                    break
                save()
                try:
                    child.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    pass
            report.update(exit_code=child.wait(), elapsed_seconds=time.monotonic() - started)
        text = (args.output / "stdout").read_text() + (args.output / "stderr").read_text()
        report["warning_lines"] = [line for line in text.splitlines() if line.lstrip().startswith("warning:") or "warning[" in line]
        report["status"] = "passed" if report["exit_code"] == 0 and not report["warning_lines"] and "resource_stop" not in report else "failed"
    except BaseException:
        stop()
        report.update(status="failed", error=traceback.format_exc())
        raise
    finally:
        report["files"] = {p.name: identity(p) for p in args.output.iterdir() if p.is_file() and p.name != "receipt.json"}
        report["ended_ns"] = time.time_ns()
        save()
    print(json.dumps({key: report[key] for key in ("status", "exit_code", "elapsed_seconds", "sampled_peak_group_rss_bytes")}))
    if report["status"] != "passed":
        raise SystemExit(1)


if __name__ == "__main__":
    main()
