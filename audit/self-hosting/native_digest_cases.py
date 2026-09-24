"""Installed Digest acceptance against the input file's complete particle."""
import json


def check(root, repo, run, package, execute, decode, record, observations, commands, zero, prior_program):
    vectors = json.loads((repo.parent / "joy/cli/tests/artifact_vectors.json").read_text())
    vector = next(v for v in vectors if v["name"] == "identity_tree")
    input_file = root / "digest-input.dag"
    input_file.write_bytes(bytes(vector["input"]))
    raw = input_file.read_bytes()
    words = [int.from_bytes(raw[8 + 8 * i:16 + 8 * i], "little") for i in range(4)]
    full = ((words[0], words[1]), (words[2], words[3]))
    value = decode(input_file)

    def source(body):
        return f"program sample fn main(input:Noun)->Noun{{{body}}}".encode()

    output = "nox_noun_pair(nox_noun_pair(nox_noun_atom(d[0]),nox_noun_atom(d[1])),nox_noun_pair(nox_noun_atom(d[2]),nox_noun_atom(d[3])))"
    cases = [
        ("digest-identity", source("let d:Digest=nox_noun_identity(input) " + output), full),
        ("digest-persistence", source("let d=nox_noun_identity(input) let mut copy=d copy=nox_noun_identity(nox_noun_atom(0)) " + output), full),
        ("digest-helper", ("program sample fn id(d:Digest)->Digest{d} fn main(input:Noun)->Noun{let d=id(nox_noun_identity(input)) " + output + "}").encode(), full),
        ("digest-loop", source("let mut d=nox_noun_identity(nox_noun_atom(0)) for i in 0..3{d=nox_noun_identity(input)} " + output), full),
        ("digest-equal", source("let d=nox_noun_identity(input) if d==nox_noun_identity(input){input}else{nox_noun_atom(0)}"), value),
        ("digest-unequal", source("let d=nox_noun_identity(input) if d==nox_noun_identity(nox_noun_atom(0)){nox_noun_atom(0)}else{input}"), value),
        ("digest-u32-index", source("nox_noun_atom(nox_noun_identity(input)[as_u32(sub(3,1))])"), words[2]),
        ("digest-index-newline", source("let d=nox_noun_identity(input) nox_noun_atom(d\n[3])"), words[3]),
        ("digest-field-normalized", source("nox_noun_atom(nox_noun_identity(input)[18446744069414584321])"), words[0]),
        ("digest-unselected-trap", source("if false{nox_noun_identity(input)[4]} input"), value),
    ]
    for name, content, expected in cases:
        directory, job = package(name, content, {"arena_nodes": 786432})
        program = directory / "program.dag"
        compiled = execute(job, program)
        assert compiled["execution"]["compiler_job"]["status"] == "success"
        artifact = decode(program)
        assert artifact == record(0x41525431, 0, 0, 0, artifact[1][1][1][1][0])
        output_file = directory / "output.dag"
        executed = run(["run-artifact", program, "--input", input_file, "-o", output_file])
        assert decode(output_file) == expected, (name, decode(output_file), expected)
        assert executed["execution"]["program_particle"] == compiled["published_particle"]
        observations.append({"case": name, "source_hex": content.hex(), "expected": expected,
                             "input_particle": raw[8:40].hex(), "output_particle": output_file.read_bytes()[8:40].hex(),
                             "compiler_execution": compiled, "program_execution": executed,
                             "program_bytes": len(program.read_bytes()), "complete_output_checked": True})

    for name, body, code in [
        ("digest-wrong-type", "let d:Digest=input input", 5),
        ("digest-wrong-index", "nox_noun_identity(input)[true] input", 5),
        ("digest-wrong-arity", "nox_noun_identity(input,input) input", 5),
        ("digest-wrong-spelling", "nox_noun_identitx(input) input", 5),
        ("digest-mismatched-delimiter", "nox_noun_identity(input)[sub(3,1] input", 2),
        ("digest-condition", "if nox_noun_identity(input){return input} input", 5),
    ]:
        content = source(body)
        directory, job = package(name, content, {"arena_nodes": 786432})
        report = execute(job, directory / "result.dag", emit="result")
        result = report["execution"]["compiler_job"]
        assert result["status"] == "compile_error" and len(result["diagnostics"]) == 1
        assert result["diagnostics"][0]["code"] == code, (name, result)
        protected = directory / "protected.dag"
        protected.write_bytes(prior_program)
        execute(job, protected, expected=1, force=True)
        assert protected.read_bytes() == prior_program
        observations.append({"case": name, "source_hex": content.hex(), "diagnostics": result["diagnostics"],
                             "compiler_execution": report, "previous_program_preserved": True})

    for name, body, error in [
        ("digest-oob", "nox_noun_atom(nox_noun_identity(input)[4])", "InvZero"),
        ("digest-oob-discarded", "nox_noun_identity(input)[18446744069414584320] input", "InvZero"),
        ("digest-base-first", "nox_noun_atom(nox_noun_identity(nox_noun_head(nox_noun_atom(7)))[nox_noun_as_field(input)])", "AxisError"),
        ("digest-index-type-trap", "nox_noun_atom(nox_noun_identity(input)[nox_noun_as_field(input)])", "TypeError"),
    ]:
        content = source(body)
        directory, job = package(name, content, {"arena_nodes": 786432})
        program = directory / "program.dag"
        compiled = execute(job, program)
        assert compiled["execution"]["compiler_job"]["status"] == "success"
        output_file = directory / "output.dag"
        output_file.write_bytes(zero.read_bytes())
        run(["run-artifact", program, "--input", input_file, "-o", output_file, "--force"], expected=1)
        assert commands[-1]["stderr"] == f"error: execution failed: {error}\n", commands[-1]
        assert output_file.read_bytes() == zero.read_bytes()
        observations.append({"case": name, "source_hex": content.hex(), "compiler_execution": compiled,
                             "program_execution_error": commands[-1]["stderr"], "previous_output_preserved": True})
