"""Installed explicit import scope acceptance on both nox and Triton, with pinned inputs."""
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

    vault = {
        "bank/vault.tri": "module bank.vault pub const VALUE:Field=9 pub struct Seal{pub words:[Field;2],secret:Field} pub fn make()->Seal{Seal{words:[7,9],secret:11}} pub fn read(s:Seal)->Field{s.words[1]} pub fn first<N>(x:[Field;N])->Field{x[0]}",
        "facade.tri": "module facade use bank.vault pub fn make()->vault.Seal{vault.make()} pub fn read(s:vault.Seal)->Field{vault.read(s)} pub fn first<N>(x:[Field;N])->Field{vault.first(x)} pub struct Wrapped{pub inner:vault.Seal} pub fn wrap()->Wrapped{Wrapped{inner:vault.make()}}",
    }
    common = {
        "a/common.tri": "module a.common pub struct X{pub words:[Field;2]} pub const A:Field=3 pub fn first()->Field{5} pub fn shared()->Field{7}",
        "b/common.tri": "module b.common pub struct Y{pub words:[Field;3]} pub const B:Field=11 pub fn second()->Field{13} pub fn shared()->Field{17}",
    }
    collision = {
        "b.tri": "module b pub struct Box{pub words:[Field;2]} pub fn make()->Box{Box{words:[7,9]}}",
        "facade.tri": "module facade use b pub fn make()->b.Box{b.make()}",
        "other/b.tri": "module other.b pub struct Box{pub words:[Field;3]} pub fn unrelated()->Field{11}",
    }
    cases = []
    def positive(name, modules, uses, statements, expression, expected):
        cases.append((name, modules, uses, statements, expression, expected, "debug"))
    positive("direct-full-short", vault, "use bank.vault", "", "bank.vault.VALUE+vault.first([7,9])", 16)
    positive("opaque-return", vault, "use facade", "let p=facade.make()", "p.words[1]+facade.read(p)+facade.first([3,5])", 21)
    positive("opaque-nested-return", vault, "use facade", "let p=facade.wrap()", "p.inner.words[1]", 9)
    positive("canonical-return-collision", collision, "use facade use other.b", "let p=facade.make()", "p.words[1]", 9)
    positive("canonical-return-store", collision, "use facade use other.b", "let mut p=facade.make() p.words[1]=13", "p.words[1]", 13)
    for name, uses, expected in [("ab", "use a.common use b.common",58), ("ba","use b.common use a.common",48), ("aba","use a.common use b.common use a.common",48)]:
        positive("retained-symbols-"+name, common, uses, "let p:common.X=common.X{words:[7,9]}", "p.words[1]+common.A+common.B+common.first()+common.second()+common.shared()", expected)
    positive("canonical-legacy-once", {}, "use std.convert use vm.core.convert", "", "std.convert.as_field(convert.as_u32(vm.core.convert.as_field(as_u32(7))))", 7)
    for name, definitions in [
        ("final-public-struct", "struct S{pub x:Field} pub struct S{pub x:Field}"),
        ("inactive-private-struct", "pub struct S{pub x:Field} #[cfg(absent)] struct S{pub x:Field}"),
    ]:
        positive(name,{"value.tri":"module value "+definitions},"use value","let s=value.S{x:7}","s.x",7)
    positive("constant-owner", {
        "a/same.tri":"module a.same pub const VALUE:Field=7 pub const SIZE:U32=2",
        "helper.tri":"module helper use a.same pub const VALUE:Field=same.VALUE const K:U32=same.SIZE pub fn f<N>(a:[Field;N+K])->Field{a[2]+VALUE}",
        "b/same.tri":"module b.same pub const VALUE:Field=13 const SIZE:U32=99",
    }, "use helper use b.same", "", "helper.f<1>([3,5,11])+helper.VALUE+same.VALUE", 38)
    failures = []
    for index, expression in enumerate([
        "let p=bank.vault.make() bank.vault.read(p)", "let p=vault.make() vault.read(p)",
        "bank.vault.VALUE", "vault.VALUE", "let p:bank.vault.Seal=facade.make() 7", "let p:vault.Seal=facade.make() 7",
        "bank.vault.first([7,9])", "vault.first([7,9])", "let p=facade.make() p.secret",
    ]):
        failures.append((f"hidden-{index}", vault, "use facade fn bad()->Field{"+expression+"}"))
    for uses in ["use a use z", "use z use a"]:
        failures.append(("sibling-"+str(len(failures)), {"a.tri":"module a pub fn value()->Field{7}","z.tri":"module z pub fn leak()->Field{a.value()}"},uses))
    for name, module in [("owner-mismatch","module other"),("imported-program","program value fn main(){}"),("private-final-struct","module value pub struct S{pub x:Field} struct S{pub x:Field}")]:
        tail = " fn bad()->Field{let s=value.S{x:7} s.x}" if name == "private-final-struct" else ""
        failures.append((name,{"value.tri":module},"use value"+tail))
    failures.append(("transitive-intrinsic", {"ext/leaf.tri":"module ext.leaf #[intrinsic(assert)] pub fn stop(c:Bool)","facade.tri":"module facade use ext.leaf pub fn value()->Field{leaf.stop(true) 7}"},"use facade fn bad(){ext.leaf.stop(true)}"))
    with tempfile.TemporaryDirectory(prefix="trident-explicit-imports-") as temporary:
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
                source = f"program imports\n{declarations}\n{entry}\n"
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
                source = f"program imports\n{declarations}\n{entry}\n"
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
        baseline = json.loads((repo / "audit/self-hosting/native-attributes-cli.json").read_text())
        assert sha(compiler) == baseline["compiler_sha256"]
        receipt = {"schema": "trident/explicit-imports-cli/v1", "kind": "local-development",
                   "binary_sha256": {name: sha(path) for name, path in binaries.items()},
                   "commands": commands, "observations": observations,
                   "compiler_sha256": sha(compiler), "compiler_particle": compiler.read_bytes()[8:40].hex(),
                   "entire_c1_artifact_unchanged": True, "c1_baseline": "audit/self-hosting/native-attributes-cli.json"}
    args.output.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({"commands": len(commands), "observations": len(observations), "c1_identical": True}))


if __name__ == "__main__":
    main()
