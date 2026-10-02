"""Check one complete self-build certificate after successful fresh verification."""
import argparse
import fcntl
import json
from pathlib import Path
import shutil
import signal
import time
import traceback

from guard import GIB, document, identity, run, shared
import prior_cases_v4 as prior_cases

ROOT = Path(__file__).resolve().parent
WHOLE = ROOT.parent / "whole-proof"
JOY = ROOT.parent / "production-install/installed/bin/joy"
PREVIOUS = ROOT.parent / f"whole-proof-attacks-v3-c{2}"
HELPER_ROOT = ROOT.parent / "whole-proof-helper-v4"
HELPER = HELPER_ROOT / "target-helper/release/whole-proof-mutator"
BINDINGS = ("compiler", "source", "dependency", "cfg", "job-limit")
MUTATIONS = (
    ("cost", "semantic terminal: Claim"),
    ("valid-output-payload", "certificate result identity mismatch"),
    ("valid-output-topology", "certificate result identity mismatch"),
    ("valid-output-payload-rebound", "semantic terminal: Claim"),
    ("valid-output-topology-rebound", "semantic terminal: Claim"),
    ("omit-terminal", "certificate read: failed to fill whole buffer"),
    ("drop-first", "certificate frame order/chain mismatch"),
    ("swap-first-two", "certificate frame order/chain mismatch"),
    ("omit-completion", "certificate completion: failed to fill whole buffer"),
    ("truncate-last-byte", "certificate completion: failed to fill whole buffer"),
    ("trailing-byte", "certificate completion: certificate trailing bytes"),
)


def require(value, message):
    if not value:
        raise ValueError(message)



def validate_case_matrix(prior, rejections):
    fresh_names = [mode for mode, _ in MUTATIONS]
    require(prior["status"] == "passed-selected-cases" and
            [row["name"] for row in prior["rejections"]] == list(prior_cases.PRIOR_NAMES),
            "exact twelve independently replayed cases")
    require([row["name"] for row in rejections] == fresh_names,
            "all eleven distinct missing cases executed")
    require(len(prior["controls"]) == 2 and
            len(set(prior_cases.PRIOR_NAMES) | set(fresh_names)) == 23,
            "two replayed controls and all twenty-three distinct cases")


def load(path):
    return json.loads(Path(path).read_text())


def immutable_files():
    files = [ROOT / "whole_suite.py", ROOT / "guard.py", PREVIOUS / "preparation.json",
             HELPER_ROOT / "build-1/receipt.json", HELPER_ROOT / "fixture-1/receipt.json",
             ROOT.parent / "whole-helper-digest-independent-review/independent-review.json",
             Path(prior_cases.__file__), JOY, HELPER]
    files += sorted((HELPER_ROOT / "helper").glob("*.rs"))
    files += [HELPER_ROOT / "helper/Cargo.toml", HELPER_ROOT / "helper/Cargo.lock"]
    files += [p for p in sorted((PREVIOUS / "inputs").rglob("*")) if p.is_file()]
    return {str(p): identity(p) for p in files}


def eligible(generation):
    producer = WHOLE / f"attempts/c{generation}-selfbuild-1"
    verifier = WHOLE / f"attempts/c{generation}-fresh-verification-1"
    records = [load(p / "receipt.json") for p in (producer, verifier)]
    for record, action in zip(records, ("prove", "verify")):
        require(record["status"] == "passed" and record["exit_code"] == 0,
                "complete successful producer and fresh verifier required")
        require(record["generation"] == generation and record["action"] == action,
                "actual whole-build generation and command")
        require(record["binary"] == identity(JOY) == record["binary_after"], "production binary")
        require(record["preparation"] == identity(WHOLE / "preparation.json"), "frozen preparation")
        require(record["profile_identity"] == identity(WHOLE / "profile.json"), "frozen profile")
        require(record["inputs_before"] == record["inputs_after"], "frozen inputs unchanged")
        require(all(identity(WHOLE / "inputs" / n) == v for n, v in record["inputs_before"].items()),
                "all original inputs still exact")
    proof = producer / "proof.joysc"
    expected = identity(proof)
    require(expected == records[0]["files"]["proof.joysc"] == records[1]["proof_input_after"],
            "same successful complete certificate")
    require(Path(records[1]["proof_input"]["path"]) == proof, "fresh verifier used this certificate")
    for directory, record in zip((producer, verifier), records):
        require(identity(directory / "stdout") == record["files"]["stdout"], "recorded raw response")
    proved, verified = (load(p / "stdout") for p in (producer, verifier))
    require(proved["ok"] is True and verified["ok"] is True, "successful versioned responses")
    require(proved["schema"] == "joy/artifact-proof/v1" and
            verified["schema"] == "joy/artifact-verification/v1", "versioned responses")
    expected_fields = {k: v for k, v in proved["verification"].items()
                       if k not in ("elapsed_micros", "prover_observations")}
    actual_fields = {k: v for k, v in verified["verification"].items() if k != "elapsed_micros"}
    require(expected_fields == actual_fields, "all fresh semantic and transport fields agree")
    expected_artifact = load(WHOLE / "profile.json")["expected_artifact_sha256"]
    require(identity(verifier / "compiler.dag")["sha256"] == expected_artifact,
            "exact accepted C2/C3 program")
    return proof, verifier, verified["verification"]


