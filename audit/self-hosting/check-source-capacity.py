"""Installed C1 source admission, original spans and independent runtime limits.

This checks a bounded source-capacity increment, not complete self-compilation.
Receipts and their source/JOB/RES/ART files survive failures.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile

HOST = ["--budget", "100000000", "--arena-nodes", "3145728",
        "--frames", "65536", "--time-ms", "300000"]
LIMITS = dict(source_bytes=131072, modules=128, diagnostics=16,
              sequence_length=4096, validation_visits=1000000,
              artifact_bytes=16777216, artifact_nodes=196608,
              artifact_depth=4096, reductions=100000000,
              arena_nodes=3145728, evaluator_frames=65536)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--joy", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    binary, output = args.joy.resolve(), args.output.resolve()
    repo = Path(__file__).resolve().parents[2]
    output.parent.mkdir(parents=True, exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix=output.stem + "-files-", dir=output.parent))
    report = dict(schema="trident/source-capacity/v1", status="running",
                  scope="source admission and spans; compiler-scale execution remains open",
                  artifact_directory=str(root), binary=str(binary), binary_sha256=sha(binary),
                  commands=[], observations=[])

    def flush():
        output.write_text(json.dumps(report, indent=2) + "\n")

    def run(arguments, expected=(0,)):
        command = [str(binary), *map(str, arguments)]
        result = subprocess.run(command, cwd=repo, capture_output=True, text=True)
        report["commands"].append(dict(command=command, cwd=str(repo),
                                       exit_code=result.returncode, stdout=result.stdout,
                                       stderr=result.stderr))
        flush()
        assert result.returncode in expected, report["commands"][-1]
        return json.loads(result.stdout) if result.stdout.lstrip().startswith("{") else None

    spec = importlib.util.spec_from_file_location(
        "source_capacity_decode", Path(__file__).with_name("check-generated-compiler-profile.py"))
    decoder = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(decoder)
    compiler = root / "compiler.dag"
    try:
        run(["build", repo / "compiler/nox/main.tri", "--emit", "artifact",
             "--artifact-profile", "compiler-job", "-o", compiler])
        report["compiler"] = dict(sha256=sha(compiler), particle=compiler.read_bytes()[8:40].hex(),
                                  dag_entries=int.from_bytes(compiler.read_bytes()[40:44], "little"))
        zero = root / "zero.dag"
        vectors = json.loads((repo.parent / "joy/cli/tests/compiler_vectors.json").read_text())
        zero.write_bytes(bytes.fromhex(vectors["files"]["zero"]))
        prefix = b"program sample //" + b" " * 4096 + b"\n"
        cases = [
            ("baseline", {"sample": b"program sample fn main()->Field{13}"}, 0),
            ("entry-after-comment", {"sample": prefix + b"fn main()->Field{13}"}, 0),
            ("dependency-over-4096", {
                "a": b"module a pub const X:Field=13 //" + b" " * 4096,
                "sample": b"program sample use a fn main()->Field{a.X}"}, 0),
            ("diagnostic-after-comment", {"sample": prefix + b"fn main()->Field{missing}"}, 5),
            ("invalid4097", {"sample": b"\xff" + bytes(4096)}, 1),
            ("exact65536", {"sample": b"\xff" + bytes(65535)}, 1),
            ("excess65537", {"sample": b"\xff" + bytes(65536)}, 7),
        ]
        baseline = None
        for name, sources, code in cases:
            directory = root / name
            directory.mkdir()
            modules = []
            for index, (owner, source) in enumerate(sorted(sources.items())):
                file = f"{index}.tri"
                (directory / file).write_bytes(source)
                modules.append(dict(logical_path=owner, file=file,
                                    origin_name="source-capacity", origin_version="1"))
            limits = dict(LIMITS, source_bytes=sum(map(len, sources.values())))
            manifest = dict(version=1, entry_module="sample", entry_function="main",
                            modules=modules, options=dict(target=0, input_profile=0,
                            output_profile=0, optimization=0, cfg_flags=[]), limits=limits)
            (directory / "package.json").write_text(json.dumps(manifest, indent=2) + "\n")
            job, result, program = [directory / file for file in ("job.dag", "result.dag", "program.dag")]
            run(["pack-job", "--compiler", compiler, "--manifest", directory / "package.json",
                 "-o", job, *HOST])
            protected = baseline if baseline is not None else zero.read_bytes()
            program.write_bytes(protected)
            observed = run(["run-artifact", compiler, "--input", job, "--emit", "result",
                            "-o", result, *HOST], (0, 1) if name == "exact65536" else (0,))
            row = dict(case=name, source_hex={owner: data.hex() for owner, data in sources.items()},
                       limits=limits, compiler_execution=observed)
            if observed is None:
                assert name == "exact65536"
                assert "Unavailable" in report["commands"][-1]["stderr"]
                assert not result.exists()
                run(["run-artifact", compiler, "--input", job, "--emit", "program", "-o", program,
                     "--force", *HOST], (1,))
                assert program.read_bytes() == protected
                row.update(result="runtime arena exhausted; no RES1", previous_program_preserved=True)
            elif code:
                diagnostics = observed["execution"]["compiler_job"]["diagnostics"]
                assert len(diagnostics) == 1 and diagnostics[0]["code"] == code
                if name == "diagnostic-after-comment":
                    diagnostic = diagnostics[0]
                    start, end = diagnostic["start_byte"], diagnostic["end_byte"]
                    assert start > 4096 and sources["sample"][start:end] == b"missing"
                    assert diagnostic["module_index"] == 0
                if name == "excess65537":
                    assert diagnostics[0]["start_byte"] == diagnostics[0]["end_byte"] == 0
                run(["run-artifact", compiler, "--input", job, "--emit", "program", "-o", program,
                     "--force", *HOST], (1,))
                assert program.read_bytes() == protected
                row.update(diagnostics=diagnostics, previous_program_preserved=True)
            else:
                run(["run-artifact", compiler, "--input", job, "--emit", "program", "-o", program,
                     "--force", *HOST])
                decoder.check_result(result, job, program)
                raw_output = directory / "value.dag"
                run(["run-artifact", program, "--input", zero, "-o", raw_output])
                assert decoder.decode(raw_output) == 13
                if baseline is None:
                    baseline = program.read_bytes()
                assert program.read_bytes() == baseline
                row.update(artifact_sha256=sha(program), expected=13)
            report["observations"].append(row)
            flush()
        report["status"] = "passed"
    except BaseException:
        report["status"] = "failed"
        raise
    finally:
        flush()


if __name__ == "__main__":
    main()
