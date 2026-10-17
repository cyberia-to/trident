#!/usr/bin/env python3
"""Retain one original-run component; never grant matrix acceptance."""
import argparse
import gzip
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import zipfile

RUN = 36353842247
HEAD = "23691cd2c6885bf25bfc023799552559724dbc2b"
RUNNER_SHA = "805af512ad48b99fc77b2e476ef6b1e67e7068ca4c4304a59a366c06110cb0f1"
WORKFLOW_SHA = "7af25add1b3a94e35bf9964db80cb3bc798ff5d6446e06cb37fc19bf5729b975"
ARCHIVER_SHA = "acef178c58f48005c30ed7a300f84fc5ed8c5ab0ddcf498b318bb74ec2ec4e4e"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def sha(path):
    return digest(path.read_bytes())


def load(path):
    return json.loads(path.read_bytes())


def write(path, data):
    with path.open("xb") as output:
        output.write(data)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inputs", required=True, type=Path)
    parser.add_argument("--work", required=True, type=Path)
    parser.add_argument("--artifact-id", required=True, type=int)
    parser.add_argument("--job-id", required=True, type=int)
    parser.add_argument("--target", required=True)
    parser.add_argument("--label", required=True)
    args = parser.parse_args()
    require(sys.flags.optimize == 0, "unoptimized Python required")
    require(re.fullmatch(r"[a-z0-9-]+", args.label) is not None, "portable label required")
    here = Path(__file__).resolve().parent
    repo = here.parents[3]
    destination = here / args.label
    require(not destination.exists() and not args.work.exists(), "fresh work and receipt directories required")
    archiver = here.parent / "archive-tool/archive.py"
    require(sha(archiver) == ARCHIVER_SHA, "reviewed archiver identity")
    runner = args.inputs / "bootstrap-runner-at-23691cd.py"
    workflow = args.inputs / "selfhost-bootstrap-at-23691cd.yml"
    require(sha(runner) == RUNNER_SHA and sha(workflow) == WORKFLOW_SHA, "original runner/workflow identity")
    for path, original in ((runner, "audit/self-hosting/bootstrap-runner.py"),
                           (workflow, ".github/workflows/selfhost-bootstrap.yml")):
        require(path.read_bytes() == subprocess.check_output(["git", "show", HEAD + ":" + original], cwd=repo),
                "snapshot differs from original commit")
    pins = dict(re.findall(r'"([a-z]+)":"([0-9a-f]{40})"', workflow.read_text()))
    pins["trident"] = HEAD
    require(len(pins) == 9, "exact original nine pins required")
    metadata = args.inputs / f"artifact-{args.artifact_id}.json"
    archive = args.inputs / f"artifact-{args.artifact_id}.zip"
    meta = load(metadata)
    name = "bootstrap-" + args.target
    require(meta["id"] == args.artifact_id and meta["name"] == name, "artifact identity")
    require(meta["workflow_run"]["id"] == RUN and meta["workflow_run"]["head_sha"] == HEAD,
            "original artifact origin")
    require(meta["digest"] == "sha256:" + sha(archive) and meta["size_in_bytes"] == archive.stat().st_size,
            "original ZIP digest/size")
    job_path = args.inputs / f"job-{args.job_id}-direct.json"
    job_log = args.inputs / f"job-{args.job_id}-api.log"
    job = load(job_path)
    require(job["id"] == args.job_id and job["run_id"] == RUN and job["head_sha"] == HEAD,
            "direct job origin")
    require(job["name"] == "Bootstrap " + args.target and job["status"] == "completed", "completed native job")
    args.work.mkdir(parents=True)
    report = dict(schema="trident/original-bootstrap-retention/v1", status="running",
                  scope="Original v1 component retention only; original matrix remains red; no new-run acceptance",
                  matrix_acceptance=False, run_id=RUN, head_sha=HEAD, target=args.target,
                  command=[sys.executable, *sys.argv], collector_sha256=sha(Path(__file__)),
                  checkout_revision=subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip(),
                  archive_sha256=sha(archive), metadata_sha256=sha(metadata), archive_bytes=archive.stat().st_size,
                  runner_sha256=RUNNER_SHA, workflow_sha256=WORKFLOW_SHA, archiver_sha256=ARCHIVER_SHA,
                  pins=pins, commands=[], started_ns=time.time_ns())
    files = {}

    def retain(label, raw):
        files[label] = raw

    def command(label, argv):
        env = os.environ.copy()
        env.pop("PYTHONOPTIMIZE", None)
        env["PYTHONDONTWRITEBYTECODE"] = "1"
        started = time.time_ns()
        result = subprocess.run([str(value) for value in argv], cwd=repo, env=env, capture_output=True)
        retain(label + ".stdout", result.stdout)
        retain(label + ".stderr", result.stderr)
        row = dict(command=[str(value) for value in argv], cwd=str(repo), exit_code=result.returncode,
                   started_ns=started, finished_ns=time.time_ns(),
                   stdout=dict(bytes=len(result.stdout), sha256=digest(result.stdout)),
                   stderr=dict(bytes=len(result.stderr), sha256=digest(result.stderr)))
        report["commands"].append(row)
        require(result.returncode == 0, label + " failed")

    try:
        store = here / "platform-store"
        command("import", [sys.executable, archiver, "import", "--zip", archive, "--metadata", metadata,
                           "--run-id", RUN, "--head-sha", HEAD, "--name", name, "--store", store])
        index = (store / "index.json").read_bytes()
        report["index_sha256"] = digest(index)
        retain("index.json", index)
        restored = args.work / "restored"
        command("restore", [sys.executable, archiver, "restore", "--store", store, "--output", restored,
                            "--index-sha256", digest(index)])
        tree = restored / name
        with zipfile.ZipFile(archive) as source:
            members = [row for row in source.infolist() if not row.is_dir()]
            require({row.filename for row in members} == {p.relative_to(tree).as_posix() for p in tree.rglob("*") if p.is_file()},
                    "ZIP/restored file set")
            for row in members:
                require(source.read(row) == (tree / row.filename).read_bytes(), "ZIP/restored bytes")
            report.update(restored_files=len(members), restored_bytes=sum(row.file_size for row in members),
                          all_zip_file_bytes_equal=True)
        spec = importlib.util.spec_from_file_location("original_bootstrap", runner)
        original = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(original)
        require(args.target in original.TARGETS, "original native target")
        raw_report = load(tree / "receipt.json")
        require(raw_report["pins"] == pins and raw_report["runner_sha256"] == RUNNER_SHA, "producer source identities")
        require(raw_report["target"] == args.target and raw_report["rust_version"] == "1.95.0", "native target/toolchain")
        report.update(receipt_sha256=sha(tree / "receipt.json"), producer_status=raw_report["status"],
                      producer_error=raw_report.get("error"),
                      job_id=args.job_id, job_conclusion=job["conclusion"],
                      command_count=len(raw_report["commands"]),
                      unsuccessful_commands=[c for c in raw_report["commands"] if c.get("exit_code") != 0])
        if raw_report["status"] == "passed":
            require(job["conclusion"] == "success", "passed receipt requires successful job")
            report["compiler_sha256"] = original.compare([(tree, raw_report)])
            report["component_comparison"] = "passed"
        else:
            report["component_comparison"] = "excluded-nonpassed-receipt"
        report["repetitions"] = []
        for row in raw_report["repetitions"]:
            summary = dict(number=row["number"], status=row.get("status", "incomplete"), corpora={})
            for label, identity in row["corpora"].items():
                corpus = load(tree / identity["path"])
                builds = [c for c in corpus["commands"] if len(c["command"]) > 1 and c["command"][1] == "build"]
                require(all(c.get("reference_only") is True for c in builds), "unlabeled reference build")
                summary["corpora"][label] = dict(observations=len(corpus["observations"]),
                                                commands=len(corpus["commands"]), reference_builds=len(builds))
            for generation in (2, 3):
                if f"step{generation}" in row:
                    step = load(tree / row[f"step{generation}"]["path"])
                    summary[f"step{generation}"] = step.get("execution", {}).get("execution")
            report["repetitions"].append(summary)
        warnings = []
        for c in raw_report["commands"]:
            if c["command"][:2] == ["cargo", "build"]:
                warnings.extend(line for line in (tree / c["stderr"]).read_text().splitlines()
                                if re.match(r"^warning(?::|\[)", line))
        report["rust_warning_lines"] = warnings
        require(not warnings, "native Rust build warnings")
        require(sha(archive) == report["archive_sha256"] and sha(metadata) == report["metadata_sha256"], "inputs changed")
        report["status"] = "retained"
    except BaseException as error:
        report.update(status="failed", error=f"{type(error).__name__}: {error}")
    finally:
        report["finished_ns"] = time.time_ns()
        retain("retention.json", (json.dumps(report, indent=2) + "\n").encode())
        for label, path in (("job.json", job_path), ("job.log", job_log),
                            ("artifact-api.json", metadata), ("original-runner.py", runner),
                            ("original-workflow.yml", workflow), ("collector.py", Path(__file__))):
            retain(label, path.read_bytes())
        destination.mkdir()
        manifest = []
        for label, raw in files.items():
            stored = gzip.compress(raw, compresslevel=9, mtime=0)
            write(destination / (label + ".gz"), stored)
            manifest.append(dict(path=label + ".gz", raw_bytes=len(raw), raw_sha256=digest(raw),
                                 stored_bytes=len(stored), stored_sha256=digest(stored)))
        write(destination / "files.json", (json.dumps(dict(files=manifest), indent=2) + "\n").encode())
    print(json.dumps({key: report[key] for key in ("status", "matrix_acceptance", "target")}))
    if report["status"] != "retained":
        print(report.get("error"), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
