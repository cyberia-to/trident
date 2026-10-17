"""Bind original 36 native phases, three restored stores, local replay and CI verdict."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import runpy
import stat
import subprocess
import sys
import time
import zipfile

RUN = 36382085561
HEAD = "57491633fbccb58ae44dca2da438ee31430be1bc"
EXPECTED = "d1dca2687520edbded3bc6e3088fe7fb1d72a43318a3cc15b87e1aea3ca35dde"
COMPILER = "76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8"
ORIGIN = dict(run_id=str(RUN), run_attempt="1", head_sha=HEAD)
ROLES = ("producer", "c2", "c3")
MAX_BYTES, MAX_FILES = 4 << 30, 50_000
MAX_JSON = 32 << 20


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def load(path):
    require(path.is_file() and not path.is_symlink() and path.stat().st_size <= MAX_JSON, "bounded ordinary JSON")
    return json.loads(path.read_bytes())


def identity(path):
    return dict(path=str(path), bytes=path.stat().st_size, sha256=sha(path))


def archive(inputs, artifact_id, name):
    require(type(artifact_id) is int and artifact_id > 0, "positive integer artifact ID")
    api = inputs / f"artifact-{artifact_id}.json"
    path = inputs / f"artifact-{artifact_id}.zip"
    meta = load(api)
    require(type(meta["id"]) is int and meta["id"] == artifact_id and meta["name"] == name, "artifact id/name")
    require(meta["workflow_run"]["id"] == RUN and meta["workflow_run"]["head_sha"] == HEAD, "artifact origin")
    require(path.is_file() and not path.is_symlink() and path.stat().st_size <= MAX_BYTES, "bounded ordinary ZIP")
    require(meta["digest"] == "sha256:" + sha(path) and meta["size_in_bytes"] == path.stat().st_size, "original ZIP/API identity")
    return path, dict(metadata=identity(api), zip=identity(path))


def store_binding(store, entry, name, provenance):
    digest = entry["manifest_sha256"]
    require(isinstance(digest, str) and re.fullmatch(r"[0-9a-f]{64}", digest), "store manifest digest")
    path = store / "manifests" / (digest + ".json")
    manifest = load(path)
    require(sha(path) == digest, "pinned store manifest bytes")
    require(manifest["schema"] == 1 and manifest["name"] == name and manifest["id"] == entry["id"] and
            manifest["run_id"] == RUN and manifest["head_sha"] == HEAD, "store manifest origin")
    require(manifest["zip"] == {key: provenance["zip"][key] for key in ("bytes", "sha256")}, "store original ZIP binding")
    meta_sha = manifest["metadata_sha256"]
    require(meta_sha == provenance["metadata"]["sha256"], "store original API binding")
    retained_meta = store / "metadata" / (meta_sha + ".json")
    load(retained_meta)
    require(sha(retained_meta) == meta_sha, "retained original API bytes")
    return dict(manifest=identity(path), metadata=identity(retained_meta))


def exact_tree(path, root):
    require(root.is_dir() and not root.is_symlink(), "ordinary restored phase directory")
    restored_names = set()
    for count, item in enumerate(root.rglob("*"), 1):
        require(count <= MAX_FILES * 2, "bounded restored tree entries")
        mode = item.lstat().st_mode
        require(stat.S_ISREG(mode) or stat.S_ISDIR(mode), "ordinary restored tree entry")
        if stat.S_ISREG(mode):
            restored_names.add(item.relative_to(root).as_posix())
    with zipfile.ZipFile(path) as source:
        require(len(source.infolist()) <= MAX_FILES * 2, "bounded ZIP entries")
        files = [item for item in source.infolist() if not item.is_dir()]
        names = [item.filename for item in files]
        require(len(files) <= MAX_FILES and len(set(names)) == len(names), "bounded unique ZIP files")
        require(restored_names == set(names), "exact restored file set")
        total = 0
        for item in files:
            relative = PurePosixPath(item.filename)
            require(not relative.is_absolute() and ".." not in relative.parts and
                    relative.as_posix() == item.filename and "\\" not in item.filename and
                    not stat.S_ISLNK(item.external_attr >> 16), "ordinary ZIP member path")
            total += item.file_size
            require(total <= MAX_BYTES, "expanded ZIP byte bound")
            with source.open(item) as original, (root / relative).open("rb") as restored:
                size = 0
                while block := original.read(1 << 20):
                    size += len(block)
                    require(size <= item.file_size and restored.read(len(block)) == block, "exact restored ZIP bytes")
                require(size == item.file_size and restored.read(1) == b"", "exact restored ZIP length")
    return dict(files=len(files), raw_bytes=total)


def job_identity(path, name):
    job = load(path)
    require(type(job["id"]) is int and job["id"] > 0 and path.name == f"job-{job['id']}-direct.json", "direct job file identity")
    require(job["run_id"] == RUN and job["head_sha"] == HEAD and job["run_attempt"] == 1, "direct job origin")
    require(job["status"] == "completed" and job["conclusion"] == "success" and job["name"] == name, "original successful job")
    return job


def distinct_job(job, selected):
    require(job["id"] not in selected, "distinct native job IDs required")
    selected.add(job["id"])


def receipt_map(rows, expected_count):
    require(len(rows) == expected_count, "aggregate phase receipt count")
    require(all(Path(row["path"].replace("\\", "/")).name == "receipt.json" for row in rows), "aggregate receipt role")
    result = {Path(row["path"].replace("\\", "/")).parent.name: row["sha256"] for row in rows}
    require(len(result) == expected_count, "aggregate phase receipt uniqueness")
    return result


def s1_binding(step, reference):
    keys = ("compiler_sha256", "result_sha256", "inventory_sha256", "job_sha256")
    require(all(step[key] == reference[key] for key in keys), "frozen S1 compiler/source/JOB identities")
    execution = dict(step["execution"]["execution"])
    require(type(execution["elapsed_micros"]) is int and execution["elapsed_micros"] >= 0, "measured S1 elapsed time")
    del execution["elapsed_micros"]
    require(execution == reference["execution_without_elapsed"], "frozen S1 execution fields")
    return {key: step[key] for key in keys}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ("restored", "stores", "indices", "inputs", "runner-directory", "expected", "aggregate-job", "final-run", "output"):
        parser.add_argument("--" + key, type=Path, required=True)
    parser.add_argument("--indices-sha256", required=True)
    parser.add_argument("--aggregate-id", type=int, required=True)
    args = parser.parse_args()
    require(os.environ.get("GITHUB_ACTIONS") != "true", "local replay must not impersonate CI")
    output = args.output.resolve()
    for path in (args.restored, args.stores, args.indices, args.inputs, args.runner_directory, args.expected, args.aggregate_job, args.final_run):
        path = path.resolve()
        require(not output.is_relative_to(path) and not path.is_relative_to(output), "input/output paths must be separate")
    require(not output.exists(), "fresh output directory required")
    require(sha(args.expected) == EXPECTED, "frozen expected source and implementation contract")
    expected = load(args.expected)
    require(expected["run_id"] == RUN and expected["head_sha"] == HEAD and expected["run_attempt"] == 1, "expected origin")
    require(expected["pins"]["trident"] == HEAD, "expected Trident source pin")
    local_steps = expected["local_reference_steps"]
    require(type(local_steps) is list and len(local_steps) == 2 and all(type(row) is dict for row in local_steps), "two frozen S1 reference steps")
    for name, digest in expected["implementation"].items():
        require(sha(args.runner_directory / name) == digest, "frozen phase implementation")
    require(sha(args.indices) == args.indices_sha256, "pinned final three-store indices")
    indices = load(args.indices)
    require(set(indices) == set(ROLES), "three distinct phase store roles")
    sys.dont_write_bytecode = True
    sys.path.insert(0, str(args.runner_directory))
    module = runpy.run_path(str(args.runner_directory / "bootstrap-phases.py"))
    checker, runner = module["C"], module["B"]
    expected_names, entries = {}, {}
    for role in ROLES:
        index_path = args.stores / role / "index.json"
        require(sha(index_path) == indices[role], "exact final phase store index")
        index = load(index_path)
        require(index["schema"] == 1 and index["run_id"] == RUN and index["head_sha"] == HEAD, "phase store origin/schema")
        names = {f"bootstrap-phase-{target}-repeat-{number}-{role}-attempt-1": (target, number, role)
                 for target in runner.TARGETS for number in (1, 2)}
        require(set(index["artifacts"]) == set(names), "exact twelve artifact names per phase store")
        expected_names.update(names)
        entries.update(index["artifacts"])
    require(len({row["id"] for row in entries.values()}) == 36, "thirty-six distinct original artifacts")
    require(all(type(row["id"]) is int and row["id"] > 0 for row in entries.values()), "positive integer phase artifact IDs")
    require({p.name for p in args.restored.iterdir()} == set(expected_names), "exact thirty-six restored phase directories")
    jobs = {}
    for path in args.inputs.glob("job-*-direct.json"):
        job = load(path)
        if job["name"].startswith("Bootstrap "):
            require(job["name"] not in jobs, "duplicate original phase job name")
            jobs[job["name"]] = path
    output.mkdir(parents=True, exist_ok=False)
    report = dict(schema="trident/retained-sh6-phase-matrix-validation/v1", status="running", started_ns=time.time_ns(),
                  command=[sys.executable, *sys.argv], validator_sha256=sha(Path(__file__)), expected=identity(args.expected),
                  indices=identity(args.indices), expected_origin=ORIGIN, phases=[],
                  scope="Original 36-phase native CI aggregate and independent local replay of the same retained bytes")
    try:
        reports, selected_jobs = [], set()
        for name in sorted(expected_names):
            root = args.restored / name
            target, number, role = expected_names[name]
            path, provenance = archive(args.inputs, entries[name]["id"], name)
            retained_store = store_binding(args.stores / role, entries[name], name, provenance)
            tree = exact_tree(path, root)
            receipt = load(root / "receipt.json")
            require(receipt["target"] == target and receipt["repeat"] == number, "artifact target/repetition binding")
            require(receipt["ci_origin"] == ORIGIN and receipt["pins"] == expected["pins"], "exact phase origin/pins")
            require(receipt["rust_version"] == expected["rust_version"] and receipt["implementation"] == expected["implementation"], "phase Rust/implementation")
            job_name = f"Bootstrap {target} repeat {number} / " + ("Self-build C2 and C3" if role == "producer" else f"Corpus C{role[1]}")
            require(job_name in jobs, "original native phase job required")
            job = job_identity(jobs[job_name], job_name)
            distinct_job(job, selected_jobs)
            job_log = args.inputs / f"job-{job['id']}-api.log"
            require(job_log.is_file(), "original phase job log required")
            bindings = []
            if role == "producer":
                require(receipt["phase"] == "producer", "producer artifact phase role")
                checker.producer(root, receipt)
                row = receipt["repetitions"][0]
                for generation, reference in zip((2, 3), local_steps):
                    step_path = runner.retained(root, row[f"step{generation}"])
                    step = runner.load(step_path)
                    binding = s1_binding(step, reference)
                    bindings.append(dict(generation=generation, receipt=identity(step_path), identities=binding,
                                         execution_without_elapsed=reference["execution_without_elapsed"]))
                builds = [row for row in receipt["commands"] if row["command"][:2] == ["cargo", "build"]]
                require(len(builds) == 2, "exact two native tool builds")
                for build in builds:
                    stderr = root / build["stderr"]
                    require(stderr.stat().st_size <= MAX_JSON, "bounded native build log")
                    require(not re.search(rb"^warning(?:\[[^\]\r\n]+\])?:", stderr.read_bytes(), re.M), "native build warnings")
            else:
                require(receipt["phase"] == "corpus" and receipt["generation"] == int(role[1]), "corpus artifact generation binding")
                checker.corpus(root, receipt)
            reports.append((root, receipt))
            report["phases"].append(dict(name=name, **tree, **provenance, receipt=identity(root / "receipt.json"),
                                         store=retained_store, job=identity(jobs[job_name]), job_log=identity(job_log), local_s1_steps=bindings))
        command = [sys.executable, "-B", "-W", "error", str(args.runner_directory / "bootstrap-phases.py"), "--phase", "matrix",
                   "--matrix", str(args.restored), "--pins-json", json.dumps(expected["pins"]), "--rust-version", expected["rust_version"],
                   "--output", str(output / "local-replay")]
        env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1")
        with (output / "replay.stdout").open("xb") as stdout, (output / "replay.stderr").open("xb") as stderr:
            result = subprocess.run(command, env=env, stdout=stdout, stderr=stderr)
        report["local_command"] = dict(argv=command, cwd=str(Path.cwd()), exit_code=result.returncode,
                                       stdout=identity(output / "replay.stdout"), stderr=identity(output / "replay.stderr"))
        require(result.returncode == 0, "frozen phase matrix CLI failed")
        local = load(output / "local-replay/receipt.json")
        require(local["status"] == "passed" and local["ci_origin"] is None and local["compiler_sha256"] == COMPILER, "actual local matrix replay")
        require(checker.compare_matrix(reports, ORIGIN) == COMPILER, "explicit original-origin phase comparison")
        require(args.aggregate_id not in {row["id"] for row in entries.values()}, "separate aggregate artifact ID")
        path, provenance = archive(args.inputs, args.aggregate_id, "bootstrap-phase-matrix-comparison-attempt-1")
        with zipfile.ZipFile(path) as source:
            require(set(source.namelist()) == {"receipt.json", "files.json"} and len(source.namelist()) == 2, "original aggregate member set")
            require(all(item.file_size <= 16 << 20 for item in source.infolist()), "bounded aggregate members")
            raw, files = source.read("receipt.json"), source.read("files.json")
        actual = json.loads(raw)
        require(actual["schema"] == checker.SCHEMA and actual["phase"] == "matrix" and actual["status"] == "passed", "original CI matrix passed")
        require(actual["ci_origin"] == ORIGIN and actual["implementation"] == expected["implementation"] and actual["compiler_sha256"] == COMPILER, "original matrix origin/implementation/compiler")
        require(actual["files"] == dict(path="files.json", bytes=len(files), sha256=hashlib.sha256(files).hexdigest()) and json.loads(files) == {}, "original aggregate manifest")
        restored = {root.name: sha(root / "receipt.json") for root, _ in reports}
        require(receipt_map(actual["phase_reports"], 36) == restored == receipt_map(local["phase_reports"], 36), "original/local/restored phase receipts match")
        require(len(selected_jobs) == 36, "thirty-six original native job IDs")
        distinct_job(job_identity(args.aggregate_job, "Compare twelve native self-builds and twenty-four compiler corpora"), selected_jobs)
        run = load(args.final_run)
        require(run["id"] == RUN and run["head_sha"] == HEAD and run["run_attempt"] == 1 and run["status"] == "completed" and run["conclusion"] == "success", "original final run verdict")
        (output / "original-aggregate.json").write_bytes(raw)
        (output / "original-aggregate-files.json").write_bytes(files)
        report["original_aggregate"] = dict(**provenance, receipt=identity(output / "original-aggregate.json"),
                                             manifest=identity(output / "original-aggregate-files.json"), job=identity(args.aggregate_job), run=identity(args.final_run))
        report.update(status="passed", compiler_sha256=COMPILER, native_repetitions=12, native_corpus_jobs=24)
    except BaseException as error:
        report.update(status="failed", error=f"{type(error).__name__}: {error}")
    finally:
        report["ended_ns"] = time.time_ns()
        (output / "verification.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({key: report[key] for key in ("status", "scope")}))
    if report["status"] != "passed":
        print(report["error"], file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
