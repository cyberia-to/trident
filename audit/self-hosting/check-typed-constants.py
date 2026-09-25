"""Installed seed constant acceptance on both nox and Triton, with pinned inputs."""
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
    parser.add_argument("--trisha", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[2]
    binaries = {"joy": args.joy.resolve(), "trisha": args.trisha.resolve()}
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

    # name, modules, declarations, statements, scalar expression, expected, profile
    cases = [
        ("local-forward", {}, "const X:Field=Y\nconst Y:Field=7", "", "X", 7, "debug"),
        ("typed-u32", {}, "const X:U32=Y\nconst Y:U32=4294967295", "", "as_field(X)", 4294967295, "debug"),
        ("field-normalization", {}, "const X:Field=Y\nconst Y:Field=18446744069414584322", "", "X", 1, "debug"),
        ("final-signature-size", {}, "const N:U32=1\nfn f(a:[Field;N])->Field{a[1]}\nconst N:U32=2", "", "f([3,7])", 7, "debug"),
        ("final-local-type", {}, "pub const X:U32=7\nconst X:Field=4294967296", "", "X", 4294967296, "debug"),
        ("import-alias", {"values.tri": "module values\npub const X:U32=7\npub const Y:U32=X"}, "use values\nconst Z:U32=values.Y", "", "as_field(Z)", 7, "debug"),
        ("owner-layouts", {
            "sizes.tri": "module sizes\npub const BASE:U32=2\npub const N:U32=BASE",
            "shapes.tri": "module shapes\nuse sizes\nconst K:U32=sizes.N\npub struct Pair{pub a:[Field;K],pub b:Field}\npub fn make()->Pair{Pair{a:[3,7],b:11}}\npub fn out()->[Field;K]{[5,13]}\npub fn f<N>(a:[Field;N+K])->Field{a[2]}",
        }, "use shapes\nconst K:U32=99", "let p=shapes.make() let a=shapes.out()", "p.a[1]+p.b+a[1]+shapes.f<1>([17,19,23])", 54, "debug"),
        ("lexical-alias-owners", {
            "early.tri": "module std.same\npub const VALUE:Field=7\npub const SIZE:U32=2",
            "helper.tri": "module helper\nuse early\npub const VALUE:Field=same.VALUE\nconst K:U32=same.SIZE\npub fn f<N>(a:[Field;N+K])->Field{a[2]+VALUE}",
            "late.tri": "module os.same\npub const VALUE:Field=13\nconst SIZE:U32=99",
        }, "use helper\nuse late\nconst K:U32=99\nconst V:Field=helper.VALUE", "", "helper.f<1>([3,5,11])+V+same.VALUE", 38, "debug"),
    ]
    for profile, expected in [("debug", 7), ("release", 9)]:
        cases.append((f"cfg-{profile}", {}, "#[cfg(debug)] const X:Field=7\n#[cfg(release)] const X:Field=9\n#[cfg(absent)] const X:Noun=missing()", "", "X", expected, profile))
    failures = [
        ("unused-unknown", {}, "const X:Field=missing"),
        ("unused-call", {}, "const X:Field=missing()"),
        ("unused-expression", {}, "const X:Field=1+2"),
        ("cycle", {}, "const X:Field=Y\nconst Y:Field=X"),
        ("wrong-alias-type", {}, "const X:Field=7\nconst Y:U32=X"),
        ("raw-u32-overflow", {}, "const X:U32=18446744069414584322"),
        ("replaced-invalid", {}, "const X:Field=missing\nconst X:Field=7"),
        ("replacement-invalid", {}, "const X:Field=7\nconst X:Field=missing"),
        ("private-final", {"values.tri": "module values\npub const X:U32=7\nconst X:Field=4294967296"}, "use values\nconst Y:U32=values.X"),
        ("raw-dimension", {}, "const X:Field=18446744069414584322\nconst N:Field=X\nfn f(a:[Field;N])->Field{a[0]}\nfn g()->Field{f([7])}"),
    ]
    with tempfile.TemporaryDirectory(prefix="trident-typed-constants-") as temporary:
        root = Path(temporary)
        zero = root / "zero.dag"
        vectors = json.loads((repo.parent / "joy/cli/tests/compiler_vectors.json").read_text())
        zero.write_bytes(bytes.fromhex(vectors["files"]["zero"]))
        protected = {}
        for name, modules, declarations, statements, expression, expected, profile in cases:
            directory = root / name
            directory.mkdir()
            for filename, content in modules.items():
                (directory / filename).write_text(content)
            sources = {}
            for owner in binaries:
                entry = (f"fn main(input:Noun)->Noun{{{statements} nox_noun_atom({expression})}}" if owner == "joy"
                         else f"fn main(){{{statements} pub_write({expression})}}")
                source = f"program constants\n{declarations}\n{entry}\n"
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
                (directory / filename).write_text(content)
            sources = {}
            for owner in binaries:
                entry = "fn main(input:Noun)->Noun{input}" if owner == "joy" else "fn main(){pub_write(7)}"
                source = f"program constants\n{declarations}\n{entry}\n"
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
        compiler = root / "compiler.dag"
        run("joy", ["build", repo / "compiler/nox/main.tri", "--emit", "artifact", "--artifact-profile", "compiler-job", "-o", compiler])
        baseline = json.loads((repo / "audit/self-hosting/native-arrays-cli.json").read_text())
        assert sha(compiler) == baseline["compiler_sha256"]
        receipt = {"schema": "trident/typed-constants-cli/v1", "kind": "local-development",
                   "binary_sha256": {name: sha(path) for name, path in binaries.items()},
                   "commands": commands, "observations": observations,
                   "compiler_sha256": sha(compiler), "compiler_particle": compiler.read_bytes()[8:40].hex(),
                   "entire_c1_artifact_unchanged": True, "c1_baseline": "audit/self-hosting/native-arrays-cli.json"}
    args.output.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({"commands": len(commands), "observations": len(observations), "c1_identical": True}))


if __name__ == "__main__":
    main()
