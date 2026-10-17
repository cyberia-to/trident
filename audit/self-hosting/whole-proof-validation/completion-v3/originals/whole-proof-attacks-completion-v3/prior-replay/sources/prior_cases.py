"""Replay completed cases from the failed immutable v2 attempt; never relabel it."""
import hashlib
import importlib.util
import json
from pathlib import Path

CHECKER_SHA = "503bb17b57ec2141788f1536b1750e875b5d1f8b108751e40e30d58ab2658c9c"
PRIOR_NAMES = tuple(name for kind in ("compiler", "source", "dependency", "cfg")
                    for name in ("binding-" + kind, "rebound-" + kind)) + ("binding-job-limit",)


def review(base, number, proof, verifier, result):
    base = Path(base).resolve()
    checker_path = base / "whole-proof-final-review-v2/check.py"
    if hashlib.sha256(checker_path.read_bytes()).hexdigest() != CHECKER_SHA:
        raise ValueError("immutable original checker changed")
    spec = importlib.util.spec_from_file_location("immutable_sh8_v2_checker", checker_path)
    C = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(C)
    for name, expected in C.PINS.items():
        C.require(C.identity(base / name)["sha256"] == expected, "original source pin: " + name)
    attacks = base / f"whole-proof-attacks-v2-c{number}"
    suite_path = attacks / f"whole-c{number}/receipt.json"
    suite = C.load(suite_path)
    C.require(suite["schema"] == "trident/whole-self-build-certificate-checks/v2"
              and suite["status"] == "failed" and suite["generation"] == number,
              "original failed v2 suite is retained")
    whole = base / "whole-proof"
    profile = C.load(whole / "profile.json")
    p = C.load(whole / f"attempts/c{number}-selfbuild-1/receipt.json")
    compiler = verifier / "compiler.dag"
    C.require(suite["profile"] == profile and suite["original_proof"] == C.identity(proof),
              "prior cases use exact original proof and profile")
    C.require(suite["immutable_inputs_before"] == suite["immutable_inputs_after"], "prior immutable inputs")
    for path, expected in suite["immutable_inputs_after"].items():
        C.same(Path(path), expected)
    C.require([row["name"] for row in suite["controls"]] == ["original-fresh-verification", "rechain"],
              "exact two completed controls")
    C.require([row["name"] for row in suite["rejections"]] == list(PRIOR_NAMES),
              "exact nine prior successful distinct cases, excluding failed rebound-job-limit")
    original, rechain = suite["controls"]
    C.require(original["reused_existing_actual_control"] is True and
              original["receipt"] == str(verifier / "receipt.json"), "original fresh control provenance")
    C.same(verifier / "receipt.json", original["receipt_identity"])
    C.same(proof, original["certificate"])
    C.same(compiler, original["output"])
    work = suite_path.parent
    ERRORS = C.ERRORS
    helper = attacks / "target-helper/release/whole-proof-mutator"
    index, noun = work / "index.json", work / "result.dag"
    proof_identity = C.identity(proof)
    C.diagnostic(attacks / f"attempts/whole-c{number}-index",
               [helper, "index", proof, index, noun, proof_identity["sha256"]], 0,
               dict(generation=number, complete_proof=True),
               {str(p): C.identity(p) for p in (helper, proof)}, "helper")
    indexed = C.load(index)
    C.require(indexed["schema"] == "trident/whole-proof-mutation-index/v1"
            and indexed["source_sha256"] == proof_identity["sha256"] and indexed["source_bytes"] == proof_identity["bytes"]
            and indexed["records"] == result["records"] and indexed["decoded_bytes"] == result["transport"]["decoded_bytes"], "complete full-proof index")
    construction_inputs = {str(path): C.identity(path) for path in (helper, index, noun)}
    construction_inputs[str(proof)] = proof_identity
    preparation = C.load(attacks / "preparation.json")
    frozen = attacks / "inputs/frozen"
    C.files(frozen, p["inputs_before"])
    flags = C.profile_flags(profile)
    for row in [rechain, *suite["rejections"]]:
        negative = row["name"] != "rechain"
        name = row["name"]
        target_compiler, target_job, mode, context = C.case_recipe(name, number, preparation, frozen, attacks)
        certificate = work / ("certificate-" + name + ".joysc") if mode else proof
        output = work / (name + ".dag")
        directory = attacks / f"attempts/whole-c{number}-verify-{name}"
        C.require(C.within(attacks, row["verification_receipt"]) == directory / "receipt.json", "distinct named verifier evidence")
        joy = base / "production-install/installed/bin/joy"
        argv = [joy, "verify-artifact", target_compiler, "--input", target_job,
                "--proof", certificate, "--output", output, "--emit", "program", *flags]
        inputs = {str(path): C.identity(path) for path in (joy, target_compiler, target_job)}
        inputs[str(certificate)] = row["certificate"]
        if negative:
            argv.append("--force")
            inputs[str(output)] = row["protected_output"]
        C.diagnostic(directory, argv, 1 if negative else 0,
                   dict(generation=number, expected_error=ERRORS[name] if negative else None), inputs, "verify")
        if mode is None:
            C.require(row["certificate"] == proof_identity, "unaltered original certificate binding")
        if negative:
            C.require(row["error"] == ERRORS[row["name"]] and (directory / "stdout").stat().st_size == 0
                    and row["error"] in (directory / "stderr").read_text(), "specific negative rejection")
            C.same(output, row["protected_output"])
        else:
            C.require(row["certificate"] == proof_identity, "byte-identical chain control")
            C.same(output, C.identity(compiler))
            response = C.load(directory / "stdout")
            C.require(response["ok"] is True and response["schema"] == "joy/artifact-verification/v1"
                    and "prover_observations" not in response["verification"]
                    and C.semantic(response["verification"]) == C.semantic(result), "rechain verification")
        recipe = row["recipe"]
        C.require((recipe is None) == (mode is None), "required construction cannot be skipped")
        if mode is not None:
            construction_dir = attacks / f"attempts/whole-c{number}-construct-{name}"
            C.require(C.within(attacks, recipe["construction_receipt"]) == construction_dir / "receipt.json", "distinct construction evidence")
            C.diagnostic(construction_dir, [helper, "mutate", proof, index, noun, certificate, mode, *context], 0,
                       dict(generation=number, mode=mode), construction_inputs, "helper")
            C.require(recipe["certificate"] == row["certificate"] and recipe["mode"] == mode
                    and recipe["context"] == context, "exact construction request binding")
            construction_output = C.load(construction_dir / "stdout")
            C.require(construction_output["mode"] == mode and construction_output["wire_bytes"] == row["certificate"]["bytes"]
                    and construction_output["source_records"] == indexed["records"], "actual helper output dimensions")
            if mode in ("drop-first", "swap-first-two", "omit-completion", "truncate-last-byte", "trailing-byte"):
                C.require(construction_output["source_decoded_bytes"] == indexed["decoded_bytes"], "framing recipe decoded source")
            else:
                C.require(construction_output["source_sha256"] == proof_identity["sha256"], "semantic recipe original proof")
            sidecar_required = name.startswith("valid-output-")
            C.require(("canonical_output_sidecar" in recipe) == sidecar_required, "required canonical output evidence")
            if sidecar_required:
                C.require(C.within(attacks, recipe["canonical_output_sidecar"]["path"]) == certificate.with_suffix(".dag"), "canonical sidecar filename")
                C.same(C.within(attacks, recipe["canonical_output_sidecar"]["path"]), recipe["canonical_output_sidecar"])
    C.require({str(path): C.identity(path) for path in (helper, proof, index, noun)} == construction_inputs,
            "original proof and mutation inputs unchanged during evidence replay")

    return dict(schema="trident/prior-complete-proof-cases/v1", status="passed-selected-cases",
                scope="Case-level replay only; immutable enclosing v2 suite remains failed; no fresh case execution",
                generation=number, original_suite=dict(path=str(suite_path), **C.identity(suite_path)),
                original_checker=dict(path=str(checker_path), **C.identity(checker_path)),
                original_proof=proof_identity, profile=profile,
                controls=suite["controls"], rejections=suite["rejections"],
                index=dict(path=str(index), **C.identity(index)),
                result=dict(path=str(noun), **C.identity(noun)))
