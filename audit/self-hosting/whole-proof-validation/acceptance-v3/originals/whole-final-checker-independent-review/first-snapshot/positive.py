"""Retain reviewed v2 full-producer, fresh-verifier and corpus admission checks."""
from pathlib import Path
from common import (require, identity, load, same, files, semantic, profile_flags,
                    within, OUTPUT, PINS, CORPUS_FIXTURE_REVISION)


def review(base, number, bootstrap):
    whole = base / "whole-proof"
    producer = whole / f"attempts/c{number}-selfbuild-1"
    verifier = whole / f"attempts/c{number}-fresh-verification-1"
    p, v = (load(path / "receipt.json") for path in (producer, verifier))
    proof, compiler = producer / "proof.joysc", verifier / "compiler.dag"
    profile = load(whole / "profile.json")
    joy = base / "production-install/installed/bin/joy"
    for directory, row, action in ((producer, p, "prove"), (verifier, v, "verify")):
        require(row["status"] == "passed" and row["exit_code"] == 0
                and row["action"] == action and row["generation"] == number
                and "resource_stop" not in row, "complete actual producer/fresh verifier required")
        same(whole / "run.py", row["driver"])
        same(whole / "preparation.json", row["preparation"])
        same(whole / "profile.json", row["profile_identity"])
        same(whole / "installed-source-receipt.json", row["installed_source_receipt"])
        same(base / "production-install/installed/bin/joy", row["binary"])
        require(row["binary"] == row["binary_after"] and row["inputs_before"] == row["inputs_after"],
                "full command inputs changed")
        expected_command = [str(joy), action + "-artifact", str(whole / f"inputs/c{number}.dag"),
                            "--input", str(whole / f"inputs/c{number}-job.dag"), *profile_flags(profile)]
        expected_command += (["--output", "proof.joysc"] if action == "prove" else
                             ["--proof", str(proof), "--output", "compiler.dag", "--emit", "program"])
        require(row["command"] == expected_command and row["cwd"] == str(directory)
                and row["environment"] == {"PATH": ""}, "exact full producer/verifier invocation")
        require(set(row["files"]) == {"stdout", "stderr", "resources.jsonl", "proof.joysc" if action == "prove" else "compiler.dag"},
                "complete full command file inventory")
        files(whole / "inputs", row["inputs_before"])
        files(directory, row["files"])
    require(p["profile"] == v["profile"] == load(whole / "profile.json"), "profile differs")
    require(p["inputs_before"] == v["inputs_before"] == load(whole / "preparation.json")["files"],
            "complete same frozen package required")
    require(p["accepted_execution_coordinates"] == v["accepted_execution_coordinates"], "self-build coordinates")
    require(v["proof_input"]["path"] == str(proof), "fresh process consumed another certificate")
    same(proof, v["proof_input"])
    require(v["proof_input_after"] == p["files"]["proof.joysc"], "proof changed after verification")
    require(identity(compiler)["sha256"] == OUTPUT, "extracted compiler fixed point")
    pr, vr = load(producer / "stdout"), load(verifier / "stdout")
    require(pr["ok"] is True and vr["ok"] is True and pr["schema"] == "joy/artifact-proof/v1"
            and vr["schema"] == "joy/artifact-verification/v1", "versioned full command responses")
    require(semantic(pr["verification"]) == semantic(vr["verification"])
            and "prover_observations" not in vr["verification"], "fresh semantic/transport agreement")
    old = load(whole / f"inputs/accepted-c{number + 1}-step.json")["execution"]["execution"]
    result = vr["verification"]
    require(result["format"] == "joy-nox-disclosed-compiler-v1"
            and result["disclosure"] == "complete public witness"
            and result["physical_resource_claim"] == "unattested", "declared native public relation")
    require(all(result[key] == old[key] for key in ("program_particle", "input_particle", "output_particle", "charged_reductions", "compiler_job")),
            "actual accepted complete self-build relation")
    require(result["logical_peak_frames"] == old["peak_frames"]
            and result["expanded_steps"] == old["compaction"]["evaluator_checkpoints"] - 1, "logical execution coordinates")

    corpus_dir = base / f"whole-proof-corpus-v2/c{number + 1}"
    corpus = load(corpus_dir / "receipt.json")
    require(corpus["schema"] == "trident/proof-extracted-compiler-corpus/v2" and corpus["status"] == "passed"
            and corpus["proof_generation"] == number and corpus["generation"] == number + 1, "fresh extracted-compiler corpus")
    require(corpus["driver"]["sha256"] == PINS["whole-proof-corpus-v2/run.py"]
            and corpus["original_runner"]["sha256"] == PINS["whole-acceptance/trident/audit/self-hosting/bootstrap-runner.py"], "unchanged corpus drivers")
    fixture = base / "corpus-path-fix/trident"
    helpers = fixture / "audit/self-hosting"
    require(corpus["fixture_revision"] == CORPUS_FIXTURE_REVISION
            and corpus["joy_fixture_revision"] == "6e0ec4d8440e2521df08f442d64f54e667044716"
            and corpus["fixture_repository"] == str(fixture) and corpus["joy_fixture_repository"] == str(fixture.parent / "joy"),
            "exact recorded clean fixture checkouts")
    git_binary = Path(corpus["runtime_environment"]["TRIDENT_AUDIT_GIT"])
    require(git_binary.is_absolute(), "absolute metadata Git")
    require(corpus["runtime_environment"] == {"PATH": "", "PYTHONUTF8": "1", "PYTHONDONTWRITEBYTECODE": "1", "TRIDENT_AUDIT_GIT": str(git_binary)}
            and corpus["limits"] == dict(per_corpus_wall_seconds=5400, evidence_files=50000, evidence_bytes=4 * 1024**3), "unchanged corpus environment and resource profile")
    same(verifier / "receipt.json", corpus["verified_receipt"])
    expected_inputs = {str(path) for path in (compiler, joy, verifier / "receipt.json", verifier / "stdout",
                                            fixture.parent / "joy/cli/tests/compiler_vectors.json", git_binary)}
    require(corpus["inputs_before"] == corpus["inputs_after"] and set(corpus["inputs_after"]) == expected_inputs,
            "all six actual extracted-compiler corpus inputs including metadata Git")
    for path, expected in corpus["inputs_after"].items():
        same(Path(path), expected)
    same(corpus_dir / "files.json", corpus["files"])
    require(load(corpus_dir / "files.json") == bootstrap.evidence_files(corpus_dir), "complete retained corpus file inventory")
    require(len(corpus["corpora"]) == len(bootstrap.CORPORA) and corpus["observations"] == sum(bootstrap.CORPORA.values()), "all regression observations")
    for script, count in bootstrap.CORPORA.items():
        artifact = corpus["corpora"][f"c{number + 1}-{script.removesuffix('.py')}"]
        path = within(corpus_dir, artifact["path"])
        same(path, artifact)
        bootstrap.corpus_result(load(path), compiler, joy, count)
    require(len(corpus["commands"]) == len(bootstrap.CORPORA)
            and all(row["status"] == "completed" and row["exit_code"] == 0 for row in corpus["commands"]), "complete corpus child processes")
    orchestration = load(base / "whole-proof-corpus-v2/orchestration/receipt.json")
    launch = [row for row in orchestration["generations"] if row["proof_generation"] == number]
    require(len(launch) == 1 and launch[0]["status"] == "passed" and launch[0]["exit_code"] == 0, "actual successful corpus launch")
    corpus_shared = base / "whole-proof-corpus-v2"
    corpus_review = load(corpus_shared / "independent-review.json")
    require(orchestration["status"] == "passed" and corpus_review["status"] == "passed-source-review"
            and orchestration["reviewed_sources"] == corpus_review["sources"] == load(corpus_shared / "sources.json"), "reviewed exact corpus orchestration")
    require(orchestration["independent_review_sha256"] == identity(corpus_shared / "independent-review.json")["sha256"], "corpus launch review binding")
    for path, expected in corpus_review["sources"].items():
        same(Path(path), expected)
    generated = load(corpus_dir / f"corpora/c{number + 1}-check-generated-compiler-profile.json")
    require(generated["metadata_git"] == dict(path=str(git_binary), sha256=identity(git_binary)["sha256"]), "actual metadata Git identity")
    require([row["command"] for row in generated["commands"][:2]] == [[str(git_binary), "rev-parse", "HEAD"], [str(git_binary), "status", "--porcelain=v1"]], "exact metadata commands")
    python = launch[0]["command"][0]
    require(Path(python).is_absolute() and launch[0]["command"] == [python, "-B", str(base / "whole-proof-corpus-v2/run.py"),
            "--generation", str(number), "--fixtures", str(fixture), "--revision", corpus["fixture_revision"]], "exact corpus wrapper invocation")
    for row, script in zip(corpus["commands"], bootstrap.CORPORA):
        label = f"c{number + 1}-{script.removesuffix('.py')}"
        expected_command = [python, str(helpers / "bootstrap-runner.py"), "--corpus-child", str(helpers / script),
                            "--retain", str(corpus_dir / "corpora" / (label + "-files")), "--joy", str(joy),
                            "--compiler", str(compiler), "--output", str(corpus_dir / "corpora" / (label + ".json"))]
        require(row["command"] == expected_command and row["cwd"] == str(fixture), "exact supplied-compiler corpus invocation")
    return dict(generation=number, producer_receipt=identity(producer / "receipt.json"),
                verifier_receipt=identity(verifier / "receipt.json"), proof=identity(proof), compiler=identity(compiler),
                prove_seconds=p["elapsed_seconds"], verify_seconds=v["elapsed_seconds"],
                sampled_prove_rss_bytes=p["sampled_peak_rss_bytes"], sampled_verify_rss_bytes=v["sampled_peak_rss_bytes"],
                verification=result, corpus_receipt=identity(corpus_dir / "receipt.json"),
                corpus_observations=corpus["observations"])
