"""SH2/SH3 installed CLI acceptance: fixed guest through Noun, Digest and tuple values."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import native_noun_cases
import native_digest_cases
import native_tuple_cases

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
    accepted = json.loads((repo / "audit/self-hosting/native-control-cli.json").read_text())
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

        job_hosts = {}

        def package(name, source, limits=None):
            directory = root / name
            directory.mkdir()
            (directory / "source.tri").write_bytes(source)
            manifest = copy.deepcopy(base)
            if limits:
                manifest["limits"].update(limits)
            (directory / "package.json").write_text(json.dumps(manifest))
            job = directory / "job.dag"
            arena = 786432 if manifest["limits"]["arena_nodes"] > 196608 else 196608
            job_hosts[job] = arena
            run(["pack-job", "--compiler", compiler, "--manifest", directory / "package.json", "-o", job,
                 *host, "--arena-nodes", arena])
            return directory, job

        def execute(job, output, emit="program", expected=0, force=False):
            return run(["run-artifact", compiler, "--input", job, "--emit", emit, "-o", output,
                        *host, "--arena-nodes", job_hosts[job], *(["--force"] if force else [])], expected)

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
        def function_source(body):
            return ("program sample " + body).encode()

        cases.extend([
            ("unused-function", function_source("fn helper()->Field{7} fn main()->Field{9}"), (1, 9), 9),
            ("function-forward", function_source("fn main()->Field{f(2)} fn f(x:Field)->Field{x+1}"), None, 3),
            ("function-nested", function_source("fn f(a:Field,b:Field)->Field{10*a+b} fn g(x:Field)->Field{x+1} fn main()->Field{f(g(2),g(3))}"), None, 34),
            ("function-frame", function_source("fn f(a:Field,b:Field,c:Field)->Field{let d=9 let e=8 a+10*b+100*c+d+e} fn main()->Field{f(1,2,3,)}"), None, 338),
            ("function-early-return", function_source("fn f(x:Field)->Field{if x==2{return 7} 9} fn main()->Field{let x=3 f(2)+f(x)+x}"), None, 19),
            ("function-local-name", function_source("fn f()->Field{7} fn main()->Field{let f=9 f()+f}"), None, 16),
            ("function-bool", function_source("fn f(x:Bool)->Bool{if x{return false} true} fn main()->Field{if f(false){7}else{9}}"), None, 7),
            ("function-unit", function_source("fn f(){} fn main()->Field{let mut x=f() x=f() if x==f(){7}else{9}}"), None, 7),
            ("function-bare-return", function_source("fn f(){return} fn g(){return f()} fn main()->Field{g() 7}"), None, 7),
            ("duplicate-parameters", function_source("fn f(x:Field,x:Bool)->Bool{x} fn main()->Field{if f(4,true){7}else{9}}"), None, 7),
            ("duplicate-functions", function_source("fn f()->Field{7} fn f()->Bool{true} fn main()->Field{if f(){7}else{9}}"), None, 7),
            ("duplicate-main", function_source("fn main(x:Field)->Field{x} fn main()->Field{9}"), (1,9), 9),
            ("table-order-original", function_source("fn f(x:Field)->Field{x+1} fn g(x:Field)->Field{f(x)*2} fn main()->Field{g(3)}"), None, 8),
            ("table-order-reversed", function_source("fn main()->Field{g(3)} fn g(x:Field)->Field{f(x)*2} fn f(x:Field)->Field{x+1}"), None, 8),
            ("table-order-unused", function_source("fn unused()->Field{9} fn main()->Field{g(3)} fn g(x:Field)->Field{f(x)*2} fn f(x:Field)->Field{x+1}"), None, 8),
        ])
        cases.extend([
            ("scalar-zero", source("as_field(as_u32(0))"), None, 0),
            ("scalar-maximum", source("as_field(as_u32(4294967295))"), None, 4294967295),
            ("scalar-modulus", source("as_field(as_u32(18446744069414584321))"), None, 0),
            ("scalar-modulus-plus-one", source("as_field(as_u32(18446744069414584322))"), None, 1),
            ("scalar-local", source("let mut x:U32=as_u32(7) let y=x x=as_u32(9) as_field(y)*10+as_field(x)"), None, 79),
            ("scalar-mask", source("as_field(as_u32(4294967295)&as_u32(4278190080))"), None, 4278190080),
            ("scalar-comparison", source("if as_u32(0)<as_u32(4294967295){7}else{9}"), None, 7),
            ("scalar-precedence", source("if as_u32(2)<as_u32(3)&as_u32(1){7}else{9}"), None, 9),
            ("scalar-subtraction", source("sub(sub(20,3),sub(7,2))"), None, 12),
            ("scalar-typed-call", function_source("fn f(x:U32)->U32{x} fn main()->Field{as_field(f(as_u32(9)))}"), None, 9),
            ("scalar-builtin-override", function_source("fn main()->Field{as_u32(7)} fn as_u32(x:Field)->Field{x+2}"), None, 9),
            ("scalar-local-collision", source("let as_u32=9 let as_field=8 as_field(as_u32(7))+as_field"), None, 15),
            ("scalar-unselected-trap", source("if false{return as_field(as_u32(4294967296))} 7"), None, 7),
        ])
        cases.extend([
            (f"loop-count-{count}", source(f"let mut x=0 for i in 0..{count}{{x=x+1}} x"), None, count)
            for count in [0, 1, 4097, 5000]
        ])
        cases.extend([
            ("loop-nonzero", source("let mut x=0 for i in 2..5{x=x*10+as_field(i)} x"), None, 234),
            ("loop-shadow", source("let i=7 let mut x=0 for i in 0..3{x=x+as_field(i)} x+i"), None, 10),
            ("loop-nested", source("let mut x=0 for i in 0..3{for i in 1..3{x=x+as_field(i)} x=x+as_field(i)*10} x"), None, 39),
            ("loop-return", source("for i in 0..5000{if as_field(i)==3{return 7}} as_field(as_u32(4294967296))"), None, 7),
            ("loop-helper", function_source("fn f(x:Field)->Field{for i in 0..3{return x+as_field(i)}} fn main()->Field{let mut x=0 for i in 0..4{x=x+f(2)} x}"), None, 8),
            ("loop-u32-last", source("for i in 4294967295..4294967296{return as_field(i)} 7"), None, 4294967295),
            ("loop-full-range-return", source("for i in 0..4294967296{return as_field(i)+7} 9"), None, 7),
            ("loop-tail-discard", source("for i in 0..1{7} 9"), None, 9),
        ])
        loop_sizes = {}
        table_bytes = None
        for name, content, formula, expected in cases:
            directory, job = package(name, content, {"arena_nodes": 786432} if name.startswith(("scalar-", "loop-")) else None)
            program = directory / "program.dag"
            compiled = execute(job, program)
            assert compiled["execution"]["compiler_job"]["status"] == "success"
            assert compiled["execution"]["program_particle"] == compiler_particle
            if name in prior_cases:
                assert compiled["published_particle"] == prior_cases[name]["compiler_execution"]["published_particle"]
            if name.startswith("table-order-"):
                if table_bytes is not None:
                    assert program.read_bytes() == table_bytes
                table_bytes = program.read_bytes()
            actual = decode(program)
            emitted_formula = actual[1][1][1][1][0]
            assert actual == record(0x41525431, 0, 0, 0, emitted_formula)
            if formula is not None:
                assert emitted_formula == formula
            output = directory / "output.dag"
            limits = ["--budget", "10000000", "--frames", "65536"] if name.startswith("loop-") else []
            executed = run(["run-artifact", program, "--input", zero, "-o", output, *limits])
            if name.startswith("loop-count-"):
                loop_sizes[expected] = len(program.read_bytes())
            if name == "loop-count-5000":
                saved = output.read_bytes()
                for constrained, error in [([], "Frames"), (["--frames", "65536", "--budget", "1"], "budget exhausted"), (["--frames", "65536", "--arena-nodes", "1000"], "Unavailable")]:
                    run(["run-artifact", program, "--input", zero, "-o", output, "--force", *constrained], expected=1)
                    assert error in commands[-1]["stderr"]
                    assert output.read_bytes() == saved
            assert executed["execution"]["program_particle"] == compiled["published_particle"]
            assert decode(output) == expected
            observations.append({"case": name, "source_hex": content.hex(), "expected": expected,
                                 "compiler_execution": compiled, "program_execution": executed,
                                 "program_bytes": len(program.read_bytes()), "exact_independent_formula": formula is not None,
                                 "accepted_control_identity_preserved": name in prior_cases})
            if name == "precedence":
                baseline = compiled["execution"]
                prior_program = program.read_bytes()

        assert abs(loop_sizes[1] - loop_sizes[5000]) < 1024
        assert abs(loop_sizes[4097] - loop_sizes[5000]) < 128

        negatives = [("unknown", source("missing"), 5),
                     ("self-reference", source("let x=x x"), 5),
                     ("immutable-write", source("let x=1 x=2 x"), 5),
                     ("shadow-mutability", source("let mut x=1 let x=2 x=3 x"), 5),
                     ("malformed-local", source("let x: =7 x"), 2),
                     ("bool-return", source("let x=1 x==1"), 5), ("overflow", source("18446744073709551616"), 1),
                     ("syntax", source("(1"), 2), ("utf8", b"//\xed\xa0\x80", 1),
                     ("unsupported", source("let x: XField=1 x"), 6),
                     ("u32-field-literal", source("let x: U32=1 x"), 5),
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
        negatives.extend([
            ("call-unknown", function_source("fn main()->Field{missing()}"), 5),
            ("call-arity", function_source("fn f(x:Field)->Field{x} fn main()->Field{f()}"), 5),
            ("call-type", function_source("fn f(x:Field)->Field{x} fn main()->Field{f(true)}"), 5),
            ("call-recursive", function_source("fn main()->Field{main()}"), 5),
            ("call-unused-cycle", function_source("fn f()->Field{g()} fn g()->Field{f()} fn main()->Field{7}"), 5),
            ("call-unselected-cycle", function_source("fn f()->Field{if false{return f()} 7} fn main()->Field{7}"), 5),
            ("unused-body-type", function_source("fn f()->Field{false} fn main()->Field{7}"), 5),
            ("unit-condition", function_source("fn f(){} fn main()->Field{if f(){7}else{9}}"), 5),
        ])
        negatives.extend([
            ("scalar-builtin-type", source("as_field(7)"), 5),
            ("scalar-builtin-arity", source("as_u32(1,2)"), 5),
            ("scalar-addition-type", source("as_field(as_u32(1)+as_u32(2))"), 5),
            ("scalar-condition-type", source("if as_u32(0){7}else{9}"), 5),
            ("scalar-comparison-type", source("if 1<2{7}else{9}"), 5),
            ("scalar-equality-type", source("if as_u32(7)==7{7}else{9}"), 5),
            ("scalar-unknown-spelling", source("as_fiele(as_u32(7))"), 5),
            ("scalar-qualified-unbound", source("convert.as_u32(7)"), 5),
            ("scalar-qualified-name", source("let convert=7 convert.as_u32(7)"), 6),
        ])
        negatives.extend([
            ("loop-missing-bound", source("for i in 0..{} 7"), 2),
            ("loop-bool-bound", source("for i in true..1{} 7"), 5),
            ("loop-index-write", source("for i in 0..2{i=as_u32(0)} 7"), 5),
            ("loop-scope-escape", source("for i in 0..2{let x=7} x"), 5),
            ("loop-empty-type", source("for i in 0..0{return false} 7"), 5),
            ("loop-tail-coverage", source("for i in 0..1{7}"), 5),
            ("loop-field-wrap", source("for i in 0..18446744069414584321{} 7"), 5),
            ("loop-u32-overflow", source("for i in 0..4294967297{} 7"), 5),
            ("loop-dynamic-unsupported", source("let n=3 for i in 0..n bounded 3{} 7"), 6),
        ])
        for name, content, code in negatives:
            directory, job = package(name, content, {"arena_nodes": 786432} if name.startswith(("scalar-", "loop-")) else None)
            report = execute(job, directory / "result.dag", emit="result")
            result = report["execution"]["compiler_job"]
            assert result["status"] == "compile_error" and len(result["diagnostics"]) == 1
            assert result["diagnostics"][0]["code"] == code, (name, code, result["diagnostics"])
            protected = directory / "protected.dag"
            protected.write_bytes(prior_program)
            execute(job, protected, expected=1, force=True)
            assert protected.read_bytes() == prior_program
            observations.append({"case": name, "source_hex": content.hex(), "diagnostics": result["diagnostics"],
                                 "compiler_execution": report, "previous_program_preserved": True})

        for name, content in [
            ("scalar-overflow", source("as_field(as_u32(4294967296))")),
            ("scalar-field-maximum", source("as_field(as_u32(18446744069414584320))")),
            ("scalar-discarded-overflow", source("as_u32(4294967296) 7")),
            ("scalar-unused-local-overflow", source("let x=as_u32(4294967296) 7")),
            ("scalar-unused-argument-overflow", function_source("fn f(x:U32)->Field{7} fn main()->Field{f(as_u32(4294967296))}")),
        ]:
            directory, job = package(name, content, {"arena_nodes": 786432})
            program = directory / "program.dag"
            compiled = execute(job, program)
            assert compiled["execution"]["compiler_job"]["status"] == "success"
            output = directory / "output.dag"
            output.write_bytes(zero.read_bytes())
            run(["run-artifact", program, "--input", zero, "-o", output, "--force"], expected=1)
            assert "InvZero" in commands[-1]["stderr"]
            assert output.read_bytes() == zero.read_bytes()
            observations.append({"case": name, "source_hex": content.hex(), "compiler_execution": compiled,
                                 "program_execution_error": "InvZero", "previous_output_preserved": True})

        native_noun_cases.check(root, repo, run, package, execute, decode, record, observations, commands, zero, prior_program)
        native_digest_cases.check(root, repo, run, package, execute, decode, record, observations, commands, zero, prior_program)
        native_tuple_cases.check(root, repo, run, package, execute, decode, record, observations, commands, zero, prior_program)

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
        nested = "f(" * 64 + "1" + ")" * 64
        directory, job = package("calls64-arena", function_source("fn f(x:Field)->Field{x} fn main()->Field{" + nested + "}"))
        protected = directory / "program.dag"
        protected.write_bytes(prior_program)
        failure = execute(job, protected, expected=1, force=True)
        assert protected.read_bytes() == prior_program
        observations.append({"case": "calls64-arena", "result": "runtime failure; no program publication",
                             "compiler_execution": failure, "previous_program_preserved": True})
        assert hashlib.sha256(compiler.read_bytes()).hexdigest() == compiler_sha

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps({"schema": "trident/native-compiler-cli/v1", "kind": "local-development",
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "compiler_sha256": compiler_sha,
        "compiler_particle": compiler_particle, "commands": commands, "observations": observations,
        "scope": "SH2 arithmetic and SH3 locals, scoped control, reusable functions, checked U32 scalar operations reusable literal-range loops and structured Noun, Digest and tuple values; complete compiler/self-build and native execution proofs remain open"}, indent=2) + "\n")
    print(json.dumps({"commands": len(commands), "observations": len(observations), "receipt": str(args.output)}))


if __name__ == "__main__":
    main()
