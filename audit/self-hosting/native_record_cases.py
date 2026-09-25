"""Installed fixed-C1 acceptance of nominal declarations, constructors and reads."""
import json


def check(root, repo, run, package, execute, decode, record, observations, commands, zero, prior_program):
    vectors = json.loads((repo.parent / "joy/cli/tests/artifact_vectors.json").read_text())
    vector = next(v for v in vectors if v["name"] == "identity_tree")
    input_file = root / "record-input.dag"
    input_file.write_bytes(bytes(vector["input"]))
    value = decode(input_file)
    word = int.from_bytes(input_file.read_bytes()[24:32], "little")

    def source(declarations, body):
        return f"program sample {declarations} fn main(input:Noun)->Noun{{{body}}}".encode()

    wide_fields = ",".join(f"f{i}:Field" for i in range(32))
    wide_values = ",".join(f"f{i}:{i}" for i in reversed(range(32)))
    long_type = "S" + "n" * 256
    long_field = "f" * 256
    cases = [
        ("record-empty", source("struct Empty{}", "let s=Empty{} if s==(Empty{}){input}else{nox_noun_atom(0)}"), value),
        ("record-shorthand", source("struct Pair{x:Field,y:Field}", "let x=7 let p:Pair=Pair{y:9,x,} nox_noun_atom(p.x*10+p.y)"), 79),
        ("record-typed-call", source("pub struct S{pub x:Noun} pub fn id(x:S)->S{x}", "id(S{x:input}).x"), value),
        ("record-nested", source("struct S{x:Noun} struct Box{inner:S,n:Field}", "Box{n:9,inner:S{x:input}}.inner.x"), value),
        ("record-unit-name", source("struct Unit{}", "let s:Unit=Unit{} if s==(Unit{}){input}else{nox_noun_atom(0)}"), value),
        ("record-later-body", b"program sample fn main(input:Noun)->Noun{let s:Later=Later{x:input} s.x} struct Later{x:Noun}", value),
        ("record-snapshot", source("struct S{x:Noun}", "let mut s=S{x:input} let old=s s=S{x:nox_noun_atom(0)} old.x"), value),
        ("record-condition", source("struct S{flag:Bool}", "if (S{flag:true}).flag{input}else{nox_noun_atom(0)}"), value),
        ("record-index-condition", source("struct S{x:Field}", "let d=nox_noun_identity(input) if d[S{x:2}.x]==d[2]{input}else{nox_noun_atom(0)}"), value),
        ("record-tuples", source("struct S{x:Noun}", "let(a,b):(S,S)=(S{x:input},S{x:nox_noun_atom(7)}) nox_noun_pair(a.x,b.x)"), (value, 7)),
        ("record-digest", source("struct S{value:Noun,fields:(Field,Bool),digest:Digest}", "let s=S{digest:nox_noun_identity(input),fields:(7,true),value:input} let(x,b)=s.fields if b{nox_noun_pair(s.value,nox_noun_pair(nox_noun_atom(x),nox_noun_atom(s.digest[2])))}else{input}"), (value, (7, word))),
        ("record-wide32", source(f"struct Wide{{{wide_fields}}}", f"let w=Wide{{{wide_values}}} nox_noun_atom(w.f31*100+w.f0+w.f17)"), 3117),
        ("record-long-type", source(f"struct {long_type}{{x:Noun}}", f"{long_type}{{x:input}}.x"), value),
        ("record-long-fields", source(f"struct S{{{long_field}a:Field,{long_field}b:Field}}", "input"), value),
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
                             "input_particle": input_file.read_bytes()[8:40].hex(), "output_particle": output.read_bytes()[8:40].hex(),
                             "compiler_execution": compiled, "program_execution": executed,
                             "program_bytes": len(program.read_bytes()), "complete_output_checked": True})

    negatives = [
        ("record-wrong-type", source("struct S{x:Field}", "S{x:true} input"), 5),
        ("record-missing", source("struct S{x:Field}", "S{} input"), 5),
        ("record-extra", source("struct S{x:Field}", "S{x:7,y:9} input"), 5),
        ("record-duplicate", source("struct S{x:Field}", "S{x:7,x:9} input"), 5),
        ("record-missing-value", source("struct S{x:Field}", "S{x:} input"), 2),
        ("record-mismatch-paren", source("struct S{x:Field}", "S{x:7) input"), 2),
        ("record-mismatch-index", source("struct S{x:Field}", "S{x:7] input"), 2),
        ("record-mismatch-group", source("struct S{x:Field}", "S{x:(7} input"), 2),
        ("record-unknown-field", source("struct S{x:Field}", "S{x:7}.y input"), 5),
        ("record-nominal-mismatch", source("struct S{x:Field} struct T{x:Field}", "let t:T=S{x:7} input"), 5),
        ("record-noun-equality", source("struct S{x:Noun}", "let s=S{x:input} s==s input"), 5),
        ("record-late-signature", source("fn f(s:Later)->Later{s} struct Later{x:Field}", "input"), 5),
        ("record-recursive-type", source("struct S{x:S}", "input"), 5),
        ("record-duplicate-type", source("struct S{} struct S{}", "input"), 5),
        ("record-duplicate-field", source("struct S{x:Field,x:Field}", "input"), 5),
        ("record-fields33", source("struct S{" + wide_fields + ",f32:Field}", "input"), 7),
    ]
    for name, content, code in negatives:
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

    for name, content, error in [
        ("record-declaration-order", source("struct Pair{first:Field,second:Noun}", "let p=Pair{second:nox_noun_head(nox_noun_atom(1)),first:as_field(as_u32(4294967296))} p.second"), "InvZero"),
        ("record-unused-trap", source("struct S{x:Noun}", "let unused=S{x:nox_noun_head(nox_noun_atom(1))} input"), "AxisError"),
    ]:
        directory, job = package(name, content, {"arena_nodes": 786432})
        program = directory / "program.dag"
        compiled = execute(job, program)
        assert compiled["execution"]["compiler_job"]["status"] == "success"
        output = directory / "output.dag"
        output.write_bytes(zero.read_bytes())
        run(["run-artifact", program, "--input", input_file, "-o", output, "--force"], expected=1)
        assert commands[-1]["stderr"] == f"error: execution failed: {error}\n"
        assert output.read_bytes() == zero.read_bytes()
        observations.append({"case": name, "source_hex": content.hex(), "compiler_execution": compiled,
                             "program_execution_error": commands[-1]["stderr"], "previous_output_preserved": True})

    # The original combined long-name source now fits the unchanged arena.
    content = source(f"struct {long_type}{{{long_field}a:Field,{long_field}b:Field}}", f"let p={long_type}{{{long_field}b:9,{long_field}a:7}} nox_noun_atom(p.{long_field}a*10+p.{long_field}b)")
    directory, job = package("record-long-arena", content, {"arena_nodes": 786432})
    program = directory / "program.dag"
    compiled = execute(job, program)
    assert compiled["execution"]["compiler_job"]["status"] == "success"
    artifact = decode(program)
    assert artifact == record(0x41525431, 0, 0, 0, artifact[1][1][1][1][0])
    output = directory / "output.dag"
    executed = run(["run-artifact", program, "--input", input_file, "-o", output])
    assert decode(output) == 79
    assert executed["execution"]["program_particle"] == compiled["published_particle"]
    observations.append({"case": "record-long-arena", "source_hex": content.hex(), "expected": 79,
                         "compiler_execution": compiled, "program_execution": executed,
                         "program_bytes": len(program.read_bytes()), "complete_output_checked": True})
