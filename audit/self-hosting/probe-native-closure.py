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
    parser.add_argument("--budget", type=int, default=100000000)
    parser.add_argument("--arena-nodes", type=int, default=3145728)
    parser.add_argument("--time-ms", type=int, default=300000)
    parser.add_argument("--validation-visits", type=int, default=1000000)
    parser.add_argument("--resident-nodes", type=int)
    parser.add_argument("--collection-work", type=int)
    parser.add_argument("--emit", choices=["result", "program"], default="result")
    args = parser.parse_args()
    if (args.resident_nodes is None) != (args.collection_work is None):
        parser.error("--resident-nodes and --collection-work must be supplied together")
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
        source_root = root / "source-root"
        for index, (name, module) in enumerate(sorted(data["modules"].items())):
            relative = Path(module["path"])
            assert not relative.is_absolute() and ".." not in relative.parts, relative
            source = repo / module["path"]
            content = source.read_bytes()
            assert len(content) == module["source_bytes"], module["path"]
            file = root / f"{index}.tri"
            file.write_bytes(content)
            snapshot = source_root / relative
            snapshot.parent.mkdir(parents=True, exist_ok=True)
            snapshot.write_bytes(content)
            report["sources"][name] = dict(path=module["path"], sha256=sha(file),
                                           source_bytes=len(content), copy=str(file))
            modules.append(dict(logical_path=name, file=file.name,
                                origin_name="native-compiler", origin_version="1"))
        total = sum(row["source_bytes"] for row in report["sources"].values())
        assert total == data["source_bytes"] and len(modules) == data["module_count"]
        # Bind the bytes actually sent to JOB1 back to every inventory BLAKE3
        # and declaration/import row, rather than trusting equal byte lengths.
        snapshot_check = run(["cargo", "run", "--release", "--locked", "--offline",
                              "--example", "selfhost_inventory", "--", "--root", source_root,
                              "--entry", "compiler/nox/main.tri", "--output", inventory, "--check"])
        if snapshot_check.returncode:
            report["status"] = "snapshot-inventory-rejected"
            return snapshot_check.returncode
        report["snapshot_inventory_checked"] = True
        limits = dict(source_bytes=total, modules=128, diagnostics=16, sequence_length=65536,
                      validation_visits=args.validation_visits, artifact_bytes=16777216,
                      artifact_nodes=196608, artifact_depth=4096, reductions=args.budget,
                      arena_nodes=args.arena_nodes, evaluator_frames=65536)
        manifest = dict(version=1, entry_module="native_compiler", entry_function="main",
                        modules=modules, options=dict(target=0, input_profile=1,
                        output_profile=1, optimization=0, cfg_flags=[]), limits=limits)
        package = root / "package.json"
        package.write_text(json.dumps(manifest, indent=2) + "\n")
        report.update(manifest=manifest, module_count=len(modules), source_bytes=total)
        host = ["--arena-nodes", str(args.arena_nodes), "--budget", str(args.budget),
                "--frames", "65536", "--time-ms", str(args.time_ms),
                "--validation-visits", str(args.validation_visits)]
        if args.resident_nodes is not None:
            host.extend(["--resident-nodes", str(args.resident_nodes),
                         "--collection-work", str(args.collection_work)])
        report["host_flags"] = host
        job, result = root / "job.dag", root / "result.dag"
        packed = run([binary, "pack-job", "--compiler", compiler, "--manifest", package,
                      "-o", job, *host])
        if packed.returncode:
            report["status"] = "host-rejected"
            return packed.returncode
        report.update(admission=json.loads(packed.stdout), job_sha256=sha(job),
                      job_dag_entries=int.from_bytes(job.read_bytes()[40:44], "little"))
        executed = run([binary, "run-artifact", compiler, "--input", job,
                        "--emit", args.emit, "-o", result, *host])
        if executed.returncode:
            assert not result.exists()
            report["status"] = ("guest-rejected" if "guest compilation failed:" in executed.stderr
                                else "runtime-rejected")
            return executed.returncode
        report["execution"] = json.loads(executed.stdout)
        report["result_sha256"] = sha(result)
        report["published_kind"] = args.emit
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
