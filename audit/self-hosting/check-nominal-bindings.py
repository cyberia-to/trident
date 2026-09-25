"""Installed seed nominal layout acceptance; no guest compiler capability claim.

Joy's replacement build requires --force. Trisha's build always replaces its
output after successful compilation and has no --force flag. Both must leave an
existing valid program byte-for-byte intact when checking rejects the source.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require(condition, detail):
    if not condition:
        raise RuntimeError(str(detail))


def noun(values):
    result = 0
    for value in reversed(values):
        result = (value, result)
    return result


def entry(owner, uses, statements, expressions):
    if owner == "joy":
        output = "nox_noun_atom(0)"
        for expression in reversed(expressions):
            output = f"nox_noun_pair(nox_noun_atom({expression}),{output})"
        body = f"fn main(input:Noun)->Noun{{{statements} {output}}}"
    else:
        output = " ".join(f"pub_write({expression})" for expression in expressions)
        body = f"fn main(){{{statements} {output}}}"
    return f"program nominal_bindings\n{uses}\n{body}\n"


def cases():
    positives = [
        {
            "case": "same-layout-final-public",
            "modules": {"value.tri": "module value struct Pair{pub x:Field,pub y:Field} "
                        "pub fn make()->Pair{Pair{x:7,y:9}} "
                        "pub struct Pair{pub x:Field,pub y:Field}"},
            "uses": "use value", "statements": "let p:value.Pair=value.make()",
            "expressions": ["p.x", "p.y"], "expected": [7, 9],
        },
        {
            "case": "same-layout-final-private-opaque",
            "modules": {"value.tri": "module value pub struct S{pub x:Field,pub y:Field,secret:Field} "
                        "pub fn make()->S{S{x:7,y:9,secret:11}} "
                        "struct S{pub x:Field,pub y:Field,secret:Field} "
                        "pub fn secret(s:S)->Field{s.secret}"},
            "uses": "use value", "statements": "let s=value.make()",
            "expressions": ["s.x", "s.y", "value.secret(s)"], "expected": [7, 9, 11],
        },
        {
            "case": "equivalent-import-aliases-and-extents",
            "modules": {
                "bank/types.tri": "module bank.types pub struct Item{pub value:Field}",
                "value.tri": "module value use bank.types const N:U32=2 "
                "struct S{pub item:bank.types.Item,pub words:[Field;N+1]} "
                "pub fn make()->S{S{item:types.Item{value:7},words:[3,5,11]}} "
                "pub struct S{pub item:types.Item,pub words:[Field;3]}",
            },
            "uses": "use value", "statements": "let s=value.make()",
            "expressions": ["s.item.value", "s.words[0]", "s.words[1]", "s.words[2]"],
            "expected": [7, 3, 5, 11],
        },
    ]
    negatives = []
    for name, declarations, statements, expression, nominal in [
        ("stale-field-type-signature",
         "pub struct S{pub x:Field} pub fn make()->S{S{x:true}} pub struct S{pub x:Bool}",
         "let s=value.make()", "s.x+1", "S"),
        ("stale-private-field-signature",
         "pub struct S{pub x:Field} pub fn make()->S{S{x:7}} pub struct S{x:Field}",
         "let s=value.make()", "s.x", "S"),
        ("changed-field-order",
         "struct S{x:Field,y:Field} struct S{y:Field,x:Field}", "", "7", "S"),
        ("changed-field-name",
         "struct S{x:Field} struct S{y:Field}", "", "7", "S"),
        ("nested-layout-cycle",
         "struct A{x:Field} struct B{a:A} struct A{b:B}", "", "7", "A"),
    ]:
        negatives.append({
            "case": name, "modules": {"value.tri": "module value " + declarations},
            "uses": "use value", "statements": statements, "expressions": [expression],
            "diagnostic": f"incompatible redeclaration of struct 'value.{nominal}'",
        })
    return positives, negatives


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--joy", type=Path, required=True)
    parser.add_argument("--trident", type=Path, required=True)
    parser.add_argument("--trisha", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[2]
    binaries = {owner: getattr(args, owner).resolve() for owner in ("joy", "trident", "trisha")}
    decoder = Path(__file__).with_name("run-native-compiler.py")
    receipt = {
        "schema": "trident/nominal-bindings-cli/v1", "kind": "local-development",
        "status": "running", "commands": [], "observations": [],
        "binary_paths": {owner: str(path) for owner, path in binaries.items()},
        "binary_sha256_start": {},
        "replacement_mode": {"joy": "--force", "trisha": "build replaces after successful compilation; no --force flag"},
        "scope": "Installed seed compilation and execution on nox/Triton; guest imports and self-build are not exercised.",
    }
    # Invalidate any previous passing receipt before fallible setup or execution.
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(receipt, indent=2) + "\n")

    def run(command, cwd=repo, expected=0):
        command = list(map(str, command))
        result = subprocess.run(command, cwd=cwd, capture_output=True, text=True, check=False)
        row = {"command": command, "cwd": str(cwd), "expected_exit_code": expected,
               "exit_code": result.returncode, "stdout": result.stdout, "stderr": result.stderr}
        receipt["commands"].append(row)
        require(result.returncode == expected, row)
        return result

    def build(owner, path, output, replace=False):
        command = [binaries[owner], "build", path, "--profile", "debug", "-o", output]
        if owner == "joy":
            command += ["--emit", "artifact", "--artifact-profile", "raw"]
            if replace:
                command += ["--force"]
        return command

    def execute(owner, program, directory, expected, zero):
        if owner == "joy":
            output = directory / "output.dag"
            run([binaries[owner], "run-artifact", program, "--input", zero, "-o", output])
            actual = native.decode(output)
            require(actual == noun(expected), (owner, actual, noun(expected)))
            return {"executed": actual, "expected_noun": noun(expected),
                    "output_sha256": sha(output), "output_hex": output.read_bytes().hex()}
        result = run([binaries[owner], "run", "--tasm", program])
        actual = [int(line) for line in result.stdout.splitlines()]
        require(actual == expected, (owner, actual, expected))
        return {"executed": actual}

    try:
        for owner, path in binaries.items():
            receipt["binary_sha256_start"][owner] = sha(path)
        spec = importlib.util.spec_from_file_location("native_acceptance", decoder)
        native = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(native)
        receipt["source_revisions"] = {}
        for owner in ("trident", "joy", "trisha"):
            path = repo if owner == "trident" else repo.parent / owner
            receipt["source_revisions"][owner] = run(["git", "rev-parse", "HEAD"], cwd=path).stdout.strip()
            run([binaries[owner], "--version"])
        receipt["source_snapshots"] = {}
        for relative in (
            "src/typecheck/nominal_bindings.rs", "src/typecheck/file.rs",
            "src/typecheck/mod.rs", "src/ir/tree/lower/nox/modules.rs",
            "src/ir/tir/builder/types.rs", "reference/language.md",
            "audit/self-hosting/check-nominal-bindings.py", "audit/self-hosting/run-native-compiler.py",
        ):
            path = repo / relative
            receipt["source_snapshots"][relative] = {"sha256": sha(path), "source": path.read_text()}
        with tempfile.TemporaryDirectory(prefix="trident-nominal-bindings-") as temporary:
            root = Path(temporary)
            vectors = repo.parent / "joy/cli/tests/compiler_vectors.json"
            zero = root / "zero.dag"
            zero.write_bytes(bytes.fromhex(json.loads(vectors.read_text())["files"]["zero"]))
            receipt["input"] = {"vector_file": str(vectors), "vector_file_sha256": sha(vectors),
                                "zero_hex": zero.read_bytes().hex(), "zero_sha256": sha(zero)}
            protected = {}
            positives, negatives = cases()
            for accepted, corpus in ((True, positives), (False, negatives)):
                for case in corpus:
                    observation = {**case, "accepted": accepted, "profile": "debug", "owners": {}}
                    receipt["observations"].append(observation)
                    for owner in ("joy", "trisha"):
                        directory = root / case["case"] / owner
                        directory.mkdir(parents=True)
                        for filename, source in case["modules"].items():
                            path = directory / filename
                            path.parent.mkdir(parents=True, exist_ok=True)
                            path.write_text(source)
                        source = entry(owner, case["uses"], case["statements"], case["expressions"])
                        path = directory / "entry.tri"
                        path.write_text(source)
                        program = directory / ("program.dag" if owner == "joy" else "program.tasm")
                        row = {"source": source, "source_sha256": sha(path)}
                        observation["owners"][owner] = row
                        if accepted:
                            run(build(owner, path, program))
                            row.update(execute(owner, program, directory, case["expected"], zero))
                            row.update(program_sha256=sha(program), complete_output_checked=True)
                            protected[owner] = {"bytes": program.read_bytes(), "case": case["case"]}
                        else:
                            previous = protected[owner]
                            program.write_bytes(previous["bytes"])
                            row.update(protected_program_case=previous["case"], protected_sha256_before=sha(program))
                            result = run(build(owner, path, program, replace=True), expected=1)
                            require(case["diagnostic"] in result.stdout + result.stderr,
                                    (case["case"], owner, result.stdout, result.stderr))
                            require(program.read_bytes() == previous["bytes"], (case["case"], owner, "output changed"))
                            row.update(protected_sha256_after=sha(program), previous_program_preserved=True)
            receipt["status"] = "passed"
    except Exception as error:
        receipt["status"] = "failed"
        receipt["error"] = repr(error)
        raise
    finally:
        receipt["binary_sha256_end"] = {}
        for owner, path in binaries.items():
            try:
                receipt["binary_sha256_end"][owner] = sha(path)
            except Exception as error:
                receipt.setdefault("binary_hash_errors", {})[owner] = repr(error)
        receipt["binaries_unchanged"] = (
            len(receipt["binary_sha256_start"]) == len(binaries)
            and receipt["binary_sha256_start"] == receipt["binary_sha256_end"]
        )
        if not receipt["binaries_unchanged"]:
            receipt["status"] = "failed"
            receipt.setdefault("error", "installed binaries changed or became unavailable during acceptance")
        receipt["counts"] = {"commands": len(receipt["commands"]), "cases": len(receipt["observations"]),
                             "owner_observations": sum(len(case["owners"]) for case in receipt["observations"])}
        args.output.write_text(json.dumps(receipt, indent=2) + "\n")
    require(receipt["status"] == "passed", receipt.get("error"))
    print(json.dumps({"status": receipt["status"], **receipt["counts"]}))


if __name__ == "__main__":
    main()
