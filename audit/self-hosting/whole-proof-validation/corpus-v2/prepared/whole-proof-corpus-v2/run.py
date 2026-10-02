"""Run unchanged SH6 corpora with compilers extracted by fresh full-proof verification."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import platform
import signal
import shutil
import os
import subprocess
import sys
import time
import traceback

ROOT = Path(__file__).resolve().parent
WHOLE = ROOT.parent / "whole-proof"
JOY = ROOT.parent / "production-install/installed/bin/joy"


def identity(path):
    if path.is_symlink() or not path.is_file():
        raise ValueError("regular input file required")
    with path.open("rb") as source:
        digest = hashlib.file_digest(source, "sha256").hexdigest()
    return dict(bytes=path.stat().st_size, sha256=digest)


def load(path):
    return json.loads(path.read_text())


def cancel(signum, _frame):
    raise InterruptedError("corpus signal: " + str(signum))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--generation", type=int, choices=(1, 2), required=True,
                        help="complete proof producer generation; extracted compiler is the next generation")
    parser.add_argument("--fixtures", type=Path, required=True)
    parser.add_argument("--revision", required=True)
    args = parser.parse_args()
    signal.signal(signal.SIGTERM, cancel)
    repo = args.fixtures.resolve()
    helpers = repo / "audit/self-hosting"
    compiler_generation = args.generation + 1
    verified = WHOLE / f"attempts/c{args.generation}-fresh-verification-1"
    receipt = load(verified / "receipt.json")
    if (receipt["status"] != "passed" or receipt["exit_code"] != 0 or
            receipt["action"] != "verify" or receipt["generation"] != args.generation):
        raise ValueError("actual successful fresh whole-proof verification required")
    compiler = verified / "compiler.dag"
    joy_fixtures = repo.parent / "joy"
    vectors = joy_fixtures / "cli/tests/compiler_vectors.json"
    selected_git = shutil.which("git")
    if selected_git is None:
        raise ValueError("absolute metadata Git executable is required")
    git_binary = Path(selected_git).resolve()
    if not os.access(git_binary, os.X_OK):
        raise ValueError("metadata Git executable is not executable")
    before = {str(p): identity(p) for p in (compiler, JOY, verified / "receipt.json", verified / "stdout", vectors, git_binary)}
    if not (before[str(JOY)] == receipt["binary"] == receipt["binary_after"] and
            before[str(compiler)] == receipt["files"]["compiler.dag"] and
            before[str(verified / "stdout")] == receipt["files"]["stdout"]):
        raise ValueError("same verified compiler and production Joy required")
    response = load(verified / "stdout")
    if response.get("ok") is not True or response.get("schema") != "joy/artifact-verification/v1":
        raise ValueError("successful versioned fresh verification response required")
    if receipt["profile_identity"] != identity(WHOLE / "profile.json"):
        raise ValueError("unchanged accepted profile required")
    profile = load(WHOLE / "profile.json")
    if identity(compiler)["sha256"] != profile["expected_artifact_sha256"]:
        raise ValueError("exact accepted C2/C3 program bytes required")
    git = lambda *values: subprocess.check_output([str(git_binary), *values], cwd=repo, text=True)
    git_joy = lambda *values: subprocess.check_output([str(git_binary), *values], cwd=joy_fixtures, text=True)
    if git("rev-parse", "HEAD").strip() != args.revision or git("status", "--porcelain", "--untracked-files=all"):
        raise ValueError("clean exact fixture revision required")
    joy_revision = git_joy("rev-parse", "HEAD").strip()
    if (joy_revision != "6e0ec4d8440e2521df08f442d64f54e667044716" or
            git_joy("status", "--porcelain", "--untracked-files=all")):
        raise ValueError("clean pinned production Joy fixtures required")
    sys.path.insert(0, str(helpers))
    spec = importlib.util.spec_from_file_location("original_bootstrap_runner", helpers / "bootstrap-runner.py")
    original = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(original)
    report = dict(generation=compiler_generation, proof_generation=args.generation,
                  scope="Fresh SH6 case matrix with explicit absolute metadata Git using the actual compiler extracted by full certificate verification; this is a local current-runtime check, separate from the original six-platform SH6 matrix.",
                  fixture_revision=args.revision, fixture_repository=str(repo),
                  joy_fixture_revision=joy_revision, joy_fixture_repository=str(joy_fixtures),
                  driver=identity(Path(__file__)), original_runner=identity(helpers / "bootstrap-runner.py"),
                  verified_receipt=before[str(verified / "receipt.json")],
                  inputs_before=before, corpora={}, started_ns=time.time_ns(),
                  host=dict(system=platform.system(), machine=platform.machine(), python=sys.version),
                  limits=dict(per_corpus_wall_seconds=5400, evidence_files=original.MAX_EVIDENCE_FILES,
                              evidence_bytes=original.MAX_EVIDENCE_BYTES),
                  runtime_environment={"PATH": "", "PYTHONUTF8": "1", "PYTHONDONTWRITEBYTECODE": "1",
                                       "TRIDENT_AUDIT_GIT": str(git_binary)})
    audit = original.Audit(ROOT / f"c{compiler_generation}", report,
                           schema="trident/proof-extracted-compiler-corpus/v2")
    # Every child executable is an absolute path; no compiler/runtime may be resolved through PATH.
    audit.env = dict(report["runtime_environment"])
    try:
        evidence = audit.output / "corpora"
        evidence.mkdir()
        original.generation_corpora(audit, evidence, helpers, compiler, JOY,
                                    compiler_generation, audit.report)
        original.require(len(audit.report["corpora"]) == len(original.CORPORA), "all unchanged corpora")
        audit.report["inputs_after"] = {path: identity(Path(path)) for path in before}
        original.require(audit.report["inputs_after"] == before, "verified inputs remain unchanged")
        original.require(git("rev-parse", "HEAD").strip() == args.revision and
                         not git("status", "--porcelain", "--untracked-files=all"), "fixture source unchanged")
        original.require(git_joy("rev-parse", "HEAD").strip() == joy_revision and
                         not git_joy("status", "--porcelain", "--untracked-files=all"), "Joy fixture source unchanged")
        files = original.evidence_files(audit.output)
        with (audit.output / "files.json").open("x") as output:
            json.dump(files, output, sort_keys=True)
        audit.report.update(status="passed", files=audit.identity(audit.output / "files.json"),
                            observations=sum(original.CORPORA.values()))
    except BaseException:
        audit.report.update(status="failed", error=traceback.format_exc())
        raise
    finally:
        audit.report["ended_ns"] = time.time_ns()
        audit.flush()
    print(json.dumps(dict(status=audit.report["status"], generation=compiler_generation,
                          corpora=len(audit.report["corpora"]), observations=audit.report["observations"])))


if __name__ == "__main__":
    main()
