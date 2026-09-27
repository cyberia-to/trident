"""Measure an exact native source closure through the installed Joy worker.

The inventory must pass selfhost_inventory --check first. No source text is
rewritten. This probe records JOB1/RES1 and preserves runtime rejection; it does
not claim usable C2, a fixed point, or a compilation proof.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import time


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--joy", type=Path, required=True)
    parser.add_argument("--compiler", type=Path, required=True)
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[2]
    binary, compiler, inventory, output = [path.resolve() for path in
        (args.joy, args.compiler, args.inventory, args.output)]
    if output.exists():
        parser.error("choose a new receipt path; existing evidence is preserved")
    output.parent.mkdir(parents=True, exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix=output.stem + "-files-", dir=output.parent))
    report = dict(schema="trident/native-closure-probe/v1", status="running",
                  scope="exact full source JOB1 and runtime boundary; no C2 usability claim",
                  artifact_directory=str(root), binary=str(binary), binary_sha256=sha(binary),
                  compiler=str(compiler), compiler_sha256=sha(compiler),
                  inventory=str(inventory), inventory_sha256=sha(inventory),
                  commands=[], sources={})

    def flush():
        output.write_text(json.dumps(report, indent=2) + "\n")

    def run(command):
        start = time.monotonic_ns()
        result = subprocess.run(list(map(str, command)), cwd=repo,
                                capture_output=True, text=True)
        report["commands"].append(dict(command=list(map(str, command)), cwd=str(repo),
                                       exit_code=result.returncode, stdout=result.stdout,
                                       stderr=result.stderr,
                                       elapsed_nanoseconds=time.monotonic_ns() - start))
        flush()
        return result

    try:
        checked = run(["cargo", "run", "--release", "--locked", "--offline",
                       "--example", "selfhost_inventory", "--", "--root", ".",
                       "--entry", "compiler/nox/main.tri", "--output", inventory, "--check"])
        if checked.returncode:
            report["status"] = "inventory-rejected"
            return checked.returncode
        data = json.loads(inventory.read_text())
        modules = []
        for index, (name, module) in enumerate(sorted(data["modules"].items())):
            source = repo / module["path"]
            content = source.read_bytes()
            assert len(content) == module["source_bytes"], module["path"]
            file = root / f"{index}.tri"
            file.write_bytes(content)
            report["sources"][name] = dict(path=module["path"], sha256=sha(file),
                                           source_bytes=len(content), copy=str(file))
            modules.append(dict(logical_path=name, file=file.name,
                                origin_name="native-compiler", origin_version="1"))
        total = sum(row["source_bytes"] for row in report["sources"].values())
        assert total == data["source_bytes"] and len(modules) == data["module_count"]
        limits = dict(source_bytes=total, modules=128, diagnostics=16, sequence_length=65536,
                      validation_visits=1000000, artifact_bytes=16777216,
                      artifact_nodes=196608, artifact_depth=4096, reductions=100000000,
                      arena_nodes=3145728, evaluator_frames=65536)
        manifest = dict(version=1, entry_module="native_compiler", entry_function="main",
                        modules=modules, options=dict(target=0, input_profile=1,
                        output_profile=1, optimization=0, cfg_flags=[]), limits=limits)
        package = root / "package.json"
        package.write_text(json.dumps(manifest, indent=2) + "\n")
        report.update(manifest=manifest, module_count=len(modules), source_bytes=total)
        host = ["--arena-nodes", "3145728", "--budget", "100000000", "--frames", "65536",
                "--time-ms", "300000"]
        job, result = root / "job.dag", root / "result.dag"
        packed = run([binary, "pack-job", "--compiler", compiler, "--manifest", package,
                      "-o", job, *host])
        if packed.returncode:
            report["status"] = "host-rejected"
            return packed.returncode
        report.update(admission=json.loads(packed.stdout), job_sha256=sha(job),
                      job_dag_entries=int.from_bytes(job.read_bytes()[40:44], "little"))
        executed = run([binary, "run-artifact", compiler, "--input", job,
                        "--emit", "result", "-o", result, *host])
        if executed.returncode:
            assert not result.exists()
            report["status"] = "runtime-rejected"
            return executed.returncode
        report["execution"] = json.loads(executed.stdout)
        report["result_sha256"] = sha(result)
        accepted = report["execution"]["execution"]["compiler_job"]["status"] == "success"
        report["status"] = "compiler-returned" if accepted else "guest-rejected"
        return 0 if accepted else 1
    except BaseException:
        report["status"] = "probe-failed"
        raise
    finally:
        report["binary_sha256_end"] = sha(binary)
        report["compiler_sha256_end"] = sha(compiler)
        if (report["binary_sha256_end"] != report["binary_sha256"] or
                report["compiler_sha256_end"] != report["compiler_sha256"]):
            report["status"] = "inputs-changed"
            flush()
            raise RuntimeError("probe inputs changed during execution")
        flush()


if __name__ == "__main__":
    raise SystemExit(main())
