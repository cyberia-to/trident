"""Check assertion failure and continuing values through installed warriors."""
import argparse
import hashlib
import json
from pathlib import Path
import runpy
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--joy", type=Path, required=True)
    parser.add_argument("--trisha", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[2]
    joy, trisha = args.joy.resolve(), args.trisha.resolve()
    decode = runpy.run_path(str(Path(__file__).with_name("run-native-compiler.py")))["decode"]
    commands, observations = [], []

    def run(binary, arguments, expected=0):
        command = [str(binary), *map(str, arguments)]
        result = subprocess.run(command, capture_output=True, text=True, timeout=60)
        commands.append({"command": command, "exit_code": result.returncode,
                         "stdout": result.stdout, "stderr": result.stderr})
        assert result.returncode == expected, commands[-1]
        return result

    with tempfile.TemporaryDirectory(prefix="trident-halting-") as temporary:
        root = Path(temporary)
        vectors = json.loads((repo.parent / "joy/cli/tests/compiler_vectors.json").read_text())
        zero, fourteen = root / "zero.dag", root / "fourteen.dag"
        zero.write_bytes(bytes.fromhex(vectors["files"]["zero"]))
        fourteen.write_bytes(bytes.fromhex(vectors["files"]["fourteen"]))
        library = "module failure\nuse vm.core.assert\npub fn choose(n:Field)->Field {if n==0 {if true {assert.is_true(false)}} else {7}}\n"
        (root / "failure.tri").write_text(library)
        native, stack = root / "native.tri", root / "stack.tri"
        native.write_text("program native\nuse failure\nfn main(input:Noun)->Noun {nox_noun_atom(failure.choose(nox_noun_as_field(input)))}\n")
        stack.write_text("program stack\nuse failure\nfn main(n:Field) {let sentinel=97 pub_write(failure.choose(n)) pub_write(sentinel)}\n")
        for profile in ["debug", "release"]:
            program, output = root / f"{profile}.dag", root / f"{profile}-output.dag"
            run(joy, ["build", native, "--emit", "artifact", "--profile", profile, "-o", program])
            run(joy, ["run-artifact", program, "--input", fourteen, "-o", output])
            assert decode(output) == 7
            previous = output.read_bytes()
            run(joy, ["run-artifact", program, "--input", zero, "-o", output, "--force"], 1)
            assert output.read_bytes() == previous
            observations.append({"owner": "joy", "profile": profile, "expected": 7,
                                 "assertion_failure_preserved_output": True,
                                 "program_particle": program.read_bytes()[8:40].hex(),
                                 "output_sha256": hashlib.sha256(previous).hexdigest()})
            for n, expected in [(14, [7, 97]), (0, None)]:
                result = run(trisha, ["run", stack, "--profile", profile, "--input-values", n],
                             1 if expected is None else 0)
                words = [int(line) for line in result.stdout.splitlines()]
                assert words == ([] if expected is None else expected)
                observations.append({"owner": "trisha", "profile": profile, "input": n,
                                     "output": words, "assertion_failure": expected is None})
    receipt = {"schema": "trident/halting-cli/v1", "kind": "local-development",
               "source_revisions": {name: subprocess.check_output(
                   ["git", "-C", str(repo.parent / name), "rev-parse", "HEAD"], text=True).strip()
                   for name in ["trident", "trisha", "joy", "nox"]},
               "commands": commands, "observations": observations}
    args.output.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({"commands": len(commands), "observations": len(observations), "receipt": str(args.output)}))


if __name__ == "__main__":
    main()
