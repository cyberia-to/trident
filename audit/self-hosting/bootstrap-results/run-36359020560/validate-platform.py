#!/usr/bin/env python3
"""Replay the frozen runner and compare one actual CI repetition to local S1."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import time


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def differences(left, right, prefix=""):
    if type(left) is not type(right):
        return [prefix]
    if isinstance(left, dict):
        result = []
        for key in sorted(left.keys() | right.keys()):
            path = f"{prefix}.{key}" if prefix else key
            result.extend([path] if key not in left or key not in right else
                          differences(left[key], right[key], path))
        return result
    if isinstance(left, list):
        if len(left) != len(right):
            return [prefix]
        return [path for i, (a, b) in enumerate(zip(left, right))
                for path in differences(a, b, f"{prefix}[{i}]")]
    return [] if left == right else [prefix]


parser = argparse.ArgumentParser(description=__doc__)
for flag in ("root", "runner", "expected", "output"):
    parser.add_argument("--" + flag, type=Path, required=True)
parser.add_argument("--target", required=True)
parser.add_argument("--repeat", type=int, choices=(1, 2), required=True)
args = parser.parse_args()
require(not args.output.exists(), "output must be fresh")
expected = json.loads(args.expected.read_bytes())
require(sha(args.runner) == expected["runner_sha256"], "frozen runner identity")
spec = importlib.util.spec_from_file_location("frozen_bootstrap_runner", args.runner)
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)
result = dict(command=[sys.executable, *sys.argv], started_ns=time.time_ns(),
              runner_sha256=sha(args.runner), validator_sha256=sha(Path(__file__)),
              expected_sha256=sha(args.expected), matrix_acceptance=False)
try:
    local_steps = expected["local_reference_steps"]
    require(type(local_steps) is list and len(local_steps) == 2 and
            all(type(row) is dict for row in local_steps), "exactly two local reference steps required")
    report = runner.load(args.root / "receipt.json")
    require(report["pins"] == expected["pins"], "exact source pins")
    require(report["runner_sha256"] == expected["runner_sha256"], "producer runner")
    require(report["rust_version"] == expected["rust_version"], "native Rust release")
    require(report["ci_origin"] == dict(run_id=str(expected["run_id"]),
            run_attempt=str(expected["run_attempt"]), head_sha=expected["head_sha"]),
            "exact CI origin")
    require(report["target"] == args.target and
            report["selected_repetitions"] == [args.repeat], "target/repetition")
    compiler = runner.compare([(args.root, report)])
    row = report["repetitions"][0]
    observations = []
    for generation, local in zip((2, 3), local_steps):
        step_path = runner.retained(args.root, row[f"step{generation}"])
        step = runner.load(step_path)
        for key in ("compiler_sha256", "result_sha256", "inventory_sha256", "job_sha256"):
            require(step[key] == local[key], f"S1 step{generation} {key}")
        execution = step["execution"]["execution"]
        observed = {key: value for key, value in execution.items() if key != "elapsed_micros"}
        observations.append(dict(generation=generation, receipt_sha256=sha(step_path),
            elapsed_micros=execution["elapsed_micros"],
            charged_reductions=execution["charged_reductions"],
            allocated_nodes=execution["allocated_nodes"],
            peak_frames=execution["peak_frames"],
            execution_except_elapsed_differences=differences(observed, local["execution_without_elapsed"])))
    result.update(status="passed", target=args.target, repetition=args.repeat,
                  receipt_sha256=sha(args.root / "receipt.json"),
                  compiler_sha256=compiler, steps=observations,
                  scope="Single retained repetition and frozen S1 identity; six-target matrix remains separate")
except BaseException as error:
    result.update(status="failed", error=f"{type(error).__name__}: {error}")
finally:
    result["finished_ns"] = time.time_ns()
    args.output.write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps({key: result[key] for key in ("status", "matrix_acceptance")}))
if result["status"] != "passed":
    print(result["error"], file=sys.stderr)
    raise SystemExit(1)
