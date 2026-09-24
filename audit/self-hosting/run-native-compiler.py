"""SH2/SH3 installed CLI acceptance: fixed guest, arithmetic, locals and control."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

P = 18446744069414584321


def decode(path):
    """Small independent reader for the already admitted canonical output DAG."""
    data = path.read_bytes()
    assert data[:8] == b"NOXDAG01"
    root, count, cursor, nodes = data[8:40], int.from_bytes(data[40:44], "little"), 44, {}
    for _ in range(count):
        particle = data[cursor:cursor + 32]
        width = data[cursor + 32]
        cursor += 33
        payload = data[cursor:cursor + width]
        cursor += width
        assert particle not in nodes
        if width == 8:
            value = int.from_bytes(payload, "little")
            assert value < P
        else:
            assert width == 64
            value = (nodes[payload[:32]], nodes[payload[32:]])
        nodes[particle] = value
    assert cursor == len(data) and particle == root
    return nodes[root]


def record(tag, *fields):
    body = 0
    for value in reversed(fields):
        body = (value, body)
    return tag, body


def collection_visits(length, packed=False):
    leaves = (length + 3) // 4 if packed else length
    height = (max(leaves, 1) - 1).bit_length()

    def tree(used, capacity):
        if used == 0 or capacity == 1:
            return 1
        half = capacity // 2
        return 1 + tree(min(used, half), half) + tree(max(used - half, 0), half)

    return tree(leaves, 1 << height) + (leaves * (height + 1) if packed else 0)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--joy", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    binary = args.joy.resolve()
    repo = Path(__file__).resolve().parents[2]
    commands, observations = [], []
    accepted = json.loads((repo / "audit/self-hosting/native-locals-cli.json").read_text())
    prior_cases = {item["case"]: item for item in accepted["observations"] if "case" in item and "expected" in item}
    host = ["--budget", "100000000", "--frames", "65536"]
    with tempfile.TemporaryDirectory(prefix="trident-sh2-") as temporary:
        root = Path(temporary)

        def run(arguments, expected=0):
            command = [str(binary), *map(str, arguments)]
            result = subprocess.run(command, capture_output=True, text=True, check=False)
            row = {"command": command, "exit_code": result.returncode,
                   "stdout": result.stdout, "stderr": result.stderr}
            commands.append(row)
            assert result.returncode == expected, row
            return json.loads(result.stdout) if result.stdout.startswith("{") else None

        # The only source build in this runner. All case files are created later.
        compiler = root / "compiler.dag"
        run(["build", repo / "compiler/nox/main.tri", "--emit", "artifact",
             "--artifact-profile", "compiler-job", "-o", compiler])
        compiler_sha = hashlib.sha256(compiler.read_bytes()).hexdigest()
        compiler_particle = compiler.read_bytes()[8:40].hex()
        vectors = json.loads((repo.parent / "joy/cli/tests/compiler_vectors.json").read_text())
        zero = root / "zero.dag"
        zero.write_bytes(bytes.fromhex(vectors["files"]["zero"]))
        base = {"version": 1, "entry_module": "sample", "entry_function": "main",
                "modules": [{"logical_path": "sample", "file": "source.tri", "origin_name": "pilot", "origin_version": "1"}],
                "options": {"target": 0, "input_profile": 0, "output_profile": 0, "optimization": 0, "cfg_flags": []},
                "limits": {"source_bytes": 8192, "modules": 128, "diagnostics": 16, "sequence_length": 8192,
                           "validation_visits": 1000000, "artifact_bytes": 16777216, "artifact_nodes": 196608,
                           "artifact_depth": 4096, "reductions": 100000000, "arena_nodes": 196608, "evaluator_frames": 65536}}

        def package(name, source, limits=None):
            directory = root / name
            directory.mkdir()
            (directory / "source.tri").write_bytes(source)
            manifest = copy.deepcopy(base)
            if limits:
                manifest["limits"].update(limits)
            (directory / "package.json").write_text(json.dumps(manifest))
            job = directory / "job.dag"
            run(["pack-job", "--compiler", compiler, "--manifest", directory / "package.json", "-o", job, *host])
            return directory, job

        def execute(job, output, emit="program", expected=0, force=False):
            return run(["run-artifact", compiler, "--input", job, "--emit", emit, "-o", output,
                        *host, *(["--force"] if force else [])], expected)

        def source(expression):
            return f"program sample fn main() -> Field {{ {expression} }}".encode()

        precedence = (5, ((1, 2), (7, ((1, 3), (1, 4)))))
        parentheses = (7, ((5, ((1, 2), (1, 3))), (1, 4)))
        cases = [("precedence", source("2+3*4"), precedence, 14),
                 ("parentheses", source("(2+3)*4"), parentheses, 20),
                 ("modulus", source("00018446744069414584321"), (1, 0), 0),
                 ("maximum", source("18446744073709551615"), (1, 4294967294), 4294967294),
                 ("unicode-comment", "// ж😀\r comment\nprogram\tsample\r\nfn main() -> Field { 2+3*4 }".encode(), precedence, 14),
                 ("stack64", source("(" * 64 + "1" + ")" * 64), (1, 1), 1)]
        cases.extend([
            ("typed-local", source("let x: Field=7 x"), None, 7),
            ("dependent-local", source("let a=2+3 let b=a*4 b+a"), None, 25),
            ("mutable-snapshot", source("let mut x=7 let y=x x=9 y*100+x"), None, 709),
            ("shadow", source("let x=7 let x=x+1 x"), None, 8),
            ("long-identifiers", source("let common_prefix_a=3 let common_prefix_b=7 common_prefix_a*10+common_prefix_b"), None, 37),
            ("literal-tail", source("let x=7 9"), None, 9),
            ("parenthesized-tail", source("let x=7 (x+1)"), None, 8),
            ("body-chunks", source("let mut x=0 " + "x=x+1 " * 9 + "x"), None, 9),
        ])
        cases.extend([
            ("bool-precedence", source("let b:Bool=1+2*3==7 if b {11} else {22}"), None, 11),
            ("bool-mutation", source("let mut b=true b=false if b {11} else {22}"), None, 22),
            ("field-zero", source("if 0 {11} else {22}"), None, 11),
            ("field-nonzero", source("if 2 {11} else {22}"), None, 22),
            ("terminal-nested", source("if true {if false {1} else {2}} else {3}"), None, 2),
            ("intermediate-tail", source("if true {if true {false} else {false}} 7"), None, 7),
            ("early-return", source("let mut x=1 if true {return x+6} else {x=9} x"), None, 7),
            ("else-if", source("let mut x=0 if false{x=1}else if false{x=2}else{x=3} x=x+4 x"), None, 7),
            ("branch-scope", source("let mut x=3 if true {x=7 let x=9 x} else {let x=8 x} x"), None, 7),
            ("expression-statement", source("1 2"), None, 2),
            ("empty-arms", source("if true {} else {} 7"), None, 7),
        ])
        for name, content, formula, expected in cases:
            directory, job = package(name, content)
            program = directory / "program.dag"
            compiled = execute(job, program)
            assert compiled["execution"]["compiler_job"]["status"] == "success"
            assert compiled["execution"]["program_particle"] == compiler_particle
            if name in prior_cases:
                assert compiled["published_particle"] == prior_cases[name]["compiler_execution"]["published_particle"]
            actual = decode(program)
            emitted_formula = actual[1][1][1][1][0]
            assert actual == record(0x41525431, 0, 0, 0, emitted_formula)
            if formula is not None:
                assert emitted_formula == formula
            output = directory / "output.dag"
            executed = run(["run-artifact", program, "--input", zero, "-o", output])
            assert executed["execution"]["program_particle"] == compiled["published_particle"]
            assert decode(output) == expected
            observations.append({"case": name, "source_hex": content.hex(), "expected": expected,
                                 "compiler_execution": compiled, "program_execution": executed,
                                 "exact_independent_formula": formula is not None,
                                 "accepted_locals_identity_preserved": name in prior_cases})
            if name == "precedence":
                baseline = compiled["execution"]
                prior_program = program.read_bytes()

        negatives = [("unknown", source("missing"), 5),
                     ("self-reference", source("let x=x x"), 5),
                     ("immutable-write", source("let x=1 x=2 x"), 5),
                     ("shadow-mutability", source("let mut x=1 let x=2 x=3 x"), 5),
                     ("malformed-local", source("let x: =7 x"), 2),
                     ("bool-return", source("let x=1 x==1"), 5), ("overflow", source("18446744073709551616"), 1),
                     ("syntax", source("(1"), 2), ("utf8", b"//\xed\xa0\x80", 1),
                     ("unsupported", source("let x: U32=1 x"), 6),
                     ("stack65", source("(" * 65 + "1" + ")" * 65), 7),
                     ("source4097", b"\xff" + bytes(4096), 7)]
        negatives.extend([
            ("unselected-type", source("if true {7} else {false}"), 5),
            ("unselected-name", source("if true {7} else {missing}"), 5),
            ("branch-name-escape", source("if true {let x=7} x"), 5),
            ("bool-write-type", source("let mut b=true b=1 7"), 5),
            ("missing-return", source("let b=true if b {7}"), 5),
            ("unreachable", source("return 7 9"), 5),
            ("malformed-else", source("if true {7} else 9"), 2),
        ])
        for name, content, code in negatives:
            directory, job = package(name, content)
            report = execute(job, directory / "result.dag", emit="result")
            result = report["execution"]["compiler_job"]
            assert result["status"] == "compile_error" and len(result["diagnostics"]) == 1
            assert result["diagnostics"][0]["code"] == code
            protected = directory / "protected.dag"
            protected.write_bytes(prior_program)
            execute(job, protected, expected=1, force=True)
            assert protected.read_bytes() == prior_program
            observations.append({"case": name, "source_hex": content.hex(), "diagnostics": result["diagnostics"],
                                 "compiler_execution": report, "previous_program_preserved": True})

        local_bytes = None
        for cap in [7, 8, 16]:
            directory, job = package(f"local-cap-{cap}", source("let mut x=7 let y=x x=9 y*100+x"),
                                     {"sequence_length": cap, "diagnostics": 1})
            if cap == 7:
                report = execute(job, directory / "result.dag", emit="result")
                assert report["execution"]["compiler_job"]["diagnostics"][0]["code"] == 7
            else:
                program = directory / "program.dag"
                report = execute(job, program)
                if local_bytes is not None:
                    assert program.read_bytes() == local_bytes
                local_bytes = program.read_bytes()
                output = directory / "output.dag"
                run(["run-artifact", program, "--input", zero, "-o", output])
                assert decode(output) == 709
            observations.append({"case": "local-sequence-cap", "requested": cap,
                                 "compiler_execution": report, "expected": 709 if cap >= 8 else "diagnostic7"})

        control_bytes = None
        for cap in [1, 2, 3]:
            directory, job = package(f"block-cap-{cap}", source("if true {7}"),
                                     {"sequence_length": cap, "diagnostics": 1})
            if cap == 1:
                report = execute(job, directory / "result.dag", emit="result")
                assert report["execution"]["compiler_job"]["diagnostics"][0]["code"] == 7
            else:
                program = directory / "program.dag"
                report = execute(job, program)
                if control_bytes is not None:
                    assert program.read_bytes() == control_bytes
                control_bytes = program.read_bytes()
                output = directory / "output.dag"
                run(["run-artifact", program, "--input", zero, "-o", output])
                assert decode(output) == 7
            observations.append({"case": "block-sequence-cap", "requested": cap,
                                 "compiler_execution": report, "expected": 7 if cap >= 2 else "diagnostic7"})

        # These limits are enforced during the actual compiler execution.
        guest_visits = (112 + collection_visits(6, True) + collection_visits(4, True)
                        + collection_visits(1) + collection_visits(len(source("2+3*4")), True))
        assert baseline["compiler_job"]["input_validation_visits"] < guest_visits - 1
        for delta in [0, -1]:
            directory, job = package(f"guest-visits-{delta}", source("2+3*4"), {"validation_visits": guest_visits + delta})
            protected = directory / "program.dag"
            protected.write_bytes(prior_program)
            result = execute(job, protected, expected=0 if delta == 0 else 1, force=True)
            assert protected.read_bytes() == prior_program
            observations.append({"limit": "guest_validation_visits", "requested": guest_visits + delta,
                                 "boundary": "exact" if delta == 0 else "one-below", "compiler_execution": result,
                                 "host_admission_below_guest_boundary": True, "previous_program_preserved": True})

        for limit, metric in [("reductions", "charged_reductions"), ("evaluator_frames", "peak_frames")]:
            exact = baseline[metric]
            for delta in [0, -1]:
                directory, job = package(f"{limit}-{delta}", source("2+3*4"), {limit: exact + delta})
                protected = directory / "program.dag"
                protected.write_bytes(prior_program)
                result = execute(job, protected, expected=0 if delta == 0 else 1, force=True)
                assert protected.read_bytes() == prior_program
                observations.append({"limit": limit, "requested": exact + delta, "boundary": "exact" if delta == 0 else "one-below",
                                     "compiler_execution": result, "previous_program_preserved": True})

        # Requested arena value becomes input data. Calibrate with slack to
        # include any extra atom introduced by changing that metadata value.
        directory, job = package("arena-calibration", source("2+3*4"), {"arena_nodes": baseline["allocated_nodes"] + 32})
        calibration = execute(job, directory / "program.dag")
        exact_nodes = calibration["execution"]["allocated_nodes"]
        for delta in [0, -1]:
            directory, job = package(f"arena-{delta}", source("2+3*4"), {"arena_nodes": exact_nodes + delta})
            protected = directory / "program.dag"
            protected.write_bytes(prior_program)
            result = execute(job, protected, expected=0 if delta == 0 else 1, force=True)
            assert protected.read_bytes() == prior_program
            observations.append({"limit": "arena_nodes", "requested": exact_nodes + delta,
                                 "boundary": "exact" if delta == 0 else "one-below", "compiler_execution": result,
                                 "previous_program_preserved": True})

        # Record the actual runtime limit below the algorithmic source ceiling.
        directory, job = package("source4096-arena", b"\xff" + bytes(4095))
        protected = directory / "program.dag"
        protected.write_bytes(prior_program)
        execute(job, protected, expected=1, force=True)
        assert protected.read_bytes() == prior_program
        observations.append({"case": "source4096-arena", "result": "runtime failure; no RES1 or program publication",
                             "previous_program_preserved": True})
        directory, job = package("assignments31-arena", source("let mut x=0 " + "x=x+1 " * 31 + "x"))
        protected = directory / "program.dag"
        protected.write_bytes(prior_program)
        execute(job, protected, expected=1, force=True)
        assert protected.read_bytes() == prior_program
        observations.append({"case": "assignments31-arena", "result": "runtime failure; no program publication",
                             "previous_program_preserved": True})
        assert hashlib.sha256(compiler.read_bytes()).hexdigest() == compiler_sha

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps({"schema": "trident/native-compiler-cli/v1", "kind": "local-development",
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "compiler_sha256": compiler_sha,
        "compiler_particle": compiler_particle, "commands": commands, "observations": observations,
        "scope": "SH2 arithmetic and SH3 typed locals, scoped control and early returns; complete compiler/self-build and native execution proofs remain open"}, indent=2) + "\n")
    print(json.dumps({"commands": len(commands), "observations": len(observations), "receipt": str(args.output)}))


if __name__ == "__main__":
    main()
