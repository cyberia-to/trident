"""Installed guest Noun acceptance, including complete structured output."""
import json


def check(root, repo, run, package, execute, decode, record, observations, commands, zero, prior_program):
    vectors = json.loads((repo.parent / "joy/cli/tests/artifact_vectors.json").read_text())
    vector = next(v for v in vectors if v["name"] == "identity_tree")
    input_file = root / "nested-input.dag"
    input_file.write_bytes(bytes(vector["input"]))
    input_value = ((1, 2), 3)
    assert decode(input_file) == input_value

    def source(body):
        return f"program sample fn main(input:Noun)->Noun{{{body}}}".encode()

    def functions(body):
        return ("program sample " + body).encode()

    cases = [
        ("noun-input", source("input"), input_value),
        ("noun-padded-frame", source("let x=input let y=x let z=y let q=z q"), input_value),
        ("noun-swap", source("nox_noun_pair(nox_noun_tail(input),nox_noun_head(input))"), (3, (1, 2))),
        ("noun-share", source("nox_noun_pair(input,input)"), (input_value, input_value)),
        ("noun-alias-equality", source("let x=input if nox_noun_eq(x,input){input}else{nox_noun_atom(0)}"), input_value),
        ("noun-unequal", source("if nox_noun_eq(nox_noun_head(input),nox_noun_tail(input)){input}else{nox_noun_atom(9)}"), 9),
        ("noun-as-field", source("nox_noun_atom(nox_noun_as_field(nox_noun_tail(input))+4)"), 7),
        ("noun-helper", functions("fn id(x:Noun)->Noun{x} fn main(input:Noun)->Noun{id(input)}"), input_value),
        ("noun-loop", source("let mut x=input for i in 0..3{x=nox_noun_pair(nox_noun_atom(as_field(i)),x)} x"), (2, (1, (0, input_value)))),
        ("noun-loop-return", source("for i in 0..2{for j in 0..3{return input}} input"), input_value),
        ("noun-override", functions("fn nox_noun_head(x:Noun)->Noun{x} fn main(input:Noun)->Noun{nox_noun_head(input)}"), input_value),
        ("noun-main-override", functions("fn main()->Field{7} fn main(input:Noun)->Noun{input}"), input_value),
        ("noun-scalar-main", functions("fn main(input:Noun)->Noun{input} fn main()->Field{7}"), 7),
        ("noun-unselected-trap", source("if false{return nox_noun_head(nox_noun_atom(7))} input"), input_value),
    ]
    for name, content, expected in cases:
        directory, job = package(name, content, {"arena_nodes": 786432})
        program = directory / "program.dag"
        compiled = execute(job, program)
        assert compiled["execution"]["compiler_job"]["status"] == "success"
        artifact = decode(program)
        formula = artifact[1][1][1][1][0]
        assert artifact == record(0x41525431, 0, 0, 0, formula)
        output = directory / "output.dag"
        executed = run(["run-artifact", program, "--input", input_file, "-o", output])
        assert decode(output) == expected, (name, decode(output), expected)
        assert executed["execution"]["program_particle"] == compiled["published_particle"]
        if name == "noun-input":
            assert output.read_bytes() == input_file.read_bytes()
        observations.append({"case": name, "source_hex": content.hex(), "expected": expected,
                             "input_particle": input_file.read_bytes()[8:40].hex(),
                             "output_particle": output.read_bytes()[8:40].hex(),
                             "compiler_execution": compiled, "program_execution": executed,
                             "program_bytes": len(program.read_bytes()), "complete_output_checked": True})

    for name, content, code in [
        ("noun-entry-result", functions("fn main(input:Noun)->Field{7}"), 3),
        ("noun-entry-parameter", functions("fn main(input:Field)->Noun{nox_noun_atom(input)}"), 3),
        ("noun-entry-arity", functions("fn main(a:Noun,b:Noun)->Noun{a}"), 3),
        ("noun-operator-equality", source("input==input input"), 5),
        ("noun-condition", source("if input{return input} input"), 5),
        ("noun-write-type", source("let mut x=input x=7 input"), 5),
        ("noun-builtin-type", source("nox_noun_pair(input,7)"), 5),
        ("noun-builtin-spelling", source("nox_noun_as_fielx(input) input"), 5),
        ("noun-qualified", source("let noun=input noun.head(input)"), 6),
    ]:
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

    for name, content in [
        ("noun-head-trap", source("nox_noun_head(nox_noun_atom(7))")),
        ("noun-tail-trap", source("nox_noun_tail(nox_noun_atom(7))")),
        ("noun-field-trap", source("nox_noun_atom(nox_noun_as_field(input))")),
        ("noun-discarded-trap", source("nox_noun_head(nox_noun_atom(7)) input")),
        ("noun-argument-trap", functions("fn f(x:Noun,y:Noun)->Noun{y} fn main(input:Noun)->Noun{f(nox_noun_head(nox_noun_atom(7)),input)}")),
    ]:
        directory, job = package(name, content, {"arena_nodes": 786432})
        program = directory / "program.dag"
        compiled = execute(job, program)
        assert compiled["execution"]["compiler_job"]["status"] == "success"
        output = directory / "output.dag"
        output.write_bytes(zero.read_bytes())
        run(["run-artifact", program, "--input", input_file, "-o", output, "--force"], expected=1)
        assert "Error(" in commands[-1]["stderr"], commands[-1]
        assert output.read_bytes() == zero.read_bytes()
        observations.append({"case": name, "source_hex": content.hex(), "compiler_execution": compiled,
                             "program_execution_error": commands[-1]["stderr"], "previous_output_preserved": True})