def flags(profile):
    result = list(profile["host_flags"])
    for flag, key in (("proof-bytes", "wire_bytes"), ("proof-decoded-bytes", "decoded_bytes"),
                      ("proof-records", "records"), ("proof-steps", "steps"),
                      ("proof-cache-slots", "cache_slots")):
        result.extend(("--" + flag, str(profile[key])))
    return result


def cancel(signum, _frame):
    raise InterruptedError("owned suite terminated: " + str(signum))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--generation", type=int, choices=(1, 2), required=True)
    args = parser.parse_args()
    signal.signal(signal.SIGTERM, cancel)
    # One mutation per generation under the shared coordinator reservation.
    require(shared.admitted(ROOT) == args.generation, "registered generation")
    with (ROOT / "whole-suite.lock").open("a+") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        execute(args.generation)


def execute(generation):
    proof, original_verifier, expected = eligible(generation)
    preparation = load(PREVIOUS / "preparation.json")
    require(preparation["status"] == "prepared" and preparation["sources_unchanged"], "alternate inputs ready")
    require(preparation["binary"] == identity(JOY), "same packing and production binary")
    require(load(HELPER_ROOT / "fixture-1/receipt.json")["status"] == "passed", "actual pilot fixture guards")
    build = load(HELPER_ROOT / "build-1/receipt.json")
    require(build["status"] == "passed" and identity(HELPER) == build["binary"], "checked helper")
    require(all(identity(HELPER_ROOT / "helper" / name) == value for name, value in build["sources"].items()),
            "helper source corresponds to tested binary")
    profile = load(WHOLE / "profile.json")
    original_identity = identity(proof)
    frozen = PREVIOUS / "inputs/frozen"
    require(all(identity(frozen / name) == value for name, value in
                load(WHOLE / "preparation.json")["files"].items()), "owned copies match all frozen inputs")
    compiler, job = frozen / f"c{generation}.dag", frozen / f"c{generation}-job.dag"
    work = ROOT / f"whole-c{generation}"
    work.mkdir()
    index, noun, temporary = work / "index.json", work / "result.dag", work / "temporary.joysc"
    before = immutable_files()
    report = dict(schema="trident/whole-self-build-certificate-completion/v4", status="running",
                  generation=generation, started_ns=time.time_ns(), original_proof=original_identity,
                  immutable_inputs_before=before, profile=profile, prior_cases=None, rejections=[])

    def save():
        document(work / "receipt.json", report)

    def recipe(name, mode, context=()):
        nonlocal temporary
        require(not temporary.exists(), "only one temporary mutation")
        temporary = work / ("certificate-" + name + ".joysc")
        require(not temporary.exists() and not temporary.with_suffix(".dag").exists(),
                "unique certificate and canonical output sidecar")
        require(not shared.stop_requested(), "shared reservation still active")
        require(shutil.disk_usage(ROOT).free >= shared.PLAN["free_floor_bytes"],
                "shared physical free disk floor")
        argv = [HELPER, "mutate", proof, index, noun, temporary, mode, *context]
        attempt = f"whole-c{generation}-construct-{name}"
        run(attempt, argv, "helper", [proof, index, noun], metadata={"generation": generation, "mode": mode})
        require(temporary.stat().st_size <= original_identity["bytes"] +
                shared.PLAN["mutant_growth_ceiling_bytes"], "stricter generation mutation reservation")
        result = dict(mode=mode, context=list(context), certificate=identity(temporary),
                      construction_receipt=f"attempts/{attempt}/receipt.json")
        sidecar = temporary.with_suffix(".dag")
        if sidecar.exists():
            result["canonical_output_sidecar"] = dict(path=str(sidecar.relative_to(ROOT)), **identity(sidecar))
        return result

    def verify(name, certificate, target_compiler, target_job, message, construction, retained=False):
        require(isinstance(message, str) and bool(message) and construction is not None,
                "completion executes explicit constructed negative cases only")
        output = work / (name + ".dag")
        output.write_bytes(b"protected output must remain unchanged\n")
        protected = identity(output)
        argv = [JOY, "verify-artifact", target_compiler, "--input", target_job,
                "--proof", certificate, "--output", output, "--emit", "program", *flags(profile), "--force"]
        attempt = f"whole-c{generation}-verify-{name}"
        run(attempt, argv, "verify", [target_compiler, target_job, certificate, output],
            expected_exit=1, metadata={"generation": generation, "expected_error": message})
        logs = ROOT / "attempts" / attempt
        require((logs / "stdout").stat().st_size == 0, "negative has no successful stdout")
        require(message in (logs / "stderr").read_text(), "intended rejection class: " + name)
        require(identity(output) == protected, "negative preserves existing output")
        report["rejections"].append(dict(name=name, certificate=identity(certificate),
            recipe=construction, verification_receipt=f"attempts/{attempt}/receipt.json",
            error=message, protected_output=protected))
        save()
        # Reclaim only this completed, classified case after its full receipt was retained.
        if not retained:
            require(certificate == temporary, "completion owns this sole temporary")
            shared.reclaim(temporary)

    save()
    try:
        prior = prior_cases.review(ROOT.parent, generation, proof, original_verifier, expected)
        report["prior_cases"] = prior
        save()
        for retained, target in ((prior["index"], index), (prior["result"], noun)):
            original = Path(retained["path"])
            require(identity(original) == {k: retained[k] for k in ("bytes", "sha256")},
                    "authenticated prior full index/result")
            shutil.copyfile(original, target)
            require(identity(target) == identity(original), "exact copied index/result")
        indexed = load(index)
        require(indexed["records"] == expected["records"] and
                indexed["decoded_bytes"] == expected["transport"]["decoded_bytes"], "complete index dimensions")
        if generation == 2:
            admission = prior_cases.admit_c2_cost(ROOT.parent, proof, original_verifier, expected)
            document(work / "retained-cost-admission.json", admission)
            retained = ROOT.parent / shared.PLAN["historical_cost"]
            verify("cost", retained, compiler, job, "semantic terminal: Claim",
                   dict(mode="cost", construction_reused=True, admission=admission), retained=True)
            document(work / "prelude-complete.json", dict(status="passed", generation=2,
                     suite_receipt=identity(work / "receipt.json"),
                     rejection=report["rejections"][0], admission=identity(work / "retained-cost-admission.json")))
        shared.wait_full_admission(ROOT)
        for mode, message in MUTATIONS:
            if generation == 2 and mode == "cost":
                continue
            rebuilt = recipe(mode, mode)
            verify(mode, temporary, compiler, job, message, rebuilt)
        validate_case_matrix(report["prior_cases"], report["rejections"])
        report["status"] = "passed-completion"
    except BaseException:
        report.update(status="failed", error=traceback.format_exc())
        raise
    finally:
        try:
            report["immutable_inputs_after"] = immutable_files()
            require(report["immutable_inputs_after"] == before and identity(proof) == original_identity,
                    "original certificate and all inputs unchanged")
        except BaseException:
            report.update(status="input-changed", input_error=traceback.format_exc())
        report["ended_ns"] = time.time_ns()
        save()
    require(report["status"] == "passed-completion", "completion suite failed")
    print(json.dumps({k: report[k] for k in ("status", "generation")}))


if __name__ == "__main__":
    main()
