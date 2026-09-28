#!/usr/bin/env python3
"""Retain the original failed CI aggregate without relabeling local replay."""
import argparse
import gzip
import json
import os
from pathlib import Path
import runpy
import subprocess
import sys
import time
import zipfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inputs", required=True, type=Path)
    parser.add_argument("--work", required=True, type=Path)
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    common = runpy.run_path(str(here / "retain-component.py"))
    require, sha, digest, load, write = (common[key] for key in ("require", "sha", "digest", "load", "write"))
    require(sys.flags.optimize == 0, "unoptimized Python required")
    destination = here / "aggregate"
    require(not args.work.exists() and not destination.exists(), "fresh work and receipt directories required")
    archiver = here.parent / "archive-tool/archive.py"
    runner = args.inputs / "bootstrap-runner-at-23691cd.py"
    require(sha(archiver) == common["ARCHIVER_SHA"] and sha(runner) == common["RUNNER_SHA"], "frozen tools")
    archive = args.inputs / "artifact-10950680824.zip"
    metadata = args.inputs / "artifact-10950680824.json"
    meta = load(metadata)
    require(meta["id"] == 10950680824 and meta["name"] == "bootstrap-matrix-comparison", "original aggregate artifact")
    require(meta["workflow_run"]["id"] == common["RUN"] and meta["workflow_run"]["head_sha"] == common["HEAD"], "origin")
    require(meta["digest"] == "sha256:" + sha(archive) and meta["size_in_bytes"] == archive.stat().st_size, "ZIP identity")
    job_path = args.inputs / "job-108776544043-direct.json"
    job = load(job_path)
    require(job["id"] == 108776544043 and job["run_id"] == common["RUN"] and
            job["head_sha"] == common["HEAD"] and job["status"] == "completed" and job["conclusion"] == "failure", "failed CI job")
    with zipfile.ZipFile(archive) as source:
        require(source.namelist() == ["receipt.json"], "aggregate ZIP members")
        raw_receipt = source.read("receipt.json")
    actual = json.loads(raw_receipt)
    require(actual["schema"] == "trident/clean-bootstrap/v1" and actual["status"] == "failed" and
            actual["error"] == "ValueError: bootstrap not passed", "original failed aggregate")
    names = set()
    for row in actual["platform_reports"]:
        name = Path(row["path"]).parent.name
        require(name not in names and sha(args.inputs / "platforms" / name / "receipt.json") == row["sha256"], "producer receipt binding")
        names.add(name)
    require(len(names) == 6, "all six original platform receipts")
    args.work.mkdir(parents=True)
    report = dict(scope="Original failed aggregate retention; local replay is separate; no acceptance",
                  status="running", matrix_acceptance=False, run_id=common["RUN"], head_sha=common["HEAD"],
                  command=[sys.executable, *sys.argv], collector_sha256=sha(Path(__file__)),
                  component_helper_sha256=sha(here / "retain-component.py"),
                  archive_sha256=sha(archive), aggregate_sha256=digest(raw_receipt),
                  runner_sha256=common["RUNNER_SHA"], archiver_sha256=common["ARCHIVER_SHA"],
                  all_six_original_receipt_hashes_match=True, commands=[], started_ns=time.time_ns())
    files = {}

    def command(label, argv, expected=0):
        env = os.environ.copy()
        env.pop("PYTHONOPTIMIZE", None)
        env["PYTHONDONTWRITEBYTECODE"] = "1"
        started = time.time_ns()
        result = subprocess.run(list(map(str, argv)), cwd=here.parents[3], env=env, capture_output=True)
        files[label + ".stdout"] = result.stdout
        files[label + ".stderr"] = result.stderr
        report["commands"].append(dict(command=list(map(str, argv)), exit_code=result.returncode,
            started_ns=started, finished_ns=time.time_ns(),
            stdout=dict(bytes=len(result.stdout), sha256=digest(result.stdout)),
            stderr=dict(bytes=len(result.stderr), sha256=digest(result.stderr))))
        require(result.returncode == expected, label + " unexpected exit")

    try:
        store = here / "platform-store"
        command("import", [sys.executable, archiver, "import", "--zip", archive, "--metadata", metadata,
            "--run-id", common["RUN"], "--head-sha", common["HEAD"], "--name", meta["name"], "--store", store])
        files["index.json"] = (store / "index.json").read_bytes()
        report["index_sha256"] = digest(files["index.json"])
        command("restore", [sys.executable, archiver, "restore", "--store", store, "--output", args.work / "restored",
            "--index-sha256", report["index_sha256"]])
        require((args.work / "restored" / meta["name"] / "receipt.json").read_bytes() == raw_receipt, "exact aggregate restoration")
        command("original-matrix-local-replay", [sys.executable, "-W", "error", runner,
            "--matrix", args.inputs / "platforms", "--output", args.work / "local-replay"], expected=1)
        local_path = args.work / "local-replay/receipt.json"
        local = load(local_path)
        require(local["status"] == "failed" and local["error"] == actual["error"], "same original failure on local replay")
        files["local-replay.json"] = local_path.read_bytes()
        report["status"] = "retained"
    except BaseException as error:
        report.update(status="failed", error=f"{type(error).__name__}: {error}")
    finally:
        report["finished_ns"] = time.time_ns()
        files["retention.json"] = (json.dumps(report, indent=2) + "\n").encode()
        for name, path in (("job.json", job_path), ("job.log", args.inputs / "job-108776544043-api.log"),
            ("artifact-api.json", metadata), ("aggregate.json", args.inputs / "aggregate/bootstrap-matrix-comparison/receipt.json"),
            ("independent-validation.json", args.inputs / "aggregate-validation.json"),
            ("collector.py", Path(__file__))):
            files[name] = path.read_bytes()
        destination.mkdir()
        manifest = []
        for name, raw in files.items():
            stored = gzip.compress(raw, compresslevel=9, mtime=0)
            write(destination / (name + ".gz"), stored)
            manifest.append(dict(path=name + ".gz", raw_bytes=len(raw), raw_sha256=digest(raw),
                                 stored_bytes=len(stored), stored_sha256=digest(stored)))
        write(destination / "files.json", (json.dumps(dict(files=manifest), indent=2) + "\n").encode())
    print(json.dumps({key: report[key] for key in ("status", "matrix_acceptance")}))
    if report["status"] != "retained":
        print(report.get("error"), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
