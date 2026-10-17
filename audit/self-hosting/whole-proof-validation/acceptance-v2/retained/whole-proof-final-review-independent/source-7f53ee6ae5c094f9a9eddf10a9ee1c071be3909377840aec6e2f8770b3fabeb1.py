"""Replay complete local SH8 evidence without executing a compiler or verifier."""
import argparse
import datetime
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import traceback

PINS = {
    "whole-proof/run.py": "71d698dda99a2af3ed322be9b000a187638a3caa9a21d026597df89d139065d5",
    "whole-proof/profile.json": "285dd5bb549795eda00beda023b91af07a047b1995cda7b99fffe3c4fe5c4fa2",
    "whole-proof/preparation.json": "064a0bb97979a390fd476c41bdfd55513fdfc087e06233663eaf8318dcb00865",
    "whole-proof-attacks/whole_suite.py": "65acbc20993403fdf8dc732ffc0c9faa83fafc75bef19c436e60a94ba97962fb",
    "whole-proof-attacks/guard.py": "c98cd24f7a18f4fcfd7ba4bac964e8517407a8666cbf34d02fe449bec860d629",
    "whole-proof-attacks/preparation.json": "566fb26095b63950ed4ded6c79f80b6f78d8083d76eead197bd6156f105db3e8",
    "whole-proof-attacks/target-helper/release/whole-proof-mutator": "ba7ced8038f76aa4ae9754b26f238c4567468f0fa914d4aa6693386408bf904c",
    "whole-proof-attacks/helper-build-2/receipt.json": "1fa8c0de27e502c609142f74a5e856925c8f306230ec3fa4e5e0e3f25f5ca7dd",
    "whole-proof-corpus/run.py": "9181b8eb98768f362c4a235307d382a4bfe470030428cb7c64ca4779933aedab",
    "whole-acceptance/trident/audit/self-hosting/bootstrap-runner.py": "b1afcd9a51c4a92b5254ebb56bbcbe88f4ad81b9cdf98f607f2ff7140a7691fc",
    "whole-acceptance/joy/cli/tests/compiler_vectors.json": "4b118cd9c9f20764e46b16676d4cb63599dc40c7e8f3a40a47c4fe5962f752ad",
    "production-install/installed/bin/joy": "8f42591ece35f192ff6f2328a8360fe0f0959f48a173248b211cd0d8f4d984f9",
}
OUTPUT = "76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8"
BINDINGS = ("compiler", "source", "dependency", "cfg", "job-limit")
ERRORS = {
    "continuation": "semantic record: Key", "generation": "semantic record: Cache",
    "cost": "semantic terminal: Claim",
    "valid-output-payload": "certificate result identity mismatch",
    "valid-output-topology": "certificate result identity mismatch",
    "valid-output-payload-rebound": "semantic terminal: Claim",
    "valid-output-topology-rebound": "semantic terminal: Claim",
    "omit-terminal": "certificate read: failed to fill whole buffer",
    "drop-first": "certificate frame order/chain mismatch",
    "swap-first-two": "certificate frame order/chain mismatch",
    "omit-completion": "certificate completion: failed to fill whole buffer",
    "truncate-last-byte": "certificate completion: failed to fill whole buffer",
    "trailing-byte": "certificate completion: certificate trailing bytes",
}
ERRORS.update({"binding-" + name: "format/context mismatch" for name in BINDINGS})
ERRORS.update({"rebound-" + name: "semantic record: Key" for name in BINDINGS})


def require(value, message):
    if not value:
        raise ValueError(message)


def identity(path):
    path = Path(path)
    require(path.is_file() and not path.is_symlink(), "regular retained file: " + str(path))
    with path.open("rb") as source:
        digest = hashlib.file_digest(source, "sha256").hexdigest()
    return dict(bytes=path.stat().st_size, sha256=digest)


def load(path):
    return json.loads(Path(path).read_text())


def same(path, expected):
    require(identity(path) == {key: expected[key] for key in ("bytes", "sha256")},
            "retained identity differs: " + str(path))


def within(root, name):
    path = (root / name).resolve()
    require(path.is_relative_to(root.resolve()), "receipt path escapes its evidence")
    return path


def files(directory, recorded):
    for name, expected in recorded.items():
        same(within(directory, name), expected)


def command_receipt(directory, exit_code):
    row = load(directory / "receipt.json")
    require(row["status"] == "passed" and row["exit_code"] == exit_code
            and row.get("expected_exit", exit_code) == exit_code
            and "resource_stop" not in row, "bounded command did not pass: " + str(directory))
    require(row["inputs_before"] == row["inputs_after"], "command input mutation")
    require(set(row["files"]) == {"stdout", "stderr", "resources.jsonl"}, "complete diagnostic log inventory")
    files(directory, row["files"])
    return row


