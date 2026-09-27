"""SH6 runner: selected clean native bootstraps and twelve-job comparison.

Run with --pins-json '{"trident":"<40 hex>", ...}' (or BOOTSTRAP_PINS),
--target <native Rust triple>, --work <fresh build directory>, --output <fresh
evidence directory>. All nine repository pins are required. Network preparation
fetches exact Git objects and locked crates; builds and compiler jobs are offline.
Rust builds C1 once per clean repetition. Supplied C2/C3 run all six corpora;
their separately labeled raw Rust reference oracles never replace a compiler.
The fixed profile is 20B reductions, 1B cumulative nodes, 3M resident nodes,
10B collection work and two hours per whole compiler job; corpus limits are unchanged.
Retained evidence is bounded to 50,000 files and 4 GiB, with a checked file manifest.

--prepare-only builds tools/inventory/C1 and reports 'prepared', never acceptance.
--repeat 1|2 selects one clean repetition; the local default runs both.
--matrix <download directory> requires six targets times two single-repeat receipts,
checks retained files and exact C2/C3 equality. Missing evidence is a failing gate.
Temporary corpus files are retained by a child-only tempfile adapter; source,
compiler instructions, expected results and existing corpus limits are unchanged.
No workflow execution or file generation alone closes SH6 or its separate proofs.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import runpy
import signal
import subprocess
import sys
import tempfile
import time

REPOS = ("trident", "joy", "nox", "hemera", "strata", "zheng", "bbg", "lens", "neuron")
TARGETS = {
    "aarch64-apple-darwin": ("Darwin", "arm64"),
    "x86_64-apple-darwin": ("Darwin", "x64"),
    "aarch64-unknown-linux-gnu": ("Linux", "arm64"),
    "x86_64-unknown-linux-gnu": ("Linux", "x64"),
    "aarch64-pc-windows-msvc": ("Windows", "arm64"),
    "x86_64-pc-windows-msvc": ("Windows", "x64"),
}
CORPORA = {
    "run-native-compiler.py": 402,
    "check-guest-constant-linking.py": 31,
    "check-guest-function-imports.py": 32,
    "check-guest-type-imports.py": 24,
    "check-guest-intrinsics.py": 37,
    "check-generated-compiler-profile.py": 21,
}
PROFILE = {"budget": 20_000_000_000, "arena-nodes": 1_000_000_000,
           "resident-nodes": 3_145_728, "collection-work": 10_000_000_000,
           "time-ms": 7_200_000, "validation-visits": 16_777_216}
SCHEMA = "trident/clean-bootstrap/v2"
MAX_FILE = 16 << 20
MAX_EVIDENCE_FILES, MAX_EVIDENCE_BYTES = 50_000, 4 << 30


def require(value, message):
    if not value:
        raise ValueError(message)


def read(path, limit=MAX_FILE):
    with Path(path).open("rb") as source:
        data = source.read(limit + 1)
    require(len(data) <= limit, f"bounded file: {path}")
    return data


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as source:
        for chunk in iter(lambda: source.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load(path):
    return json.loads(read(path))


def pins(value):
    result = json.loads(value)
    require(isinstance(result, dict) and set(result) == set(REPOS), "exact nine repository pins required")
    require(all(isinstance(v, str) and re.fullmatch(r"[0-9a-f]{40}", v) for v in result.values()),
            "pins must be full lowercase Git commit identities")
    return result


def validate_origin(origin):
    if origin is not None:
        require(isinstance(origin, dict) and set(origin) == {"run_id", "run_attempt", "head_sha"}, "CI origin fields")
        require(all(isinstance(origin[k], str) and re.fullmatch(r"[1-9][0-9]*", origin[k])
                    for k in ("run_id", "run_attempt")), "CI run identity")
        require(isinstance(origin["head_sha"], str) and re.fullmatch(r"[0-9a-f]{40}", origin["head_sha"]), "CI head identity")
    return origin


def ci_origin():
    if os.environ.get("GITHUB_ACTIONS") != "true":
        return None
    return validate_origin(dict(run_id=os.environ.get("GITHUB_RUN_ID"),
                                run_attempt=os.environ.get("GITHUB_RUN_ATTEMPT"),
                                head_sha=os.environ.get("BOOTSTRAP_HEAD_SHA")))


def selected(report):
    numbers = report["selected_repetitions"]
    require(isinstance(numbers, list) and all(type(n) is int for n in numbers)
            and numbers in ([1], [2], [1, 2]), "selected repetitions must be [1], [2] or [1, 2]")
    return numbers


def native(target, rust, version):
    machine = platform.machine().lower()
    arch = {"arm64": "arm64", "aarch64": "arm64", "x86_64": "x64", "amd64": "x64"}.get(machine)
    require(TARGETS[target] == (platform.system(), arch), "native platform differs from requested target")
    require(f"host: {target}" in rust.splitlines(), "Rust host differs from native target; cross execution forbidden")
    require(f"release: {version}" in rust.splitlines(), "Rust release differs from pinned version")
    if platform.system() == "Linux":
        require(platform.libc_ver()[0] == "glibc", "SH6 Linux requires glibc")


def stop(process):
    if process.poll() is not None:
        return
    if os.name == "nt":
        subprocess.run(["taskkill", "/PID", str(process.pid), "/T", "/F"],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=30, check=False)
    else:
        os.killpg(process.pid, signal.SIGKILL)
    process.wait(timeout=30)


class Audit:
    def __init__(self, output, report):
        self.output = output.resolve()
        self.output.mkdir(parents=True, exist_ok=False)
        self.report = dict(schema=SCHEMA, status="running", commands=[], **report)
        self.env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1", PYTHONUTF8="1")
        for name in ("RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS",
                     "PYTHONOPTIMIZE", "PYTHONPATH", "PYTHONHOME"):
            self.env.pop(name, None)
        self.flush()

    def flush(self):
        temporary = self.output / "receipt.next.json"
        temporary.write_text(json.dumps(self.report, indent=2) + "\n", encoding="utf-8", newline="\n")
        temporary.replace(self.output / "receipt.json")

    def run(self, command, cwd, timeout=600):
        command = list(map(str, command))
        number = len(self.report["commands"])
        row = dict(command=command, cwd=str(cwd), status="running", exit_code=None,
                   stdout=f"commands/{number}.stdout", stderr=f"commands/{number}.stderr")
        self.report["commands"].append(row)
        self.flush()
        directory = self.output / "commands"
        directory.mkdir(exist_ok=True)
        started = time.monotonic_ns()
        print(f"[{number}] {' '.join(command[:5])}", flush=True)
        with (self.output / row["stdout"]).open("xb") as out, (self.output / row["stderr"]).open("xb") as err:
            kwargs = {"creationflags": subprocess.CREATE_NEW_PROCESS_GROUP} if os.name == "nt" else {"start_new_session": True}
            process = None
            try:
                process = subprocess.Popen(command, cwd=cwd, env=self.env, stdout=out, stderr=err, **kwargs)
                row["exit_code"] = process.wait(timeout=timeout)
                row["status"] = "completed"
            except BaseException:
                if process is not None:
                    stop(process)
                row["status"] = "interrupted" if process is not None else "launch-failed"
                raise
            finally:
                row["elapsed_ns"] = time.monotonic_ns() - started
                self.flush()
        require(row["exit_code"] == 0, f"command {number} failed; retained {row['stderr']}")
        return read(self.output / row["stdout"]).decode("utf-8")

    def identity(self, path):
        path = Path(path)
        return dict(path=path.relative_to(self.output).as_posix(), sha256=sha(path), bytes=path.stat().st_size)


def clean(audit, repo, revision):
    require(audit.run(["git", "rev-parse", "HEAD"], repo).strip() == revision, "checkout revision changed")
    require(not audit.run(["git", "status", "--porcelain", "--untracked-files=all"], repo).strip(),
            f"source checkout changed: {repo.name}")


def prepare(audit, work, evidence, target, source_pins):
    work.mkdir(parents=True, exist_ok=False)
    evidence.mkdir()
    sources = work / "sources"
    sources.mkdir()
    for name in REPOS:
        repo = sources / name
        audit.run(["git", "init", "-q", repo], work)
        audit.run(["git", "config", "core.autocrlf", "false"], repo)
        audit.run(["git", "config", "core.longpaths", "true"], repo)
        audit.run(["git", "remote", "add", "origin", f"https://github.com/cyberia-to/{name}.git"], repo)
        audit.run(["git", "fetch", "--depth", "1", "origin", source_pins[name]], repo)
        audit.run(["git", "checkout", "--detach", "FETCH_HEAD"], repo)
        clean(audit, repo, source_pins[name])
    audit.env.update(CARGO_TARGET_DIR=str(work / "target"), CARGO_BUILD_TARGET=target)
    audit.env.pop("CARGO_NET_OFFLINE", None)
    for name in ("joy", "trident"):
        audit.run(["cargo", "fetch", "--locked", "--target", target], sources / name, 1800)
    audit.env["CARGO_NET_OFFLINE"] = "true"
    build = ["cargo", "build", "--release", "--locked", "--offline"]
    audit.run([*build, "-p", "cyber-joy"], sources / "joy", 1800)
    audit.run([*build, "--example", "selfhost_inventory"], sources / "trident", 1800)
    suffix = ".exe" if os.name == "nt" else ""
    release = work / "target" / target / "release"
    joy, checker = release / ("joy" + suffix), release / "examples" / ("selfhost_inventory" + suffix)
    inventory, c1 = evidence / "inventory.json", evidence / "c1.dag"
    audit.run([checker, "--root", sources / "trident", "--entry", "compiler/nox/main.tri", "--output", inventory], sources)
    indexed = audit.run(["git", "ls-files", "-s", "-z"], sources / "trident")
    blobs = {entry.split("\t", 1)[1]: entry.split()[1] for entry in indexed.split("\0") if entry}
    for module in load(inventory)["modules"].values():
        path = Path(module["path"])
        require(not path.is_absolute() and ".." not in path.parts, "inventory source path")
        content = read(sources / "trident" / path)
        blob = hashlib.sha1(b"blob " + str(len(content)).encode("ascii") + b"\0" + content).hexdigest()
        require(blobs[module["path"]] == blob, "working source bytes differ from pinned Git blob")
    audit.run([joy, "build", sources / "trident/compiler/nox/main.tri", "--emit", "artifact",
               "--artifact-profile", "compiler-job", "-o", c1], sources / "trident")
    return sources, joy, checker, inventory, c1


def corpus_identity(report, compiler_sha, binary_sha, count):
    require(report["status"] == "passed" and report["compiler_mode"] == "provided", "corpus did not use supplied compiler")
    require(report["compiler_sha256_start"] == report["compiler_sha256_end"] == compiler_sha, "corpus compiler identity")
    require(report.get("binary_sha256_start", report.get("binary_sha256")) == binary_sha, "corpus Joy identity")
    if "binary_sha256_end" in report:
        require(report["binary_sha256_end"] == binary_sha, "corpus Joy changed")
    require(len(report["observations"]) == count, "complete corpus observation count")
    for row in report["commands"]:
        command = row["command"]
        if len(command) > 1 and command[1] == "build":
            raw = any(command[i:i + 2] == ["--artifact-profile", "raw"] for i in range(len(command) - 1))
            require(row.get("reference_only") is True and raw, "host compiler fallback in supplied corpus")


def corpus_result(report, compiler, binary, count):
    corpus_identity(report, sha(compiler), sha(binary), count)
    require(Path(report["compiler_path"]).resolve() == compiler.resolve(), "corpus compiler path")


def repetition(audit, work, number, target, source_pins, prepare_only=False):
    evidence = audit.output / f"repeat-{number}"
    sources, joy, checker, inventory, c1 = prepare(audit, work, evidence, target, source_pins)
    summary = dict(number=number, tools={"joy": sha(joy), "inventory": sha(checker)},
                   inventory=audit.identity(inventory), c1=audit.identity(c1), corpora={})
    audit.report["repetitions"].append(summary)
    audit.flush()
    if prepare_only:
        for name in REPOS:
            clean(audit, sources / name, source_pins[name])
        return summary
    helpers = sources / "trident/audit/self-hosting"
    require(sha(helpers / "bootstrap-runner.py") == sha(__file__), "runner differs from pinned Trident source")
    steps, compiler = [], c1
    for generation in (2, 3):
        receipt = evidence / f"c{generation}-step.json"
        command = [sys.executable, helpers / "probe-native-closure.py", "--joy", joy,
                   "--compiler", compiler, "--inventory", inventory, "--output", receipt, "--emit", "program"]
        for key, value in PROFILE.items():
            command.extend(["--" + key, str(value)])
        audit.run(command, sources / "trident", 7500)
        result = load(receipt)
        require(result["status"] == "compiler-returned" and result["published_kind"] == "program", "whole compiler step failed")
        require(result["compiler_sha256"] == result["compiler_sha256_end"] == sha(compiler), "step compiler changed")
        require(result["binary_sha256"] == result["binary_sha256_end"] == sha(joy), "step Joy changed")
        emitted = Path(result["artifact_directory"]) / "result.dag"
        require(sha(emitted) == result["result_sha256"], "emitted compiler changed")
        compiler = evidence / f"c{generation}.dag"
        with compiler.open("xb") as output:
            output.write(read(emitted))
        summary[f"c{generation}"] = audit.identity(compiler)
        summary[f"step{generation}"] = audit.identity(receipt)
        steps.append(receipt)
        audit.flush()
    comparison = evidence / "fixed-point.json"
    audit.run([sys.executable, helpers / "check-selfhost-fixed-point.py", "--first", steps[0], "--second", steps[1],
               "--inventory-checker", checker, "--joy", joy, "--output", comparison], sources / "trident", 300)
    checked = load(comparison)
    require(checked["status"] == "passed", "fixed-point checker rejected")
    require(read(evidence / "c2.dag") == read(evidence / "c3.dag"), "C2/C3 bytes differ")
    summary["fixed_point"] = audit.identity(comparison)
    summary["comparison"] = checked["fixed_point"]
    for generation in (2, 3):
        compiler = evidence / f"c{generation}.dag"
        for script, count in CORPORA.items():
            label = f"c{generation}-{script.removesuffix('.py')}"
            receipt = evidence / (label + ".json")
            temporary = evidence / (label + "-files")
            temporary.mkdir()
            command = [sys.executable, Path(__file__).resolve(), "--corpus-child", helpers / script,
                       "--retain", temporary, "--joy", joy, "--compiler", compiler, "--output", receipt]
            audit.run(command, sources / "trident", 5400)
            corpus_result(load(receipt), compiler, joy, count)
            summary["corpora"][label] = audit.identity(receipt)
            audit.flush()
    for name in REPOS:
        clean(audit, sources / name, source_pins[name])
    summary["status"] = "passed"
    audit.flush()
    return summary


def retained_corpus(args):
    require(sys.flags.optimize == 0, "corpus assertions require unoptimized Python")
    require(args.corpus_child.name in CORPORA, "unknown corpus child")
    class RetainedTemporaryDirectory:
        def __init__(self, suffix=None, prefix=None, dir=None, **kwargs):
            self.name = tempfile.mkdtemp(suffix=suffix, prefix=prefix, dir=dir or args.retain)
        def __enter__(self):
            return self.name
        def __exit__(self, *_):
            pass
        def cleanup(self):
            pass
    tempfile.TemporaryDirectory = RetainedTemporaryDirectory
    sys.path.insert(0, str(args.corpus_child.parent))
    sys.argv = [str(args.corpus_child), "--joy", str(args.joy), "--compiler", str(args.compiler), "--output", str(args.output)]
    if args.corpus_child.name == "run-native-compiler.py":
        sys.argv.extend(["--time-ms", "60000"])
    runpy.run_path(str(args.corpus_child), run_name="__main__")


def retained(root, identity):
    path = (root / identity["path"]).resolve()
    require(path.is_relative_to(root.resolve()), "artifact path escapes evidence")
    require(path.stat().st_size == identity["bytes"] and sha(path) == identity["sha256"], "retained artifact identity")
    return path


def evidence_files(root):
    files, total = {}, 0
    for path in root.rglob("*"):
        require(not path.is_symlink(), "evidence contains a symbolic link")
        if path.is_file() and path not in [root / n for n in ("receipt.json", "receipt.next.json", "files.json")]:
            name, size = path.relative_to(root).as_posix(), path.stat().st_size
            total += size
            require(len(files) < MAX_EVIDENCE_FILES and total <= MAX_EVIDENCE_BYTES, "evidence size limit")
            files[name] = dict(bytes=size, sha256=sha(path))
    return files


def compiler_steps(root, row):
    c1, inventory = retained(root, row["c1"]), retained(root, row["inventory"])
    c2, c3 = retained(root, row["c2"]), retained(root, row["c3"])
    require(read(c2) == read(c3), "retained C2/C3 differ")
    fixed = load(retained(root, row["fixed_point"]))
    require(fixed["status"] == "passed" and fixed["fixed_point"]["artifact_sha256"] == sha(c2), "fixed-point evidence binding")
    for name, key in (("job_checker", "joy"), ("inventory_checker", "inventory")):
        require(fixed[name + "_sha256_start"] == fixed[name + "_sha256_end"] == row["tools"][key], "checker tool binding")
    for generation in (2, 3):
        step_path = retained(root, row[f"step{generation}"])
        step = load(step_path)
        require(step["status"] == "compiler-returned" and step["published_kind"] == "program" and
                step["result_sha256"] == row[f"c{generation}"]["sha256"], "step evidence binding")
        require(step["compiler_sha256"] == step["compiler_sha256_end"] == sha(c1 if generation == 2 else c2), "step compiler chain binding")
        require(step["binary_sha256"] == step["binary_sha256_end"] == row["tools"]["joy"], "step tool binding")
        require(fixed["steps"][generation - 2]["receipt_sha256"] == sha(step_path), "fixed-point step receipt binding")
        require(step["inventory_sha256"] == sha(inventory), "step inventory binding")
        expected = {"--" + key: str(value) for key, value in PROFILE.items()} | {"--frames": "65536"}
        flags = step["host_flags"]
        require(len(flags) == 2 * len(expected) and dict(zip(flags[::2], flags[1::2])) == expected, "step resource profile binding")
    require(row["comparison"] == fixed["fixed_point"], "comparison summary differs from checked receipt")
    return read(c2), fixed["fixed_point"], sha(inventory)


def compare(reports):
    require(len(reports) > 0, "missing bootstrap evidence")
    baseline, contract, source_contract = None, None, None
    for root, report in reports:
        require(report["schema"] == SCHEMA and report["status"] == "passed", "bootstrap not passed/current schema")
        require(report["files"]["path"] == "files.json" and load(retained(root, report["files"])) == evidence_files(root), "retained raw evidence differs")
        require(f"host: {report['target']}" in report["host"]["rust"].splitlines(), "recorded runtime was not native")
        require(f"release: {report['rust_version']}" in report["host"]["rust"].splitlines(), "recorded Rust release differs")
        numbers, origin = selected(report), validate_origin(report["ci_origin"])
        require(report["profile"] == PROFILE and len(report["repetitions"]) == len(numbers), "bootstrap profile/repetition count")
        require(origin is None or origin["head_sha"] == report["pins"]["trident"], "CI head differs from source pin")
        current = (report["pins"], report["rust_version"], report["runner_sha256"], origin)
        require(contract is None or current == contract, "bootstrap inputs differ")
        contract = current
        for number, row in zip(numbers, report["repetitions"]):
            require(type(row.get("number")) is int and row["number"] == number, "distinct numbered repetitions required")
            require(row["status"] == "passed" and len(row["corpora"]) == 2 * len(CORPORA), "incomplete corpus repetition")
            identities = [row[k] for k in ("c1", "c2", "c3", "inventory", "fixed_point", "step2", "step3")]
            for identity in identities + list(row["corpora"].values()):
                require(retained(root, identity).is_relative_to(root.resolve() / f"repeat-{number}"), "repetition evidence directory reused")
            artifact, source, inventory_sha = compiler_steps(root, row)
            require(baseline is None or artifact == baseline, "bootstrap compiler bytes differ")
            baseline = artifact
            for generation in (2, 3):
                for script, count in CORPORA.items():
                    receipt = load(retained(root, row["corpora"][f"c{generation}-{script.removesuffix('.py')}"]))
                    corpus_identity(receipt, hashlib.sha256(artifact).hexdigest(), row["tools"]["joy"], count)
            other = (source["source_sha256_set"], source["options"], source["limits"], inventory_sha)
            require(source_contract is None or source_contract == other, "frozen source/options/limits differ")
            source_contract = other
    return hashlib.sha256(baseline).hexdigest()


def compare_matrix(reports, origin=None):
    require(len(reports) == 2 * len(TARGETS), "missing/duplicate SH6 platform evidence (twelve jobs required)")
    pairs = []
    for _, report in reports:
        numbers = selected(report)
        require(len(numbers) == 1, "matrix requires one selected repetition per job")
        pairs.append((report["target"], numbers[0]))
        require(origin is None or report["ci_origin"] == origin, "matrix CI run origin differs")
    require(set(pairs) == {(target, n) for target in TARGETS for n in (1, 2)}, "missing/duplicate SH6 platform/repeat evidence")
    require(len({root.resolve() for root, _ in reports}) == len(reports), "matrix evidence directory reused")
    return compare(reports)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("output", "work", "matrix", "corpus-child", "retain", "joy", "compiler"):
        parser.add_argument("--" + name, type=Path)
    parser.add_argument("--target", choices=TARGETS)
    parser.add_argument("--pins-json", default=os.environ.get("BOOTSTRAP_PINS"))
    parser.add_argument("--rust-version", default="1.95.0")
    parser.add_argument("--prepare-only", action="store_true")
    parser.add_argument("--repeat", type=int, choices=(1, 2))
    args = parser.parse_args()
    if args.corpus_child:
        retained_corpus(args)
        return 0
    require(args.output is not None, "fresh --output directory required")
    audit = Audit(args.output, dict(scope="SH6 bootstrap evidence; proof and semantic-preservation gates separate"))
    try:
        audit.report["ci_origin"] = ci_origin()
        if args.matrix:
            require(args.repeat is None and not args.prepare_only, "matrix does not accept repetition/prepare-only selection")
            reports = [(p.parent, load(p)) for p in args.matrix.glob("*/receipt.json")]
            audit.report["platform_reports"] = [dict(path=str(root / "receipt.json"), sha256=sha(root / "receipt.json")) for root, _ in reports]
            audit.report["compiler_sha256"] = compare_matrix(reports, audit.report["ci_origin"])
        else:
            require(args.target and args.work and args.pins_json, "--target, --work and exact source pins required")
            source_pins = pins(args.pins_json)
            origin = audit.report["ci_origin"]
            require(origin is None or origin["head_sha"] == source_pins["trident"], "CI head differs from source pin")
            require(re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", args.rust_version), "exact stable Rust version required")
            require(not args.work.resolve().is_relative_to(audit.output) and not audit.output.is_relative_to(args.work.resolve()),
                    "build and evidence directories must be separate")
            args.work.mkdir(parents=True, exist_ok=False)
            audit.env["CARGO_HOME"] = str(args.work.resolve() / "cargo-home")
            audit.report.update(target=args.target, pins=source_pins, rust_version=args.rust_version,
                                profile=PROFILE, runner_sha256=sha(__file__), repetitions=[],
                                selected_repetitions=[args.repeat] if args.repeat else ([1] if args.prepare_only else [1, 2]))
            audit.env["RUSTUP_TOOLCHAIN"] = f"{args.rust_version}-{args.target}"
            rust = audit.run(["rustc", "-vV"], Path.cwd())
            native(args.target, rust, args.rust_version)
            audit.report["host"] = dict(platform=platform.platform(), python=sys.version, rust=rust)
            for number in selected(audit.report):
                repetition(audit, args.work.resolve() / f"repeat-{number}", number, args.target, source_pins, args.prepare_only)
            if not args.prepare_only:
                with (audit.output / "files.json").open("x", encoding="utf-8", newline="\n") as output:
                    json.dump(evidence_files(audit.output), output, sort_keys=True)
                audit.report["files"] = audit.identity(audit.output / "files.json")
                audit.report["status"] = "passed"
                audit.report["compiler_sha256"] = compare([(audit.output, audit.report)])
        audit.report["status"] = "prepared" if args.prepare_only else "passed"
    except BaseException as error:
        audit.report.update(status="failed", error=f"{type(error).__name__}: {error}")
        audit.flush()
        print(audit.report["error"], file=sys.stderr)
        return 1
    audit.flush()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
