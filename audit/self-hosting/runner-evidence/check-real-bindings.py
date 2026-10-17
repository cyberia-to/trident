"""Check orchestration bindings against actual completed S0 receipts, without rerunning guests."""
import argparse
import importlib.util
import json
from pathlib import Path

SOURCE = Path(__file__).resolve().parents[1] / "bootstrap-runner.py"
SPEC = importlib.util.spec_from_file_location("bootstrap", SOURCE)
BOOTSTRAP = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BOOTSTRAP)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--measurements", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    root = args.measurements.resolve()
    def identity(path):
        path = Path(path).resolve()
        return dict(path=path.relative_to(root).as_posix(), bytes=path.stat().st_size, sha256=BOOTSTRAP.sha(path))
    first = root / "body-scale-closure-20b.json"
    second = root / "c2-to-c3-20b.json"
    checked = root / "body-scale-fixed-point.json"
    a, b, fixed = map(BOOTSTRAP.load, (first, second, checked))
    row = dict(c1=identity(a["compiler"]), inventory=identity(a["inventory"]),
               c2=identity(Path(a["artifact_directory"]) / "result.dag"),
               c3=identity(Path(b["artifact_directory"]) / "result.dag"),
               step2=identity(first), step3=identity(second), fixed_point=identity(checked),
               comparison=fixed["fixed_point"], tools={"joy": BOOTSTRAP.sha(a["binary"]),
                                                     "inventory": BOOTSTRAP.sha(fixed["inventory_checker"])})
    artifact, _, _ = BOOTSTRAP.compiler_steps(root, row)
    corpora = {}
    for name, count in (("main", 402), ("constants", 31), ("callable", 32),
                        ("types", 24), ("intrinsics", 37), ("generated-profile", 21)):
        path = root / "c2-corpus" / (name + ".json")
        BOOTSTRAP.corpus_identity(BOOTSTRAP.load(path), row["c2"]["sha256"], row["tools"]["joy"], count)
        corpora[name] = dict(receipt=identity(path), observations=count)
    report = dict(status="passed", sh6_accepted=False,
                  scope="Actual compiler-step and C2 corpus binding checks only; no clean repetitions, C3 corpus or platform acceptance",
                  runner_sha256=BOOTSTRAP.sha(SOURCE), checker_sha256=BOOTSTRAP.sha(__file__),
                  command=["python3", str(Path(__file__).resolve()), "--measurements", str(root), "--output", str(args.output.resolve())],
                  steps={k: row[k] for k in ("c1", "c2", "c3", "inventory", "step2", "step3", "fixed_point")},
                  compiler_bytes=len(artifact), tools=row["tools"], c2_corpora=corpora)
    with args.output.open("x", encoding="utf-8", newline="\n") as output:
        json.dump(report, output, indent=2)
        output.write("\n")
    print(report["scope"])


if __name__ == "__main__":
    main()
