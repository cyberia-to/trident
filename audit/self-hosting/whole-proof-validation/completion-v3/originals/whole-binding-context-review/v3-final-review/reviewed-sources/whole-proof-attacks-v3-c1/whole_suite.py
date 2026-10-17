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
import binding_context
import prior_cases

ROOT = Path(__file__).resolve().parent
WHOLE = ROOT.parent / "whole-proof"
JOY = ROOT.parent / "production-install/installed/bin/joy"
HELPER = ROOT / "target-helper/release/whole-proof-mutator"
BINDINGS = ("compiler", "source", "dependency", "cfg", "job-limit")
MUTATIONS = (
    ("continuation", "semantic record: Key"),
    ("generation", "semantic record: Cache"),
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
    fresh_names = ["rebound-job-limit", *(mode for mode, _ in MUTATIONS)]
    require(prior["status"] == "passed-selected-cases" and
            [row["name"] for row in prior["rejections"]] == list(prior_cases.PRIOR_NAMES),
            "exact nine independently replayed cases")
    require([row["name"] for row in rejections] == fresh_names,
            "all fourteen distinct missing cases executed")
    require(len(prior["controls"]) == 2 and
            len(set(prior_cases.PRIOR_NAMES) | set(fresh_names)) == 23,
            "two replayed controls and all twenty-three distinct cases")


def load(path):
    return json.loads(Path(path).read_text())


def immutable_files():
    files = [ROOT / name for name in ("whole_suite.py", "guard.py", "preparation.json",
                                    "helper-build-2/receipt.json", "fixture-check/receipt.json")]
    files += [Path(binding_context.__file__), Path(prior_cases.__file__),
              shared.ROOT / "binding-fixture-acceptance.json"]
    files += sorted((ROOT / "helper").glob("*.rs"))
    files += [ROOT / "helper/Cargo.toml", ROOT / "helper/Cargo.lock", JOY, HELPER]
    files += [p for p in sorted((ROOT / "inputs").rglob("*")) if p.is_file()]
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
    preparation = load(ROOT / "preparation.json")
    require(preparation["status"] == "prepared" and preparation["sources_unchanged"], "alternate inputs ready")
    require(preparation["binary"] == identity(JOY), "same packing and production binary")
    require(load(ROOT / "fixture-check/receipt.json")["status"] == "passed", "actual pilot fixture guards")
    build = load(ROOT / "helper-build-2/receipt.json")
    require(build["status"] == "passed" and identity(HELPER) == build["binary"], "checked helper")
    require(all(identity(ROOT / "helper" / name) == value for name, value in build["sources"].items()),
            "helper source corresponds to tested binary")
    profile = load(WHOLE / "profile.json")
    original_identity = identity(proof)
    frozen = ROOT / "inputs/frozen"
    require(all(identity(frozen / name) == value for name, value in
                load(WHOLE / "preparation.json")["files"].items()), "owned copies match all frozen inputs")
    compiler, job = frozen / f"c{generation}.dag", frozen / f"c{generation}-job.dag"
    work = ROOT / f"whole-c{generation}"
    work.mkdir()
    index, noun, temporary = work / "index.json", work / "result.dag", work / "temporary.joysc"
    before = immutable_files()
    report = dict(schema="trident/whole-self-build-certificate-completion/v3", status="running",
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

    def verify(name, certificate, target_compiler, target_job, message, construction):
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
        require(certificate == temporary, "completion owns this sole temporary")
        shared.reclaim(temporary)

    save()
    try:
        fixture = load(shared.ROOT / "binding-fixture-acceptance.json")
        require(fixture["status"] == "passed" and
                fixture["context_helper"] == identity(Path(binding_context.__file__)),
                "exact admitted-context helper passed complete bounded fixture")
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
        mode = "job-limit"
        variant = preparation["generations"][str(generation)]["variants"][mode]
        target_compiler = frozen / f'c{variant["compiler_generation"]}.dag'
        target_job = ROOT / f"inputs/c{generation}/{mode}/job.dag"
        require(identity(target_compiler) == variant["compiler"] and identity(target_job) == variant["job"],
                "exact alternate request bytes")
        coordinates = binding_context.coordinates(target_compiler)
        prepared = preparation["compiler_coordinates"][str(variant["compiler_generation"])]
        require(all(coordinates[k] == prepared[k] for k in ("program_particle", "formula_particle")),
                "actual compiler coordinates equal admitted preparation")
        with target_job.open("rb") as stream:
            header = stream.read(40)
        require(len(header) == 40 and header[:8] == b"NOXDAG01", "exact JOB noun header")
        context = binding_context.derive(coordinates, variant["admission"], header[8:40].hex())
        rebuilt = recipe("rebound-" + mode, "rebind", context)
        verify("rebound-" + mode, temporary, target_compiler, target_job,
               "semantic record: Key", rebuilt)
        for mode, message in MUTATIONS:
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