def semantic(row):
    return {k: v for k, v in row.items() if k not in ("elapsed_micros", "prover_observations")}


def profile_flags(profile):
    flags = list(profile["host_flags"])
    for flag, key in (("proof-bytes", "wire_bytes"), ("proof-decoded-bytes", "decoded_bytes"),
                      ("proof-records", "records"), ("proof-steps", "steps"), ("proof-cache-slots", "cache_slots")):
        flags.extend(("--" + flag, str(profile[key])))
    return flags


def diagnostic(directory, argv, expected_exit, metadata, inputs, kind):
    row = command_receipt(directory, expected_exit)
    require(row["schema"] == "trident/whole-proof-attack-command/v1"
            and row["argv"] == list(map(str, argv)) and row["metadata"] == metadata
            and row["cwd"] == str(directory) and row["environment"] == {"PATH": ""},
            "exact diagnostic command/context: " + str(directory))
    require(row["inputs_before"] == inputs, "exact diagnostic input identities")
    require(row["driver"]["sha256"] == PINS["whole-proof-attacks/guard.py"], "reviewed command guard")
    caps = dict(wall=1800, cpu=1800, rss=1024**3, file=24 * 1024**3 + 65536) if kind == "helper" else dict(wall=7500, cpu=7500, rss=6 * 1024**3, file=32 * 1024**2)
    require(row["caps"] == caps and row["sampled_scope_disk_cap"] == 26 * 1024**3
            and row["free_floor"] == 8 * 1024**3 and row["sample_interval_seconds"] == 1, "unchanged diagnostic resource contract")
    return row


def case_recipe(name, generation, preparation, frozen, attacks):
    compiler, job = frozen / f"c{generation}.dag", frozen / f"c{generation}-job.dag"
    mode, context = name, []
    if name.startswith(("binding-", "rebound-")):
        request = name.split("-", 1)[1]
        variant = preparation["generations"][str(generation)]["variants"][request]
        compiler = frozen / f'c{variant["compiler_generation"]}.dag'
        job = attacks / f"inputs/c{generation}/{request}/job.dag"
        same(compiler, variant["compiler"])
        same(job, variant["job"])
        if name.startswith("binding-"):
            mode = None
        else:
            mode = "rebind"
            coordinates = preparation["compiler_coordinates"][str(variant["compiler_generation"])]
            context = [coordinates["program_particle"], coordinates["formula_particle"],
                       variant["admission"]["job_particle"], "1", "20000000000", "65536"]
    return compiler, job, mode, context


