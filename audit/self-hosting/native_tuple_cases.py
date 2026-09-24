"""Fresh tuple JOB1 files through one fixed installed compiler and Joy."""
import json


def check(root, repo, run, package, execute, decode, record, observations, commands, zero, prior_program):
    vectors = json.loads((repo.parent / "joy/cli/tests/artifact_vectors.json").read_text())
    vector = next(v for v in vectors if v["name"] == "identity_tree")
    input_file = root / "tuple-input.dag"
    input_file.write_bytes(bytes(vector["input"]))
    raw = input_file.read_bytes()
    value = decode(input_file)
    words = [int.from_bytes(raw[8 + 8 * i:16 + 8 * i], "little") for i in range(4)]

    def source(body, helpers=""):
        return f"program sample {helpers} fn main(input:Noun)->Noun{{{body}}}".encode()

    full_digest = "nox_noun_pair(nox_noun_pair(nox_noun_atom(a),nox_noun_atom(b)),nox_noun_pair(nox_noun_atom(c),nox_noun_atom(d)))"
    cases = [
        ("tuple-whole-noun", source("let(n,x)=(input,7) nox_noun_pair(n,nox_noun_atom(x))"), (value, 7)),
        ("tuple-helper", source("let(n,x)=id((input,7)) nox_noun_pair(n,nox_noun_atom(x))", "fn id(x:(Noun,Field))->(Noun,Field){return x}"), (value, 7)),
        ("tuple-nested-types", source("let(p,u):((Noun,Bool),U32)=((input,true),as_u32(7)) let(n,b)=p if b{nox_noun_pair(n,nox_noun_atom(as_field(u)))}else{input}"), (value, 7)),
        ("tuple-persistence", source("let mut t=(input,7) let old=t t=(nox_noun_atom(0),9) let(n,x)=old nox_noun_pair(n,nox_noun_atom(x))"), (value, 7)),
        ("tuple-unit", source("let(u,n,x)=(unit(),input,7) if u==unit(){nox_noun_pair(n,nox_noun_atom(x))}else{input}", "fn unit(){}"), (value, 7)),
        ("tuple-swap", source("let mut(a,b)=(2,7) ((a,b))=(b,a) nox_noun_pair(nox_noun_atom(a),nox_noun_atom(b))"), (7, 2)),
        ("tuple-repeat-declare", source("let(x,x)=(7,true) if x{input}else{nox_noun_atom(0)}"), value),
        ("tuple-repeat-assign", source("let mut x=1 (x,x)=(7,9) nox_noun_atom(x)"), 9),
        ("tuple-discard", source("let(_,x,_)=(input,7,true) nox_noun_atom(x)"), 7),
        ("tuple-owned-links", source("let(a,b,c)=(f(1),f(f(2)),f(3)) nox_noun_atom(a*100+b*10+c)", "fn f(x:Field)->Field{x+1}"), 244),
        ("tuple-nested-grouping", source("let(a,b,c)=(1,((2,3)),4,) let(x,y)=b nox_noun_atom(a*1000+x*100+y*10+c)"), 1234),
        ("tuple-equality", source("let x:(Field,(Bool,U32))=(7,(true,as_u32(8))) if x==(7,(true,as_u32(8))){input}else{nox_noun_atom(0)}"), value),
        ("tuple-digest-helper", source("let(a,b,c,d):Digest=id(input) " + full_digest, "fn id(n:Noun)->Digest{nox_noun_identity(n)}"), ((words[0], words[1]), (words[2], words[3]))),
        ("tuple-unselected-trap", source("if false{let(_,_)=(nox_noun_head(nox_noun_atom(7)),7)} input"), value),
    ]
    for name, content, expected in cases:
        directory, job = package(name, content, {"arena_nodes": 786432})
        program = directory / "program.dag"
        compiled = execute(job, program)
        assert compiled["execution"]["compiler_job"]["status"] == "success"
        artifact = decode(program)
        assert artifact == record(0x41525431, 0, 0, 0, artifact[1][1][1][1][0])
        output = directory / "output.dag"
        executed = run(["run-artifact", program, "--input", input_file, "-o", output])
        assert decode(output) == expected, (name, decode(output), expected)
        assert executed["execution"]["program_particle"] == compiled["published_particle"]
        observations.append({"case": name, "source_hex": content.hex(), "expected": expected,
                             "input_particle": raw[8:40].hex(), "output_particle": output.read_bytes()[8:40].hex(),
                             "compiler_execution": compiled, "program_execution": executed,
                             "program_bytes": len(program.read_bytes()), "complete_output_checked": True})

    for name, body, code in [
        ("tuple-wrong-component", "let x:(Field,Bool)=(7,9) input", 5),
        ("tuple-wrong-arity", "let(a,b,c)=(7,9) input", 5),
        ("tuple-immutable-target", "let(a,b)=(7,9) (a,b)=(9,7) input", 5),
        ("tuple-wrong-assigned-type", "let mut(a,b)=(7,9) (a,b)=(true,7) input", 5),
        ("tuple-nested-noun-equality", "let a=((input,true),7) a==a input", 5),
        ("tuple-singleton-value", "(1,) input", 2),
        ("tuple-trailing-type-comma", "let x:(Field,)=(1,2) input", 2),
        ("tuple-arity17", "(" + ",".join(["7"] * 17) + ") input", 7),
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
        ("tuple-left-first", "let(a,b)=(nox_noun_head(nox_noun_atom(7)),nox_noun_as_field(input)) input", "AxisError"),
        ("tuple-discard-trap", "let(_,_)=(nox_noun_head(nox_noun_atom(7)),7) input", "AxisError"),
        ("tuple-right-trap", "let(_,_)=(input,nox_noun_as_field(input)) input", "TypeError"),
        ("tuple-assignment-trap", "let mut a=input let mut b=7 (a,b)=(nox_noun_head(nox_noun_atom(7)),nox_noun_as_field(input)) input", "AxisError"),
    ]:
        content = source(body)
        directory, job = package(name, content, {"arena_nodes": 786432})
        program = directory / "program.dag"
        compiled = execute(job, program)
        assert compiled["execution"]["compiler_job"]["status"] == "success"
        output = directory / "output.dag"
        output.write_bytes(zero.read_bytes())
        run(["run-artifact", program, "--input", input_file, "-o", output, "--force"], expected=1)
        assert commands[-1]["stderr"] == f"error: execution failed: {error}\n", commands[-1]
        assert output.read_bytes() == zero.read_bytes()
        observations.append({"case": name, "source_hex": content.hex(), "compiler_execution": compiled,
                             "program_execution_error": commands[-1]["stderr"], "previous_output_preserved": True})
