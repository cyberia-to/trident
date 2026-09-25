"""Installed Joy execution of graph components; complete guest linking stays open."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile


def sequence(value, tag=1397051697):
    actual, (length, root) = value
    assert actual == tag
    leaves = length if tag == 1397051697 else (length + 3) // 4
    capacity = 1 << (max(leaves, 1) - 1).bit_length()

    def walk(node, size):
        if size == 1:
            return [node]
        return walk(node[0], size // 2) + walk(node[1], size // 2)

    values = walk(root, capacity)
    assert all(padding == 0 for padding in values[leaves:])
    if tag == 1397051697:
        return values[:leaves]
    assert all(0 <= word < 2**32 for word in values[:leaves])
    raw = b"".join(word.to_bytes(4, "little") for word in values[:leaves])
    assert not any(raw[length:])
    return raw[:length]


def decoded_graph(value):
    words = []
    for _ in range(8):
        word, value = value
        words.append(word)
    modules, (order, options) = value
    rows = []
    for index, (name, (source, (offset, (status, (count, uses))))) in sequence(modules):
        occurrences = []
        for _ in range(count):
            (target, (start, (path_start, end))), uses = uses
            occurrences.append({"target": target, "start": start, "path_start": path_start, "end": end})
        assert uses == 0
        rows.append({"index": index, "name": sequence(name, 1113150513).decode(),
                     "source_hex": sequence(source, 1113150513).hex(), "offset": offset,
                     "status": status, "uses": occurrences})
    return {**dict(zip(["index", "code", "start", "end", "admitted", "remaining", "bytes", "uses"], words)),
            "modules": rows, "order": sequence(order), "options": options}


def cases():
    def row(name, modules, error=(0, 0, 0, 0), order=None, sequence_cap=65536):
        return {"case": name, "sources": {path: source if isinstance(source, bytes) else source.encode()
                                           for path, source in modules.items()},
                "error": error, "order": order, "sequence_cap": sequence_cap}

    yield row("diamond-repeated", {"a": "module a use z", "b": "module b use z",
        "entry": "program entry use b use a use b fn main()->Field{7}", "unused": b"\xff", "z": "module z"},
        order=["z", "a", "b", "entry"])
    yield row("lexical-root-dfs", {"a": "module a use z", "b": "module b",
        "entry": "program entry use b use a", "z": "module z"}, order=["z", "a", "b", "entry"])
    yield row("commented-dotted", {"entry": "program entry use pkg // import\n . unit",
        "pkg.unit": "module pkg // owner\n . unit"}, order=["pkg.unit", "entry"])
    entry = "program entry use a"
    for name, source, code, start, end in [
        ("missing", b"module a use missing", 3, 9, 20),
        ("cycle", b"module a use entry", 4, 9, 18),
        ("wrong-owner", b"module wrong", 3, 7, 12),
        ("invalid-utf8", b"module a\xff", 1, 8, 9),
        ("invalid-boundary", b"module a $", 1, 9, 10),
    ]:
        yield row(name, {"a": source, "entry": entry}, error=(0, code, start, end))
    yield row("use-capacity", {"a": "module a", "entry": entry + " use a use a"},
        error=(1, 7, 26, 31), sequence_cap=2)
    exact = "module a //" + " " * (4096 - len(entry) - 11)
    yield row("source4096", {"a": exact, "entry": entry}, order=["a", "entry"])
    yield row("source4097", {"a": exact + " ", "entry": entry}, error=(0, 7, 0, 0))
    large = {"entry": "program entry use z", **{f"m{i:04}": b"\xff" for i in range(4095)}, "z": "module z"}
    yield row("index4096", large, order=["z", "entry"])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--joy", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[2]
    binary = args.joy.resolve()
    spec = importlib.util.spec_from_file_location("native", Path(__file__).with_name("run-native-compiler.py"))
    native = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(native)
    commands, observations = [], []
    sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    host = ["--budget", "100000000", "--frames", "65536", "--time-ms", "60000", "--arena-nodes", "786432",
            "--modules", "65536"]

    def run(arguments):
        command = [str(binary), *map(str, arguments)]
        result = subprocess.run(command, cwd=repo, capture_output=True, text=True)
        record = {"command": command, "exit_code": result.returncode, "stdout": result.stdout, "stderr": result.stderr}
        commands.append(record)
        assert result.returncode == 0, record
        return json.loads(result.stdout) if result.stdout.startswith("{") else None

    with tempfile.TemporaryDirectory(prefix="trident-guest-graph-") as temporary:
        root = Path(temporary)
        compiler, component, wrapper = (root / f"{name}.dag" for name in ["compiler", "component", "wrapper"])
        wrapper_source = "program wrap fn main(input:Noun)->Noun{nox_noun_pair(input,nox_noun_atom(0))}"
        (root / "wrap.tri").write_text(wrapper_source)
        for source, profile, output in [(repo / "compiler/nox/main.tri", "compiler-job", compiler),
                (repo / "tests/fixtures/native_module_graph.tri", "raw", component), (root / "wrap.tri", "raw", wrapper)]:
            run(["build", source, "--emit", "artifact", "--artifact-profile", profile, "-o", output])
        artifacts = {name: {"sha256": sha(path), "particle": path.read_bytes()[8:40].hex()}
                     for name, path in [("compiler", compiler), ("component", component), ("wrapper", wrapper)]}
        for case in cases():
            directory = root / case["case"]
            directory.mkdir()
            files = []
            sources = dict(sorted(case["sources"].items()))
            for i, (name, source) in enumerate(sources.items()):
                file = f"{i}.tri"
                (directory / file).write_bytes(source)
                files.append({"logical_path": name, "file": file, "origin_name": "graph", "origin_version": "1"})
            manifest = {"version": 1, "entry_module": "entry", "entry_function": "main", "modules": files,
                "options": {"target": 0, "input_profile": 0, "output_profile": 0, "optimization": 0, "cfg_flags": ["explicit"]},
                "limits": {"source_bytes": 8192, "modules": 65536, "diagnostics": 16,
                    "sequence_length": case["sequence_cap"], "validation_visits": 1000000,
                    "artifact_bytes": 16777216, "artifact_nodes": 196608, "artifact_depth": 4096,
                    "reductions": 100000000, "arena_nodes": 786432, "evaluator_frames": 65536}}
            package = directory / "package.json"
            package.write_text(json.dumps(manifest))
            job, wrapped, output = (directory / f"{name}.dag" for name in ["job", "wrapped", "result"])
            run(["pack-job", "--compiler", compiler, "--manifest", package, "-o", job, *host])
            run(["run-artifact", wrapper, "--input", job, "-o", wrapped, *host])
            execution = run(["run-artifact", component, "--input", wrapped, "-o", output, *host])
            decoded = decoded_graph(native.decode(output))
            actual = tuple(decoded[key] for key in ["index", "code", "start", "end"])
            if case["error"][1] == 0:
                assert decoded["code"] == 0, (case["case"], decoded)
                assert [decoded["modules"][i]["name"] for i in decoded["order"]] == case["order"]
                assert all(item["status"] == 2 for item in decoded["modules"])
                assert decoded["bytes"] == sum(len(bytes.fromhex(item["source_hex"])) for item in decoded["modules"])
                for item in decoded["modules"]:
                    assert item["source_hex"] == sources[item["name"]].hex()
                    assert item["index"] == list(sources).index(item["name"])
                if case["case"] == "diamond-repeated":
                    assert [u["target"] for u in decoded["modules"][0]["uses"]] == [1, 3, 1]
                if case["case"] == "index4096":
                    assert [m["index"] for m in decoded["modules"]] == [0, 4096]
                    assert decoded["modules"][0]["uses"][0]["target"] == 1
            else:
                assert actual == case["error"], (case["case"], actual, case["error"])
                assert decoded["order"] == []
            job_value = native.decode(job)[1]
            for _ in range(4):
                job_value = job_value[1]
            assert decoded.pop("options") == job_value[0]
            observations.append({"case": case["case"], "sources_hex": {k: v.hex() for k, v in sources.items()},
                "manifest": manifest, "job_sha256": sha(job), "result_sha256": sha(output),
                "graph": decoded, "execution": execution})
        receipt = {"schema": "trident/guest-module-graph-cli/v1", "kind": "local-component-validation",
            "binary_sha256": sha(binary), "artifacts": artifacts, "wrapper_source": wrapper_source,
            "commands": commands, "observations": observations,
            "remaining": "Complete guest linking, legacy import remaps and compiler self-build are not exercised by this component."}
    args.output.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({"commands": len(commands), "observations": len(observations)}))


if __name__ == "__main__":
    main()
