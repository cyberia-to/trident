"""Validation of original producer/corpus phase receipts; no execution fallback."""
import hashlib
import importlib.util
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
_spec = importlib.util.spec_from_file_location("bootstrap_runner", HERE / "bootstrap-runner.py")
B = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(B)
SCHEMA = "trident/clean-bootstrap-phase/v1"
MODULES = ("bootstrap-runner.py", "bootstrap-phases.py", "bootstrap_phase_checks.py")


def implementation(directory=HERE):
    return {name: B.sha(directory / name) for name in MODULES}


def normalized(path):
    return str(path).replace("\\", "/")


def option(command, flag):
    B.require(command.count(flag) == 1 and command.index(flag) + 1 < len(command), f"one {flag} required")
    return command[command.index(flag) + 1]


def common(root, report, phase):
    B.require(root.is_dir() and not root.is_symlink(), "ordinary phase directory required")
    B.require(report["schema"] == SCHEMA and report["phase"] == phase, "phase schema/role")
    B.require(report["status"] == ("produced" if phase == "producer" else "passed"), "phase incomplete")
    B.require(report["target"] in B.TARGETS and type(report["repeat"]) is int and report["repeat"] in (1, 2), "native target/repetition")
    B.pins(json.dumps(report["pins"]))
    origin = B.validate_origin(report["ci_origin"])
    B.require(origin is None or origin["head_sha"] == report["pins"]["trident"], "phase head/source pin")
    B.require(re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", report["rust_version"]), "Rust version")
    rust = report["host"]["rust"].splitlines()
    B.require(f"host: {report['target']}" in rust and f"release: {report['rust_version']}" in rust, "recorded native Rust")
    host = report["host"]
    arch = {"arm64": "arm64", "aarch64": "arm64", "x86_64": "x64", "amd64": "x64"}.get(host["machine"].lower())
    B.require((host["system"], arch) == B.TARGETS[report["target"]], "recorded native OS/architecture")
    B.require(host["system"] != "Linux" or host["libc"] == "glibc", "recorded native glibc")
    B.require(all(row["status"] == "completed" and type(row["exit_code"]) is int and row["exit_code"] == 0
                  for row in report["commands"]), "outer phase command did not complete")
    B.require(report["profile"] == B.PROFILE, "phase profile")
    impl = report["implementation"]
    B.require(impl == implementation(), "phase implementation differs from current protocol")
    B.require(report["files"]["path"] == "files.json" and
              B.load(B.retained(root, report["files"])) == B.evidence_files(root), "phase raw evidence manifest")
    return (report["pins"], report["rust_version"], report["profile"], impl, origin)


def producer(root, report):
    contract = common(root, report, "producer")
    B.require(len(report["repetitions"]) == 1, "single producer repetition")
    row = report["repetitions"][0]
    number = report["repeat"]
    B.require(type(row["number"]) is int and row["number"] == number and row["status"] == "produced", "producer repetition role")
    B.require(row["corpora"] == {}, "producer cannot substitute corpus acceptance")
    expected = {"c1": "c1.dag", "c2": "c2.dag", "c3": "c3.dag", "inventory": "inventory.json",
                "step2": "c2-step.json", "step3": "c3-step.json", "fixed_point": "fixed-point.json"}
    for key, name in expected.items():
        B.require(row[key]["path"] == f"repeat-{number}/{name}", "producer artifact role/path")
    joy_name = "joy.exe" if "windows" in report["target"] else "joy"
    B.require(report["joy"]["path"] == f"repeat-{number}/{joy_name}", "native Joy export path")
    B.require(B.sha(B.retained(root, report["joy"])) == row["tools"]["joy"], "exported Joy identity")
    artifact, source, inventory = B.compiler_steps(root, row)
    B.require(len(B.load(B.retained(root, row["inventory"]))["modules"]) == 94, "complete compiler source closure")
    return contract, artifact, (source["source_sha256_set"], source["options"], source["limits"], inventory)


def corpus(root, report):
    contract = common(root, report, "corpus")
    generation = report["generation"]
    B.require(type(generation) is int and generation in (2, 3), "corpus generation")
    B.require(report["fixture_repositories"] == {name: report["pins"][name] for name in ("trident", "joy")}, "pinned sibling fixtures")
    original = B.load(B.retained(root, report["producer_receipt"]))
    manifest = B.retained(root, report["producer_manifest"])
    binding = report["producer"]
    B.require(binding["receipt_sha256"] == report["producer_receipt"]["sha256"] and
              binding["files_sha256"] == B.sha(manifest) == original["files"]["sha256"], "original producer receipt/manifest binding")
    B.require(original["schema"] == SCHEMA and original["phase"] == "producer" and original["status"] == "produced", "consumer requires producer")
    for field in ("target", "repeat", "pins", "rust_version", "profile", "ci_origin", "implementation"):
        B.require(original[field] == report[field], f"producer/corpus {field} differs")
    B.require(len(original["repetitions"]) == 1 and original["repetitions"][0]["number"] == report["repeat"], "original repetition")
    selected = original["repetitions"][0][f"c{generation}"]
    B.require(binding["generation"] == generation and binding["compiler"] == selected and binding["joy"] == original["joy"], "selected producer generation identity")
    B.require(selected["path"] == f"repeat-{report['repeat']}/c{generation}.dag", "original compiler generation path")
    B.require(report["compiler"]["path"] == f"inputs/c{generation}.dag", "consumer compiler generation path")
    compiler = B.retained(root, report["compiler"])
    joy_name = "joy.exe" if "windows" in report["target"] else "joy"
    B.require(report["joy"]["path"] == f"inputs/{joy_name}", "consumer Joy path")
    joy = B.retained(root, report["joy"])
    B.require(report["compiler"]["sha256"] == selected["sha256"] and report["compiler"]["bytes"] == selected["bytes"], "copied producer compiler")
    B.require(report["joy"]["sha256"] == original["joy"]["sha256"] and report["joy"]["bytes"] == original["joy"]["bytes"], "copied producer Joy")
    execution = report["execution"]
    compiler_path, joy_path = normalized(execution["compiler"]), normalized(execution["joy"])
    B.require(compiler_path.rsplit("/", 1)[0] == joy_path.rsplit("/", 1)[0], "execution inputs must share one directory")
    B.require(compiler_path.endswith("/" + report["compiler"]["path"]) and
              joy_path.endswith("/" + report["joy"]["path"]), "execution input paths")
    labels = {f"c{generation}-{name.removesuffix('.py')}": name for name in B.CORPORA}
    B.require(set(report["corpora"]) == set(labels), "all six generation corpora required")
    wrappers = {}
    for row in report["commands"]:
        command = row["command"]
        executable = normalized(command[0]).split("/")[-1].lower()
        B.require(executable not in ("cargo", "cargo.exe"), "corpus phase rebuilt tools")
        if executable in ("rustc", "rustc.exe"):
            B.require(command[1:] == ["-vV"], "corpus rustc compilation forbidden")
        elif executable not in ("git", "git.exe"):
            B.require("--corpus-child" in command and len(command) > 1 and
                      normalized(command[1]).endswith("/bootstrap-runner.py"), "unknown corpus phase executable")
        if "--corpus-child" in command:
            name = normalized(option(command, "--corpus-child")).split("/")[-1]
            B.require(name not in wrappers and name in B.CORPORA, "unique known corpus wrapper")
            B.require(row["status"] == "completed" and row["exit_code"] == 0, "corpus wrapper failed")
            B.require(normalized(option(command, "--compiler")) == compiler_path and
                      normalized(option(command, "--joy")) == joy_path, "wrapper supplied compiler/Joy route")
            wrappers[name] = command
    B.require(set(wrappers) == set(B.CORPORA), "all six actual corpus invocations required")
    for label, name in labels.items():
        identity = report["corpora"][label]
        B.require(identity["path"] == f"corpora/{label}.json", "corpus receipt generation path")
        child = B.load(B.retained(root, identity))
        B.corpus_identity(child, B.sha(compiler), B.sha(joy), B.CORPORA[name])
        B.require(normalized(child["compiler_path"]) == compiler_path, "child selected generation path")
        B.require(normalized(option(wrappers[name], "--output")).endswith("/" + identity["path"]), "wrapper receipt path")
        B.require(any(len(row["command"]) > 2 and row["command"][1] == "pack-job" and
                      "--compiler" in row["command"] and normalized(option(row["command"], "--compiler")) == compiler_path
                      for row in child["commands"]), "selected compiler was not packed")
    return contract, binding


def compare_matrix(reports, origin=None):
    B.require(len(reports) == 36, "exactly thirty-six original phase receipts required")
    B.require(len({root.resolve() for root, _ in reports}) == 36, "phase directory reused")
    producers, consumers = {}, {}
    contract, baseline, source_contract = None, None, None
    for root, report in reports:
        B.require(origin is None or report["ci_origin"] == origin, "phase matrix CI run/attempt/head")
        key = (report["target"], report["repeat"])
        if report["phase"] == "producer":
            B.require(key not in producers, "duplicate producer")
            current, artifact, source = producer(root, report)
            B.require(baseline is None or baseline == artifact, "canonical compiler bytes differ")
            B.require(source_contract is None or source_contract == source, "source/options/limits differ")
            baseline, source_contract = artifact, source
            producers[key] = (root, report)
        else:
            B.require(report["phase"] == "corpus", "unknown phase role")
            current, binding = corpus(root, report)
            key = (*key, report["generation"])
            B.require(key not in consumers, "duplicate corpus generation")
            consumers[key] = binding
        B.require(contract is None or contract == current, "phase contract differs")
        contract = current
    pairs = {(target, repeat) for target in B.TARGETS for repeat in (1, 2)}
    B.require(set(producers) == pairs and set(consumers) == {(*key, g) for key in pairs for g in (2, 3)}, "complete twelve producers/twenty-four consumers required")
    for (target, repeat, generation), binding in consumers.items():
        root, report = producers[target, repeat]
        B.require(binding["receipt_sha256"] == B.sha(root / "receipt.json") and
                  binding["files_sha256"] == report["files"]["sha256"], "consumer points to different original producer")
        B.require(binding["compiler"] == report["repetitions"][0][f"c{generation}"] and
                  binding["joy"] == report["joy"], "producer export binding")
    return hashlib.sha256(baseline).hexdigest()
