"""Bind twelve restored native trees, frozen local replay and original CI aggregate."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import runpy
import subprocess
import sys
import time
import zipfile

RUN = 36359020560
HEAD = "c17bd0371c11746f46e20222c48cae2ab08be79d"
RUNNER = "d5522330b73ad5454e54406747e010579baa17e1f148b8fb2354542b48ff4e35"
COMPILER = "76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8"
ORIGIN = dict(run_id=str(RUN), run_attempt="1", head_sha=HEAD)


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(path):
    h = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def load(path):
    return json.loads(Path(path).read_bytes())


def identity(path):
    return dict(path=str(path), bytes=path.stat().st_size, sha256=sha(path))


def check_archive(inputs, artifact_id, name):
    api = inputs / f"artifact-{artifact_id}.json"
    archive = inputs / f"artifact-{artifact_id}.zip"
    meta = load(api)
    require(meta["id"] == artifact_id and meta["name"] == name, "artifact name/id")
    require(meta["workflow_run"]["id"] == RUN and meta["workflow_run"]["head_sha"] == HEAD, "artifact run/head")
    require(meta["digest"] == "sha256:" + sha(archive) and meta["size_in_bytes"] == archive.stat().st_size, "API ZIP digest/size")
    return archive, dict(metadata=identity(api), zip=identity(archive))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ("restored", "store", "inputs", "runner", "expected", "aggregate-job", "final-run", "output"):
        parser.add_argument("--" + key, type=Path, required=True)
    parser.add_argument("--index-sha256", required=True)
    parser.add_argument("--aggregate-id", type=int, required=True)
    args = parser.parse_args()
    require(os.environ.get("GITHUB_ACTIONS") != "true", "local replay must not impersonate CI")
    require(sha(args.runner) == RUNNER, "unchanged frozen runner")
    require(sha(args.store / "index.json") == args.index_sha256, "pinned store index")
    index = load(args.store / "index.json")
    require(index["run_id"] == RUN and index["head_sha"] == HEAD, "store origin")
    require(len(index["artifacts"]) == 12, "all twelve platform archives required")
    require(sha(args.expected) == "18ffd832a2ada0330e00f7d9effa188fd3056954e42f09ed32b6baf2609dc790", "frozen expected-input contract")
    expected = load(args.expected)
    require(expected["run_id"] == RUN and expected["head_sha"] == HEAD and expected["run_attempt"] == 1, "expected origin")
    require(expected["runner_sha256"] == RUNNER, "expected runner")
    sys.dont_write_bytecode = True
    runner = runpy.run_path(str(args.runner))
    pairs = {f"bootstrap-{target}-repeat-{n}-attempt-1": (target, n) for target in runner["TARGETS"] for n in (1, 2)}
    names = set(pairs)
    local_steps = expected["local_reference_steps"]
    require(type(local_steps) is list and len(local_steps) == 2 and
            all(type(row) is dict for row in local_steps), "exactly two local reference steps required")
    require(set(index["artifacts"]) == names, "exact target/repetition artifact names")
    require({p.name for p in args.restored.iterdir()} == names, "exact restored platform directories")
    args.output.mkdir(parents=True, exist_ok=False)
    report = dict(schema="trident/retained-sh6-matrix-validation/v1", status="running", started_ns=time.time_ns(),
                  command=[sys.executable, *sys.argv], validator_sha256=sha(Path(__file__)),
                  runner=identity(args.runner), index=identity(args.store / "index.json"), expected=identity(args.expected),
                  expected_origin=ORIGIN, scope="Original CI matrix and independent local replay of the same twelve retained producers", platforms=[])
    try:
        reports = []
        for name in sorted(names):
            root = args.restored / name
            archive, provenance = check_archive(args.inputs, index["artifacts"][name]["id"], name)
            total = 0
            with zipfile.ZipFile(archive) as source:
                members = [m for m in source.infolist() if not m.is_dir()]
                member_names = [m.filename for m in members]
                require(len(set(member_names)) == len(member_names), "duplicate ZIP member")
                require({p.relative_to(root).as_posix() for p in root.rglob('*') if p.is_file()} == set(member_names), "exact restored file set")
                for member in members:
                    require(member.file_size <= 16 << 20, "member file bound")
                    data = source.read(member)
                    require((root / member.filename).read_bytes() == data, "restored bytes differ from original ZIP")
                    total += len(data)
            receipt = load(root / "receipt.json")
            require(receipt["ci_origin"] == ORIGIN and receipt["pins"] == expected["pins"], "exact origin/pins")
            require(receipt["rust_version"] == expected["rust_version"], "native Rust version")
            require(receipt["runner_sha256"] == RUNNER, "producer runner identity")
            target, number = pairs[name]
            require(receipt["target"] == target and receipt["selected_repetitions"] == [number], "artifact target/repetition binding")
            require(len(receipt["repetitions"]) == 1, "single selected producer")
            row = receipt["repetitions"][0]
            source_bindings = []
            for generation, reference in zip((2, 3), local_steps):
                step_path = runner["retained"](root, row[f"step{generation}"])
                step = runner["load"](step_path)
                keys = ("compiler_sha256", "result_sha256", "inventory_sha256", "job_sha256")
                for key in keys:
                    require(step[key] == reference[key], f"frozen S1 step{generation} {key}")
                source_bindings.append(dict(generation=generation, receipt=identity(step_path),
                                            identities={key: step[key] for key in keys}))
            reports.append((root, receipt))
            report["platforms"].append(dict(name=name, files=len(members), raw_bytes=total, local_s1_steps=source_bindings,
                                            receipt=identity(root / "receipt.json"), **provenance))
        command = [sys.executable, "-W", "error", str(args.runner), "--matrix", str(args.restored), "--output", str(args.output / "local-replay")]
        env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1")
        started = time.time_ns()
        with (args.output / "replay.stdout").open("xb") as out, (args.output / "replay.stderr").open("xb") as err:
            result = subprocess.run(command, env=env, stdout=out, stderr=err)
        report["local_command"] = dict(argv=command, cwd=str(Path.cwd()), exit_code=result.returncode,
            started_ns=started, ended_ns=time.time_ns(), stdout=identity(args.output / "replay.stdout"), stderr=identity(args.output / "replay.stderr"))
        require(result.returncode == 0, "unchanged matrix CLI failed")
        local = load(args.output / "local-replay/receipt.json")
        require(local["status"] == "passed" and local["ci_origin"] is None and local["compiler_sha256"] == COMPILER, "actual local replay")
        started = time.time_ns()
        digest = runner["compare_matrix"](reports, ORIGIN)
        require(digest == COMPILER, "explicit origin matrix comparison")
        report["explicit_origin_comparison"] = dict(operation="unchanged compare_matrix(reports, expected_origin)", started_ns=started, ended_ns=time.time_ns(), compiler_sha256=digest)
        aggregate_name = "bootstrap-matrix-comparison-attempt-1"
        archive, provenance = check_archive(args.inputs, args.aggregate_id, aggregate_name)
        with zipfile.ZipFile(archive) as source:
            require(source.namelist() == ["receipt.json"], "aggregate raw member set")
            raw = source.read("receipt.json")
        actual = json.loads(raw)
        require(actual["schema"] == runner["SCHEMA"] and actual["status"] == "passed" and actual["ci_origin"] == ORIGIN and actual["compiler_sha256"] == COMPILER, "original CI matrix result")
        original_rows = actual["platform_reports"]
        require(len(original_rows) == 12, "aggregate twelve producers")
        original = {Path(row["path"]).parent.name: row["sha256"] for row in original_rows}
        restored = {root.name: sha(root / "receipt.json") for root, _ in reports}
        require(len(original) == 12 and original == restored, "all original aggregate producer hashes")
        require({Path(row["path"]).parent.name: row["sha256"] for row in local["platform_reports"]} == restored, "local producer hashes")
        job = load(args.aggregate_job)
        require(job["run_id"] == RUN and job["head_sha"] == HEAD and job["run_attempt"] == 1 and job["status"] == "completed" and job["conclusion"] == "success", "original aggregate job API")
        require(job["name"] == "Compare all twelve native platform/repetition outputs", "aggregate job name")
        run = load(args.final_run)
        require(run["id"] == RUN and run["head_sha"] == HEAD and run["run_attempt"] == 1 and run["status"] == "completed" and run["conclusion"] == "success", "original final run API")
        (args.output / "original-aggregate.json").write_bytes(raw)
        report["original_aggregate"] = dict(**provenance, receipt=identity(args.output / "original-aggregate.json"), job=identity(args.aggregate_job), run=identity(args.final_run), all_twelve_producer_hashes_match=True)
        report.update(status="passed", compiler_sha256=COMPILER, native_repetitions=12)
    except BaseException as error:
        report.update(status="failed", error=f"{type(error).__name__}: {error}")
    finally:
        report["ended_ns"] = time.time_ns()
        (args.output / "verification.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({key: report[key] for key in ("status", "scope")}))
    if report["status"] != "passed":
        print(report["error"], file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
