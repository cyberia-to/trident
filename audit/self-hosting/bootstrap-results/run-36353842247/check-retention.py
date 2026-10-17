#!/usr/bin/env python3
"""Check historical retention and safe collector rejection, not SH6 acceptance."""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inputs", type=Path, required=True)
    parser.add_argument("--work", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(not sys.flags.optimize, "unoptimized Python required")
    require(not args.output.exists(), "fresh output required")
    here = Path(__file__).resolve().parent
    repo = here.parents[3]
    index = here / "platform-store/index.json"
    index_before = index.read_bytes()
    require(sha(index_before) == "9f20e7f20437abd0ba7fa4fa521215bafac29ff5a62059ab9454833e15d89860", "final retained index")
    count = 0
    for label in ("linux-arm", "linux-x64", "windows-x64", "windows-arm", "mac-arm", "aggregate"):
        directory = here / label
        manifest = json.loads((directory / "files.json").read_bytes())["files"]
        require({p.name for p in directory.iterdir()} == {row["path"] for row in manifest} | {"files.json"}, "exact receipt files")
        for row in manifest:
            stored = (directory / row["path"]).read_bytes()
            raw = gzip.decompress(stored)
            require(len(stored) == row["stored_bytes"] and sha(stored) == row["stored_sha256"], "stored receipt identity")
            require(len(raw) == row["raw_bytes"] and sha(raw) == row["raw_sha256"], "raw receipt identity")
            count += 1
        report = json.loads(gzip.decompress((directory / "retention.json.gz").read_bytes()))
        collector = here / ("retain-aggregate.py" if label == "aggregate" else "retain-component.py")
        require(report["collector_sha256"] == sha(collector.read_bytes()), "measured collector unchanged")
        require(report["status"] == "retained" and report["matrix_acceptance"] is False, "retention scope")
    report = dict(scope="Historical retention checks only; original CI and its matrix remain failed",
                  status="running", matrix_acceptance=False, checked_receipt_archives=count,
                  command=[sys.executable, *sys.argv], checker_sha256=sha(Path(__file__).read_bytes()),
                  commands=[])
    env = os.environ.copy()
    env.pop("PYTHONOPTIMIZE", None)
    env["PYTHONDONTWRITEBYTECODE"] = "1"

    def run(argv, expected, message, extra_env=None):
        result = subprocess.run(list(map(str, argv)), cwd=repo, env=env | (extra_env or {}), capture_output=True, text=True)
        report["commands"].append(dict(command=list(map(str, argv)), env=extra_env or {},
            exit_code=result.returncode, stdout=result.stdout, stderr=result.stderr))
        require(result.returncode == expected and message in result.stdout + result.stderr, "check command outcome")

    try:
        common = ["--inputs", args.inputs, "--work", args.work]
        for script, extra in (("retain-component.py", ["--label", "linux-arm", "--artifact-id", 10950491263,
                "--job-id", 108717528179, "--target", "aarch64-unknown-linux-gnu"]), ("retain-aggregate.py", [])):
            run([sys.executable, here / script, *common, *extra], 1, "fresh work and receipt directories required")
            run([sys.executable, "-O", here / script, *common, *extra], 1, "unoptimized Python required")
            run([sys.executable, here / script, *common, *extra], 1, "unoptimized Python required", {"PYTHONOPTIMIZE": "2"})
        run([sys.executable, here / "retain-component.py", *common, "--label", "../escape", "--artifact-id", 10950491263,
             "--job-id", 108717528179, "--target", "aarch64-unknown-linux-gnu"], 1, "portable label required")
        require(index.read_bytes() == index_before, "guards mutated store")
        run([sys.executable, here / "intel-deadline/verify.py"], 0, '"status": "passed"')
        run(["git", "diff", "--exit-code", "a8b86b14fb4f6e20279f2c44ebf829bdc7b4b322", "--",
             str(here / "intel-deadline")], 0, "")
        run(["git", "diff", "--check"], 0, "")
        report["status"] = "passed"
    except BaseException as error:
        report.update(status="failed", error=f"{type(error).__name__}: {error}")
    with args.output.open("x") as output:
        json.dump(report, output, indent=2)
        output.write("\n")
    print(json.dumps({key: report[key] for key in ("status", "checked_receipt_archives", "matrix_acceptance")}))
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
