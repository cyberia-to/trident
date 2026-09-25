"""Execute dotted paths and reject incomplete paths before either warrior publishes."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    for name in ["trident", "joy", "trisha", "output"]:
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[2]
    binaries = {name: getattr(args, name).resolve() for name in ["trident", "joy", "trisha"]}
    env = os.environ.copy()
    env["PATH"] = str(binaries["trident"].parent) + os.pathsep + env["PATH"]
    spec = importlib.util.spec_from_file_location("native_acceptance", Path(__file__).with_name("run-native-compiler.py"))
    native = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(native)
    commands, observations = [], []
    sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()

    def run(owner, arguments, expected=0):
        command = [str(binaries[owner]), *map(str, arguments)]
        result = subprocess.run(command, cwd=repo, env=env, capture_output=True, text=True)
        row = {"command": command, "exit_code": result.returncode,
               "stdout": result.stdout, "stderr": result.stderr}
        commands.append(row)
        assert result.returncode == expected, row
        return result

    with tempfile.TemporaryDirectory(prefix="trident-strict-paths-") as temporary:
        root = Path(temporary)
        (root / "a").mkdir()
        declarations = "pub const X:Field=7 pub struct Box{pub x:Field} pub fn seven()->Field{7} pub fn read(b:Box)->Field{b.x}"
        modules = {"a.tri": "module a " + declarations,
                   "a/value.tri": "module a // owner\n . value " + declarations}
        for path, source in modules.items():
            (root / path).write_text(source)
        vectors = json.loads((repo.parent / "joy/cli/tests/compiler_vectors.json").read_text())
        zero = root / "zero.dag"
        zero.write_bytes(bytes.fromhex(vectors["files"]["zero"]))
        protected = {}
        for owner in ["joy", "trisha"]:
            body = "let b:a // type\n . value . Box=a // constructor\n . value . Box{x:7} let got=a // callee\n . value . read(b)"
            expression = "got+a . value . X"
            entry = (f"fn main(input:Noun)->Noun{{{body} nox_noun_atom({expression})}}" if owner == "joy"
                     else f"fn main(){{{body} pub_write({expression})}}")
            source = "program entry use a // import\n . value " + entry
            path = root / f"{owner}.tri"
            path.write_text(source)
            output = root / ("program.dag" if owner == "joy" else "program.tasm")
            build = ["build", path, "-o", output]
            if owner == "joy":
                build += ["--emit", "artifact", "--artifact-profile", "raw"]
            run(owner, build)
            if owner == "joy":
                result = root / "result.dag"
                run(owner, ["run-artifact", output, "--input", zero, "-o", result])
                actual = native.decode(result)
            else:
                actual = int(run(owner, ["run", "--tasm", output]).stdout.strip())
            assert actual == 14, (owner, actual)
            protected[owner] = output.read_bytes()
            observations.append({"case": "valid-dotted-paths", "owner": owner,
                                 "source": source, "actual": actual, "expected": 14,
                                 "program_sha256": sha(output)})
        cases = [
            ("import-eof", "program entry use a."),
            ("import-before-function", "program entry use a. fn main()->Field{7}"),
            ("import-before-public", "program entry use a. pub fn main()->Field{7}"),
            ("import-before-use", "program entry use a. use a fn main()->Field{7}"),
            ("field", "program entry use a fn main()->Field{a.X.}"),
            ("call", "program entry use a fn main()->Field{a.seven.()}"),
            ("local-type", "program entry use a fn main()->Field{let b:a.Box.=a.Box{x:7} b.x}"),
            ("parameter-type", "program entry use a fn bad(b:a.Box.)->Field{b.x} fn main()->Field{7}"),
            ("return-type", "program entry use a fn bad()->a.Box.{a.Box{x:7}} fn main()->Field{7}"),
            ("constructor", "program entry use a fn main()->Field{let b=a.Box.{x:7} b.x}"),
        ]
        for name, source in cases:
            path = root / f"{name}.tri"
            path.write_text(source)
            checked = run("trident", ["check", path], expected=1)
            assert "expected identifier" in checked.stderr, checked.stderr
            for owner in ["joy", "trisha"]:
                output = root / ("protected.dag" if owner == "joy" else "protected.tasm")
                output.write_bytes(protected[owner])
                build = ["build", path, "-o", output]
                if owner == "joy":
                    build += ["--emit", "artifact", "--artifact-profile", "raw", "--force"]
                failure = run(owner, build, expected=1)
                assert "expected identifier" in failure.stderr, failure.stderr
                assert output.read_bytes() == protected[owner]
            observations.append({"case": name, "source": source, "previous_outputs_preserved": True})
        compiler = root / "compiler.dag"
        run("joy", ["build", repo / "compiler/nox/main.tri", "--emit", "artifact",
                    "--artifact-profile", "compiler-job", "-o", compiler])
        baseline = json.loads((repo / "audit/self-hosting/guest-package-cli.json").read_text())
        assert sha(compiler) == baseline["compiler_sha256"]
        receipt = {"schema": "trident/strict-module-paths-cli/v1", "kind": "local-development",
                   "binary_sha256": {name: sha(path) for name, path in binaries.items()},
                   "modules": modules, "commands": commands, "observations": observations,
                   "compiler_sha256": sha(compiler), "compiler_particle": compiler.read_bytes()[8:40].hex(),
                   "c1_baseline": "audit/self-hosting/guest-package-cli.json", "entire_c1_artifact_unchanged": True}
    args.output.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({"commands": len(commands), "observations": len(observations), "c1_identical": True}))


if __name__ == "__main__":
    main()
