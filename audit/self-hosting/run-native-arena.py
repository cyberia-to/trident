"""SH4 installed CLI acceptance for explicit larger compiler arenas."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import runpy
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--joy", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    binary = args.joy.resolve()
    repo = Path(__file__).resolve().parents[2]
    decode = runpy.run_path(str(Path(__file__).with_name("run-native-compiler.py")))["decode"]
    accepted = json.loads(Path(__file__).with_name("native-functions-cli.json").read_text())
    commands, observations = [], []
    receipt = {"schema": "trident/native-arena-cli/v1", "kind": "local-development",
               "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
               "commands": commands, "observations": observations, "complete": False}

    def save():
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(receipt, indent=2) + "\n")

    host = ["--budget", "100000000", "--frames", "65536", "--time-ms", "60000"]
    with tempfile.TemporaryDirectory(prefix="trident-sh4-") as temporary:
        root = Path(temporary)

        def run(arguments, expected=0):
            command = [str(binary), *map(str, arguments)]
            result = subprocess.run(command, capture_output=True, text=True, check=False)
            row = {"command": command, "exit_code": result.returncode,
                   "stdout": result.stdout, "stderr": result.stderr}
            commands.append(row)
            save()
            assert result.returncode == expected, row
            if expected:
                assert not result.stdout, row
                return row
            return json.loads(result.stdout) if result.stdout.startswith("{") else None

        # Build C1 once before writing any source presented to this compiler.
        compiler = root / "compiler.dag"
        run(["build", repo / "compiler/nox/main.tri", "--emit", "artifact",
             "--artifact-profile", "compiler-job", "-o", compiler])
        compiler_sha = hashlib.sha256(compiler.read_bytes()).hexdigest()
        receipt.update(compiler_sha256=compiler_sha,
                       compiler_particle=compiler.read_bytes()[8:40].hex())
        assert receipt["compiler_particle"] == accepted["compiler_particle"]
        vectors = json.loads((repo.parent / "joy/cli/tests/compiler_vectors.json").read_text())
        zero = root / "zero.dag"
        zero.write_bytes(bytes.fromhex(vectors["files"]["zero"]))
        base = {"version": 1, "entry_module": "sample", "entry_function": "main",
                "modules": [{"logical_path": "sample", "file": "source.tri",
                             "origin_name": "pilot", "origin_version": "1"}],
                "options": {"target": 0, "input_profile": 0, "output_profile": 0,
                            "optimization": 0, "cfg_flags": []},
                "limits": {"source_bytes": 8192, "modules": 128, "diagnostics": 16,
                           "sequence_length": 8192, "validation_visits": 1000000,
                           "artifact_bytes": 16777216, "artifact_nodes": 196608,
                           "artifact_depth": 4096, "reductions": 100000000,
                           "arena_nodes": 196608, "evaluator_frames": 65536}}

        def package(name, source, allowance):
            directory = root / name
            directory.mkdir()
            (directory / "source.tri").write_bytes(source)
            manifest = copy.deepcopy(base)
            manifest["limits"]["arena_nodes"] = allowance
            (directory / "package.json").write_text(json.dumps(manifest))
            job = directory / "job.dag"
            packed = run(["pack-job", "--compiler", compiler, "--manifest",
                          directory / "package.json", "-o", job, *host,
                          "--arena-nodes", "786432"])
            return directory, job, packed

        def execute(job, output, allowance, emit="program", expected=0):
            return run(["run-artifact", compiler, "--input", job, "--emit", emit,
                        "-o", output, "--force", *host, "--arena-nodes", allowance], expected)

        def compiled_program(directory, job, allowance, expected):
            program = directory / "program.dag"
            report = execute(job, program, allowance)
            assert report["execution"]["compiler_job"]["status"] == "success"
            output = directory / "output.dag"
            executed = run(["run-artifact", program, "--input", zero, "-o", output,
                            "--force", *host])
            assert decode(output) == expected
            assert report["published_particle"] == executed["execution"]["program_particle"]
            return program.read_bytes(), report, executed

        def source(body):
            return f"program sample fn main() -> Field {{ {body} }}".encode()

        arithmetic = source("2+3*4")
        directory, job, _ = package("physical-parity", arithmetic, 196608)
        original_job = job.read_bytes()
        prior_program, small, _ = compiled_program(directory, job, 196608, 14)
        large_program, large, _ = compiled_program(directory, job, 786432, 14)
        assert prior_program == large_program
        for key in ["program_particle", "input_particle", "output_particle",
                    "charged_reductions", "allocated_nodes", "peak_frames", "compiler_job"]:
            assert small["execution"][key] == large["execution"][key], key
        assert large["execution"]["arena_reserved_bytes"] > small["execution"]["arena_reserved_bytes"]
        # Repack the same manifest in the default physical arena.
        run(["pack-job", "--compiler", compiler, "--manifest", directory / "package.json",
             "-o", job, "--force", *host])
        assert job.read_bytes() == original_job
        old = next(o for o in accepted["observations"] if o.get("case") == "precedence")
        assert small["published_particle"] == old["compiler_execution"]["published_particle"]
        observations.append({"case": "same-job-physical-parity", "small": small, "large": large,
                             "exact_job_and_program_bytes_equal": True})
        directory, job, _ = package("changed-job-allowance", arithmetic, 786432)
        changed_program, changed, _ = compiled_program(directory, job, 786432, 14)
        assert changed_program == prior_program
        assert changed["execution"]["input_particle"] != small["execution"]["input_particle"]
        assert changed["execution"]["output_particle"] != small["execution"]["output_particle"]
        observations.append({"case": "changed-job-allowance", "compiler_execution": changed,
                             "job_and_result_identities_change": True, "program_bytes_equal": True})

        def boundaries(name, content, observed_nodes, artifact):
            directory, job, _ = package(name + "-calibration", content, observed_nodes + 32)
            calibrated = execute(job, directory / "program.dag", 786432)
            exact = calibrated["execution"]["allocated_nodes"]
            for delta in [0, -1]:
                directory, job, _ = package(f"{name}-quota-{delta}", content, exact + delta)
                protected = directory / "program.dag"
                protected.write_bytes(artifact)
                result = execute(job, protected, 786432, expected=0 if delta == 0 else 1)
                assert protected.read_bytes() == artifact
                if delta == 0:
                    assert result["execution"]["allocated_nodes"] == exact
                else:
                    assert "Unavailable" in result["stderr"]
                observations.append({"case": name + "-quota", "host_allowance": 786432,
                                     "job_allowance": exact + delta,
                                     "boundary": "exact" if delta == 0 else "one-below",
                                     "compiler_execution": result, "previous_program_preserved": True})

        boundaries("small-job", arithmetic, small["execution"]["allocated_nodes"], prior_program)
        nested = "f(" * 64 + "1" + ")" * 64
        cases = [("assignments31", source("let mut x=0 " + "x=x+1 " * 31 + "x"), 31),
                 ("calls64", ("program sample fn f(x:Field)->Field{x} fn main()->Field{"
                               + nested + "}").encode(), 1),
                 ("source4096-invalid", b"\xff" + bytes(4095), None),
                 ("source4096-valid", arithmetic + b" " * (4096 - len(arithmetic)), "runtime-unavailable")]
        for name, content, expected in cases:
            directory, job, _ = package(name + "-default", content, 196608)
            protected = directory / "program.dag"
            protected.write_bytes(prior_program)
            failed = execute(job, protected, 196608, expected=1)
            assert "Unavailable" in failed["stderr"]
            assert protected.read_bytes() == prior_program
            directory, job, packed = package(name + "-large", content, 786432)
            if expected == "runtime-unavailable":
                protected = directory / "program.dag"
                protected.write_bytes(prior_program)
                failure = execute(job, protected, 786432, expected=1)
                assert "Unavailable" in failure["stderr"]
                assert protected.read_bytes() == prior_program
                observations.append({"case": name, "source_hex": content.hex(),
                                     "result": "runtime failure at both allowances; full source scale remains open",
                                     "large_failure": failure, "default_failure": failed,
                                     "previous_program_preserved": True})
            elif expected is not None:
                artifact, report, executed = compiled_program(directory, job, 786432, expected)
                assert report["execution"]["allocated_nodes"] > 196608
                if name == "assignments31":
                    boundaries(name, content, report["execution"]["allocated_nodes"], artifact)
                observations.append({"case": name, "source_hex": content.hex(), "expected": expected,
                                     "packing": packed, "compiler_execution": report,
                                     "program_execution": executed, "default_failure": failed,
                                     "default_preserves_previous_program": True})
            else:
                report = execute(job, directory / "result.dag", 786432, emit="result")
                diagnostics = report["execution"]["compiler_job"]
                assert diagnostics["status"] == "compile_error"
                assert len(diagnostics["diagnostics"]) == 1
                assert diagnostics["diagnostics"][0]["code"] == 1
                protected = directory / "program.dag"
                protected.write_bytes(prior_program)
                failure = execute(job, protected, 786432, expected=1)
                assert protected.read_bytes() == prior_program
                observations.append({"case": name, "source_hex": content.hex(),
                                     "compiler_execution": report, "program_refusal": failure,
                                     "default_failure": failed, "previous_program_preserved": True})
            save()
            print(name + (" resource failure recorded" if expected == "runtime-unavailable"
                          else " accepted"), flush=True)
        assert hashlib.sha256(compiler.read_bytes()).hexdigest() == compiler_sha
    receipt["complete"] = True
    receipt["scope"] = "Measured SH4 arena increment; full source closure, language, self-build and proof gates remain open"
    save()
    print(json.dumps({"commands": len(commands), "observations": len(observations),
                      "receipt": str(args.output)}))


if __name__ == "__main__":
    main()
