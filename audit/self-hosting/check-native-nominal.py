"""Installed Joy execution of the nominal descriptor API; source structs stay open."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile


def tree(values):
    if not values:
        return 0
    values = list(values) + [0] * ((1 << (len(values) - 1).bit_length()) - len(values))
    while len(values) > 1:
        values = list(zip(values[::2], values[1::2]))
    return values[0]


def sequence(values):
    return 1397051697, (len(values), tree(values))


def string(value):
    data = value.encode()
    return 1113150513, (len(data), tree([int.from_bytes(data[i:i + 4], "little") for i in range(0, len(data), 4)]))


def field(name, ty, public=False):
    return string(name), (ty, int(public))


def nominal(owner, name, fields, depth, nodes, flag=0):
    return 17, ((string(owner), (string(name), sequence(fields))), (depth, (nodes, flag)))


def built(ty):
    return 0, (0, ty)


def failed(code, index=0):
    return code, (index, 5)


def source(value):
    if isinstance(value, int):
        return f"nox_noun_atom({value})"
    return f"nox_noun_pair({source(value[0])},{source(value[1])})"


def cases():
    fields = [field("common_prefix_first", 0, True), field("common_prefix_second", 4)]
    ty = nominal("compiler.alpha", "Record", fields, 1, 3, 1)
    inspection = built(ty), (string("compiler.alpha"), (string("Record"), (sequence(fields), (1, (3, (1, 0))))))
    rows = [("complete-descriptor", 1, "compiler.alpha", "Record", fields, 3, 1, 0, inspection)]
    for owner, wanted, expected in [
        ("compiler.alpha", "common_prefix_second", (4, (1, (1, 1)))),
        ("compiler.alphb", "common_prefix_second", (4, (1, (1, 0)))),
        ("compiler.alphb", "common_prefix_first", (0, (0, (1, 1)))),
        ("compiler.alpha", "common_prefix_seconx", (5, (0, (0, 0)))),
    ]:
        rows.append((f"lookup-{owner}-{wanted}", 2, "compiler.alpha", "Record", fields, 3, 1, (string(owner), string(wanted)), expected))
    rows += [
        ("empty", 0, "m", "Empty", [], 1, 1, 0, built(nominal("m", "Empty", [], 1, 1))),
        ("empty-owner", 0, "", "S", [], 1, 1, 0, failed(5)),
        ("duplicate", 0, "m", "S", [field("x", 0), field("x", 3)], 4096, 64, 0, failed(5, 1)),
        ("invalid-child", 0, "m", "S", [field("x", 0), field("y", 5)], 4096, 64, 0, failed(5, 1)),
    ]
    wide = [field(f"f{i}", 0, i % 2 == 0) for i in range(32)]
    rows += [
        ("fields32", 0, "m", "Wide", wide, 33, 1, 0, built(nominal("m", "Wide", wide, 1, 33))),
        ("fields32-node-underflow", 0, "m", "Wide", wide, 32, 1, 0, failed(7, 31)),
        ("fields33", 0, "m", "Wide", wide + [field("f32", 0)], 4096, 64, 0, failed(7)),
        ("last-field32", 2, "m", "Wide", wide, 33, 1, (string("foreign"), string("f31")), (0, (31, (1, 0)))),
    ]
    current = 0
    for depth in range(1, 65):
        current = nominal("m", "S", [field("S", current)], depth, depth + 1)
    rows += [
        ("depth64", 3, "m", "S", [], 65, 64, (64, 0), built(current)),
        ("depth65", 3, "m", "S", [], 4096, 64, (65, 0), failed(7)),
    ]
    current, nodes = 0, 1
    for depth in range(1, 12):
        nodes = 1 + 2 * nodes
        current = nominal("m", "S", [field("S", current), field("m", current, True)], depth, nodes)
    rows += [
        ("shared4095", 3, "m", "S", [], 4095, 64, (11, 1), built(current)),
        ("shared-underflow", 3, "m", "S", [], 4094, 64, (11, 1), failed(7, 1)),
    ]
    for label, mode, owner, name, fields, nodes, depth, extra, expected in rows:
        value = mode, (string(owner), (string(name), (sequence(fields), ((nodes, depth), extra))))
        yield label, value, expected


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--joy", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    binary = args.joy.resolve()
    repo = Path(__file__).resolve().parents[2]
    sys.path.insert(0, str(repo / "audit/self-hosting"))
    spec = importlib.util.spec_from_file_location("native_acceptance", repo / "audit/self-hosting/run-native-compiler.py")
    reader = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(reader)
    commands, observations = [], []

    def run(arguments):
        cmd = [str(binary), *map(str, arguments)]
        result = subprocess.run(cmd, cwd=repo, capture_output=True, text=True)
        row = {"command": cmd, "exit_code": result.returncode, "stdout": result.stdout, "stderr": result.stderr}
        commands.append(row)
        assert result.returncode == 0, row
        return json.loads(result.stdout) if result.stdout.startswith("{") else None

    with tempfile.TemporaryDirectory(prefix="trident-nominal-") as directory:
        work = Path(directory)
        vectors = json.loads((repo.parent / "joy/cli/tests/compiler_vectors.json").read_text())
        zero = work / "zero.dag"
        zero.write_bytes(bytes.fromhex(vectors["files"]["zero"]))
        fixture = work / "nominal.dag"
        run(["build", "tests/fixtures/native_nominal.tri", "--emit", "artifact", "--artifact-profile", "raw", "-o", fixture])
        for label, value, expected in cases():
            generator = work / f"{label}.tri"
            generator.write_text(f"program sample fn main(input:Noun)->Noun{{{source(value)}}}\n")
            program, input_file, output = [work / f"{label}-{suffix}.dag" for suffix in ["generator", "input", "output"]]
            run(["build", generator, "--emit", "artifact", "--artifact-profile", "raw", "-o", program])
            run(["run-artifact", program, "--input", zero, "-o", input_file])
            assert reader.decode(input_file) == value
            execution = run(["run-artifact", fixture, "--input", input_file, "-o", output, "--budget", "50000000"])
            assert reader.decode(output) == expected, label
            observations.append({"case": label, "expected_shape_sha256": hashlib.sha256(json.dumps(expected, separators=(",", ":")).encode()).hexdigest(), "output_particle": output.read_bytes()[8:40].hex(), "execution": execution})
    report = {"schema": "trident/native-nominal-cli/v1", "kind": "local-development", "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "commands": commands, "observations": observations}
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(f"{len(commands)} commands; {len(observations)} complete output checks pass")


if __name__ == "__main__":
    main()
