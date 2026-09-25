"""Installed final callable ownership acceptance on both nox and Triton, with pinned inputs."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--joy", type=Path, required=True)
    parser.add_argument("--trident", type=Path, required=True)
    parser.add_argument("--trisha", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[2]
    binaries = {"joy": args.joy.resolve(), "trisha": args.trisha.resolve(), "trident": args.trident.resolve()}
    spec = importlib.util.spec_from_file_location("native_acceptance", Path(__file__).with_name("run-native-compiler.py"))
    native = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(native)
    commands, observations = [], []
    sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()

    def run(owner, arguments, expected=0):
        command = [str(binaries[owner]), *map(str, arguments)]
        result = subprocess.run(command, cwd=repo, capture_output=True, text=True)
        row = {"command": command, "exit_code": result.returncode,
               "stdout": result.stdout, "stderr": result.stderr}
        commands.append(row)
        assert result.returncode == expected, row
        return result

    cases = []
    for name, definitions, expression, expected in [
        ("final-body", "pub fn f()->Field{3} pub fn f()->Field{7}", "values.f()", 7),
        ("final-abi", "pub fn f()->Field{3} pub fn f(x:Field,y:Field)->Field{x+y}", "values.f(7,9)", 16),
        ("private-public", "fn f()->Field{3} pub fn f()->Field{7}", "values.f()", 7),
        ("ordinary-generic", "pub fn f(x:Field)->Field{x} pub fn f<N>(x:[Field;N])->Field{x[1]}", "values.f([7,9])", 9),
        ("generic-ordinary", "pub fn f<N>(x:[Field;N])->Field{x[0]} pub fn f(x:Field)->Field{x}", "values.f(7)", 7),
        ("final-generic-body", "pub fn f<N>(x:[Field;N])->Field{x[0]} pub fn f<M>(a:[Field;M])->Field{a[1]}", "values.f([7,9])", 9),
        ("replaced-call-sites", "fn first<N>(x:[Field;N])->Field{x[0]} fn old()->Field{first([1])} fn old()->Field{7} pub fn f()->Field{first([7,9])}", "values.f()", 7),
        ("mixed-generic-sites", "fn first<N>(x:[Field;N])->U32{as_u32(x[0])} pub fn f()->Field{if first([3]) < first<2>([7,9]){return 1} 0}", "values.f()", 1),
    ]:
        cases.append((name, {"values.tri": "module values\n"+definitions}, "use values", "", expression, expected, "debug"))
    for profile, expected in [("debug", 7), ("release", 9)]:
        cases.append(("final-cfg-"+profile, {"values.tri": "module values\npub fn f()->Field{7}\n#[cfg(release)] pub fn f()->Field{9}\n#[cfg(absent)] fn f()->Bool{true}"}, "use values", "", "values.f()", expected, profile))
    for name, definitions, statements, expression in [
        ("intrinsic-to-ordinary", "#[intrinsic(assert)] pub fn stop(c:Bool) pub fn stop(x:Field)->Field{x}", "", "failure.stop(7)"),
        ("ordinary-to-intrinsic", "pub fn stop(x:Field)->Field{x} #[intrinsic(assert)] pub fn stop(c:Bool)", "failure.stop(true)", "7"),
        ("replaced-recursion", "fn stop(c:Bool){stop(c)} #[intrinsic(assert)] pub fn stop(c:Bool)", "failure.stop(true)", "7"),
    ]:
        cases.append((name, {"ext/failure.tri": "module ext.failure\n"+definitions}, "use ext.failure", statements, expression, 7, "debug"))
    failures = []
    for name, definitions, use in [
        ("final-private", "pub fn f()->Field{7} fn f()->Field{9}", "fn call()->Field{values.f()}"),
        ("generic-private", "pub fn f<N>(x:[Field;N])->Field{x[0]} fn f<N>(x:[Field;N])->Field{x[0]}", "fn call()->Field{values.f([7])}"),
        ("ordinary-to-private-generic", "pub fn f(x:Field)->Field{x} fn f<N>(x:[Field;N])->Field{x[0]}", "fn call()->Field{values.f([7])}"),
        ("generic-to-private-ordinary", "pub fn f<N>(x:[Field;N])->Field{x[0]} fn f(x:Field)->Field{x}", "fn call()->Field{values.f(7)}"),
        ("replaced-invalid-body", "pub fn f()->Field{missing()} pub fn f()->Field{7}", ""),
        ("replaced-impure-body", "#[pure] pub fn f()->Field{pub_read()} pub fn f()->Field{7}", ""),
    ]:
        failures.append((name, {"values.tri": "module values\n"+definitions}, "use values\n"+use))
    with tempfile.TemporaryDirectory(prefix="trident-callable-exports-") as temporary:
        root = Path(temporary)
        zero = root / "zero.dag"
        vectors = json.loads((repo.parent / "joy/cli/tests/compiler_vectors.json").read_text())
        zero.write_bytes(bytes.fromhex(vectors["files"]["zero"]))
        protected = {}
        for name, modules, declarations, statements, expression, expected, profile in cases:
            directory = root / name
            directory.mkdir()
            for filename, content in modules.items():
                (directory / filename).parent.mkdir(parents=True, exist_ok=True)
                (directory / filename).write_text(content)
            sources = {}
            for owner in ["joy", "trisha"]:
                entry = (f"fn main(input:Noun)->Noun{{{statements} nox_noun_atom({expression})}}" if owner == "joy"
                         else f"fn main(){{{statements} pub_write({expression})}}")
                source = f"program callables\n{declarations}\n{entry}\n"
                path = directory / f"{owner}.tri"
                path.write_text(source)
                program = directory / ("program.dag" if owner == "joy" else "program.tasm")
                build = ["build", path, "--profile", profile, "-o", program]
                if owner == "joy":
                    build += ["--emit", "artifact", "--artifact-profile", "raw"]
                run(owner, build)
                if owner == "joy":
                    output = directory / "output.dag"
                    run(owner, ["run-artifact", program, "--input", zero, "-o", output])
                    actual = native.decode(output)
                else:
                    result = run(owner, ["run", "--tasm", program])
                    actual = int(result.stdout.strip())
                assert actual == expected, (name, owner, actual, expected)
                protected[owner] = program.read_bytes()
                sources[owner] = {"source": source, "program_sha256": sha(program), "executed": actual}
            observations.append({"case": name, "modules": modules, "profile": profile, "expected": expected, "owners": sources})
        for name, modules, declarations in failures:
            directory = root / name
            directory.mkdir()
            for filename, content in modules.items():
                (directory / filename).parent.mkdir(parents=True, exist_ok=True)
                (directory / filename).write_text(content)
            sources = {}
            for owner in ["joy", "trisha"]:
                entry = "fn main(input:Noun)->Noun{input}" if owner == "joy" else "fn main(){pub_write(7)}"
                source = f"program callables\n{declarations}\n{entry}\n"
                path = directory / f"{owner}.tri"
                path.write_text(source)
                output = directory / ("protected.dag" if owner == "joy" else "protected.tasm")
                output.write_bytes(protected[owner])
                build = ["build", path, "-o", output]
                if owner == "joy":
                    build += ["--emit", "artifact", "--artifact-profile", "raw", "--force"]
                run(owner, build, expected=1)
                assert output.read_bytes() == protected[owner], (name, owner)
                sources[owner] = {"source": source, "previous_program_preserved": True}
            observations.append({"case": name, "modules": modules, "owners": sources})
        for name, definitions, expected_code, expected_text in [
            ("withdrawn-test", "#[test] fn same(){assert(false)} fn same(){assert(true)}", 0, "No active #[test] functions"),
            ("added-test", "fn same(){assert(false)} #[test] fn same(){assert(true)}", 0, "1 passed; 0 failed; 0 skipped"),
            ("replaced-test-pass", "#[test] fn same(){assert(false)} #[test] fn same(){assert(true)}", 0, "1 passed; 0 failed; 0 skipped"),
            ("replaced-test-fail", "#[test] fn same(){assert(true)} #[test] fn same(){assert(false)}", 1, "0 passed; 1 failed; 0 skipped"),
            ("inactive-test", "#[test] fn same(){assert(true)} #[cfg(release)] #[test] fn same(){assert(false)}", 0, "1 passed; 0 failed; 1 skipped"),
        ]:
            path = root / (name + ".tri")
            source = "module cases\n" + definitions
            path.write_text(source)
            results = {}
            for owner in ["trident", "trisha"]:
                result = run(owner, ["test", path], expected_code)
                text = result.stdout + result.stderr
                summary = "0 passed; 0 failed; 0 skipped" if name == "withdrawn-test" and owner == "trisha" else expected_text
                assert summary in text, (name, owner, text)
                results[owner] = {"exit_code": result.returncode, "expected_summary": summary}
            observations.append({"case": name, "source": source, "owners": results})
        compiler = root / "compiler.dag"
        run("joy", ["build", repo / "compiler/nox/main.tri", "--emit", "artifact", "--artifact-profile", "compiler-job", "-o", compiler])
        baseline = json.loads((repo / "audit/self-hosting/native-attributes-cli.json").read_text())
        assert sha(compiler) == baseline["compiler_sha256"]
        receipt = {"schema": "trident/callable-exports-cli/v1", "kind": "local-development",
                   "binary_sha256": {name: sha(path) for name, path in binaries.items()},
                   "commands": commands, "observations": observations,
                   "compiler_sha256": sha(compiler), "compiler_particle": compiler.read_bytes()[8:40].hex(),
                   "entire_c1_artifact_unchanged": True, "c1_baseline": "audit/self-hosting/native-attributes-cli.json"}
    args.output.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({"commands": len(commands), "observations": len(observations), "c1_identical": True}))


if __name__ == "__main__":
    main()