def generation(base, number, bootstrap):
    whole, attacks = base / "whole-proof", base / "whole-proof-attacks"
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

    suite_path = attacks / f"whole-c{number}/receipt.json"
    suite = load(suite_path)
    require(suite["status"] == "passed" and suite["generation"] == number
            and suite["profile"] == p["profile"] and suite["original_proof"] == identity(proof), "whole adversarial suite")
    require(suite["immutable_inputs_before"] == suite["immutable_inputs_after"], "suite immutable source")
    for path, expected in suite["immutable_inputs_after"].items():
        same(Path(path), expected)
    require(len(suite["controls"]) == 2 and [r["name"] for r in suite["controls"]] == ["original-fresh-verification", "rechain"], "two controls required")
    original, rechain = suite["controls"]
    require(original["reused_existing_actual_control"] is True and original["receipt"] == str(verifier / "receipt.json"), "explicit original control reuse")
    same(verifier / "receipt.json", original["receipt_identity"])
    same(proof, original["certificate"])
    same(compiler, original["output"])
    require(len(suite["rejections"]) == len(ERRORS)
            and {r["name"] for r in suite["rejections"]} == set(ERRORS), "all distinct expected rejections")
    work = suite_path.parent
    helper = attacks / "target-helper/release/whole-proof-mutator"
    index, noun = work / "index.json", work / "result.dag"
    proof_identity = identity(proof)
    diagnostic(attacks / f"attempts/whole-c{number}-index",
               [helper, "index", proof, index, noun, proof_identity["sha256"]], 0,
               dict(generation=number, complete_proof=True),
               {str(p): identity(p) for p in (helper, proof)}, "helper")
    indexed = load(index)
    require(indexed["schema"] == "trident/whole-proof-mutation-index/v1"
            and indexed["source_sha256"] == proof_identity["sha256"] and indexed["source_bytes"] == proof_identity["bytes"]
            and indexed["records"] == result["records"] and indexed["decoded_bytes"] == result["transport"]["decoded_bytes"], "complete full-proof index")
    construction_inputs = {str(path): identity(path) for path in (helper, index, noun)}
    construction_inputs[str(proof)] = proof_identity
    preparation = load(attacks / "preparation.json")
    frozen = attacks / "inputs/frozen"
    files(frozen, p["inputs_before"])
    flags = profile_flags(profile)
    for row in [rechain, *suite["rejections"]]:
        negative = row["name"] != "rechain"
        name = row["name"]
        target_compiler, target_job, mode, context = case_recipe(name, number, preparation, frozen, attacks)
        certificate = work / ("certificate-" + name + ".joysc") if mode else proof
        output = work / (name + ".dag")
        directory = attacks / f"attempts/whole-c{number}-verify-{name}"
        require(within(attacks, row["verification_receipt"]) == directory / "receipt.json", "distinct named verifier evidence")
        joy = base / "production-install/installed/bin/joy"
        argv = [joy, "verify-artifact", target_compiler, "--input", target_job,
                "--proof", certificate, "--output", output, "--emit", "program", *flags]
        inputs = {str(path): identity(path) for path in (joy, target_compiler, target_job)}
        inputs[str(certificate)] = row["certificate"]
        if negative:
            argv.append("--force")
            inputs[str(output)] = row["protected_output"]
        diagnostic(directory, argv, 1 if negative else 0,
                   dict(generation=number, expected_error=ERRORS[name] if negative else None), inputs, "verify")
        if mode is None:
            require(row["certificate"] == proof_identity, "unaltered original certificate binding")
        if negative:
            require(row["error"] == ERRORS[row["name"]] and (directory / "stdout").stat().st_size == 0
                    and row["error"] in (directory / "stderr").read_text(), "specific negative rejection")
            same(output, row["protected_output"])
        else:
            require(row["certificate"] == proof_identity, "byte-identical chain control")
            same(output, identity(compiler))
            response = load(directory / "stdout")
            require(response["ok"] is True and response["schema"] == "joy/artifact-verification/v1"
                    and "prover_observations" not in response["verification"]
                    and semantic(response["verification"]) == semantic(result), "rechain verification")
        recipe = row["recipe"]
        require((recipe is None) == (mode is None), "required construction cannot be skipped")
        if mode is not None:
            construction_dir = attacks / f"attempts/whole-c{number}-construct-{name}"
            require(within(attacks, recipe["construction_receipt"]) == construction_dir / "receipt.json", "distinct construction evidence")
            diagnostic(construction_dir, [helper, "mutate", proof, index, noun, certificate, mode, *context], 0,
                       dict(generation=number, mode=mode), construction_inputs, "helper")
            require(recipe["certificate"] == row["certificate"] and recipe["mode"] == mode
                    and recipe["context"] == context, "exact construction request binding")
            construction_output = load(construction_dir / "stdout")
            require(construction_output["mode"] == mode and construction_output["wire_bytes"] == row["certificate"]["bytes"]
                    and construction_output["source_records"] == indexed["records"], "actual helper output dimensions")
            if mode in ("drop-first", "swap-first-two", "omit-completion", "truncate-last-byte", "trailing-byte"):
                require(construction_output["source_decoded_bytes"] == indexed["decoded_bytes"], "framing recipe decoded source")
            else:
                require(construction_output["source_sha256"] == proof_identity["sha256"], "semantic recipe original proof")
            sidecar_required = name.startswith("valid-output-")
            require(("canonical_output_sidecar" in recipe) == sidecar_required, "required canonical output evidence")
            if sidecar_required:
                require(within(attacks, recipe["canonical_output_sidecar"]["path"]) == certificate.with_suffix(".dag"), "canonical sidecar filename")
                same(within(attacks, recipe["canonical_output_sidecar"]["path"]), recipe["canonical_output_sidecar"])
    require({str(path): identity(path) for path in (helper, proof, index, noun)} == construction_inputs,
            "original proof and mutation inputs unchanged during evidence replay")

    corpus_dir = base / f"whole-proof-corpus/c{number + 1}"
    corpus = load(corpus_dir / "receipt.json")
    require(corpus["schema"] == "trident/proof-extracted-compiler-corpus/v1" and corpus["status"] == "passed"
            and corpus["proof_generation"] == number and corpus["generation"] == number + 1, "fresh extracted-compiler corpus")
    require(corpus["driver"]["sha256"] == PINS["whole-proof-corpus/run.py"]
            and corpus["original_runner"]["sha256"] == PINS["whole-acceptance/trident/audit/self-hosting/bootstrap-runner.py"], "unchanged corpus drivers")
    fixture = base / "whole-acceptance/trident"
    helpers = fixture / "audit/self-hosting"
    require(corpus["fixture_revision"] == "e57f2b4c6c6b7128e1cbbf80a77a0ccfcc8d33d7"
            and corpus["joy_fixture_revision"] == "6e0ec4d8440e2521df08f442d64f54e667044716"
            and corpus["fixture_repository"] == str(fixture) and corpus["joy_fixture_repository"] == str(fixture.parent / "joy"),
            "exact recorded clean fixture checkouts")
    require(corpus["runtime_environment"] == {"PATH": "", "PYTHONUTF8": "1", "PYTHONDONTWRITEBYTECODE": "1"}
            and corpus["limits"] == dict(per_corpus_wall_seconds=5400, evidence_files=50000, evidence_bytes=4 * 1024**3), "unchanged corpus environment and resource profile")
    same(verifier / "receipt.json", corpus["verified_receipt"])
    expected_inputs = {str(path) for path in (compiler, joy, verifier / "receipt.json", verifier / "stdout",
                                            fixture.parent / "joy/cli/tests/compiler_vectors.json")}
    require(corpus["inputs_before"] == corpus["inputs_after"] and set(corpus["inputs_after"]) == expected_inputs,
            "all five actual extracted-compiler corpus inputs")
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
    orchestration = load(base / "whole-proof-corpus/orchestration/receipt.json")
    launch = [row for row in orchestration["generations"] if row["proof_generation"] == number]
    require(len(launch) == 1 and launch[0]["status"] == "passed" and launch[0]["exit_code"] == 0, "actual successful corpus launch")
    python = launch[0]["command"][0]
    require(Path(python).is_absolute() and launch[0]["command"] == [python, "-B", str(base / "whole-proof-corpus/run.py"),
            "--generation", str(number), "--fixtures", str(fixture), "--revision", corpus["fixture_revision"]], "exact corpus wrapper invocation")
    for row, script in zip(corpus["commands"], bootstrap.CORPORA):
        label = f"c{number + 1}-{script.removesuffix('.py')}"
        expected_command = [python, str(helpers / "bootstrap-runner.py"), "--corpus-child", str(helpers / script),
                            "--retain", str(corpus_dir / "corpora" / (label + "-files")), "--joy", str(joy),
                            "--compiler", str(compiler), "--output", str(corpus_dir / "corpora" / (label + ".json"))]
        require(row["command"] == expected_command and row["cwd"] == str(fixture), "exact supplied-compiler corpus invocation")
    return dict(generation=number, producer_receipt=identity(producer / "receipt.json"),
                verifier_receipt=identity(verifier / "receipt.json"), proof=proof_identity, compiler=identity(compiler),
                prove_seconds=p["elapsed_seconds"], verify_seconds=v["elapsed_seconds"],
                sampled_prove_rss_bytes=p["sampled_peak_rss_bytes"], sampled_verify_rss_bytes=v["sampled_peak_rss_bytes"],
                verification=result, suite=identity(suite_path), controls=2, rejected_cases=len(ERRORS),
                corpus_receipt=identity(corpus_dir / "receipt.json"), corpus_observations=corpus["observations"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    base = args.base.resolve()
    report = dict(schema="trident/proved-self-build-local-review/v1", status="running", command=[sys.executable, *sys.argv],
                  checker=identity(Path(__file__)), generations=[],
                  scope="Local SH8 full-proof, adversarial and extracted-compiler regression evidence replay. Public complete-witness relation; no succinctness, zero knowledge, language-semantics preservation or release publication claim.")
    require(not args.output.exists(), "fresh review destination required")
    try:
        for name, expected in PINS.items():
            require(identity(base / name)["sha256"] == expected, "reviewed source/runtime pin: " + name)
        spec = importlib.util.spec_from_file_location("retained_bootstrap", base / "whole-acceptance/trident/audit/self-hosting/bootstrap-runner.py")
        bootstrap = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(bootstrap)
        for number in (1, 2):
            report["generations"].append(generation(base, number, bootstrap))
        require(report["generations"][0]["compiler"] == report["generations"][1]["compiler"], "C2 equals C3")
        report["status"] = "passed"
    except BaseException:
        report.update(status="failed", error=traceback.format_exc())
        raise
    finally:
        report["completed_utc"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
        with args.output.open("x") as destination:
            json.dump(report, destination, indent=2)
            destination.write("\n")
    print(json.dumps(dict(status=report["status"], generations=len(report["generations"]))))


if __name__ == "__main__":
    main()
