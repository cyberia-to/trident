"""Actual bounded constructor regression on the accepted SH7 aggregate proof."""
import copy
import json
from pathlib import Path
import shutil
import traceback

import guard
from binding_context import coordinates, derive
from guard import document, identity, run

ROOT = Path(__file__).resolve().parent
R2 = ROOT.parent
PILOT = R2 / "compiler-pilot"
ORIGINAL = R2 / "whole-proof-attacks"
JOY = R2 / "production-install/installed/bin/joy"
HELPER = ORIGINAL / "target-helper/release/whole-proof-mutator"
BINDINGS = ("compiler", "source", "dependency", "cfg", "job-limit", "job-frames")
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


def load(path):
    return json.loads(Path(path).read_text())


def require(value, message):
    if not value:
        raise ValueError(message)


def main():
    # The unchanged guard is copied into this isolated scope. Only these child
    # ceilings are narrowed for the 8.7 MB pilot; no whole proof is opened.
    for kind in guard.CAPS:
        guard.CAPS[kind]["file"] = 64 << 20
        guard.CAPS[kind]["wall"] = 330 if kind == "verify" else 150
        guard.CAPS[kind]["cpu"] = 330 if kind == "verify" else 150
    work = ROOT / "actual"
    work.mkdir(exist_ok=False)
    proof = PILOT / "attempts/prove-aggregate-1/proof.joysc"
    job = PILOT / "cases/aggregate/job.dag"
    compiler = PILOT / "inputs/compiler.dag"
    require(identity(proof)["sha256"] == "1b379caa0f299549b32b5d8acb79e91c5b719db8ffd082890d66a34457075d64", "accepted pilot proof")
    require(identity(JOY)["sha256"] == "8f42591ece35f192ff6f2328a8360fe0f0959f48a173248b211cd0d8f4d984f9", "installed Joy 6e production binary")
    build = load(ORIGINAL / "helper-build-2/receipt.json")
    require(build["status"] == "passed" and identity(HELPER) == build["binary"], "unchanged helper")
    require(all(identity(ORIGINAL / "helper" / name) == value for name, value in build["sources"].items()), "unchanged helper source")
    require(identity(ROOT / "guard.py") == identity(ORIGINAL / "guard.py"), "exact copied guard")
    source_paths = sorted(ROOT.glob("*.py")) + [JOY, HELPER, proof, job, compiler]
    source_paths += [ORIGINAL / "helper" / name for name in build["sources"]]
    source_paths += [PILOT / "cases/aggregate" / name for name in
                     ("package.json", "sample.tri", "bank/values.tri")]
    alternate_compiler = R2 / "whole-proof/inputs/c1.dag"
    source_paths.append(alternate_compiler)
    sources = {str(p): identity(p) for p in source_paths}
    document(ROOT / "sources.json", sources)
    report = dict(schema="trident/whole-binding-constructor-fixture/v1", status="running",
                  scope="Accepted SH7 aggregate fixture only; no whole certificate consumed, no SH8 acceptance",
                  sources=identity(ROOT / "sources.json"), controls=[], bindings=[], mutations=[], regressions=[])
    document(work / "receipt.json", report)
    try:
        flags = load(PILOT / "attempts/verify-aggregate-result-1/receipt.json")["argv"]
        flags = flags[flags.index("--budget"):]
        host = flags[:flags.index("--proof-bytes")]
        original_report = load(PILOT / "attempts/prove-aggregate-1/stdout")["verification"]
        original_admission = load(PILOT / "attempts/pack-aggregate/stdout")["package"]
        index, noun = work / "index.json", work / "result.dag"
        run("index", [HELPER, "index", proof, index, noun, identity(proof)["sha256"]], "helper", [proof])
        indexed = load(index)
        require(indexed["records"] == original_report["records"], "record count")
        require(indexed["decoded_bytes"] == original_report["transport"]["decoded_bytes"], "decoded bytes")

        def construct(name, mode, context=()):
            altered = work / (name + ".joysc")
            attempt = "construct-" + name
            run(attempt, [HELPER, "mutate", proof, index, noun, altered, mode, *context],
                "helper", [proof, index, noun], metadata=dict(mode=mode, context=list(context)))
            return altered

        def verify(name, candidate, program=compiler, input_job=job, error=None):
            output = work / (name + ".output")
            protected = None
            if error:
                output.write_bytes(b"protected output must remain unchanged\n")
                protected = identity(output)
            args = [JOY, "verify-artifact", program, "--input", input_job, "--proof", candidate,
                    "--output", output, *flags, *(["--force"] if error else [])]
            attempt = "verify-" + name
            run(attempt, args, "verify", [program, input_job, candidate, *([output] if error else [])],
                expected_exit=1 if error else 0, metadata=dict(expected_error=error))
            logs = ROOT / "attempts" / attempt
            if error:
                require((logs / "stdout").stat().st_size == 0, "no successful stdout: " + name)
                require(error in (logs / "stderr").read_text(), "exact error class: " + name)
                require(identity(output) == protected, "protected output unchanged: " + name)
            else:
                result = load(logs / "stdout")["verification"]
                require({k: v for k, v in result.items() if k != "elapsed_micros"} ==
                        {k: v for k, v in original_report.items()
                         if k not in ("elapsed_micros", "prover_observations")}, "complete positive fields")
                require(identity(output) == identity(noun), "complete positive result")
            return dict(name=name, certificate=identity(candidate), program=identity(program),
                        job=identity(input_job), expected_error=error, protected_output=protected,
                        receipt=str((logs / "receipt.json").relative_to(ROOT)), passed=True)

        report["controls"].append(verify("original", proof))
        rechain = construct("rechain", "rechain")
        require(identity(rechain) == identity(proof), "byte-identical rechain")
        report["controls"].append(verify("rechain", rechain))
        base = load(PILOT / "cases/aggregate/package.json")
        for mode in BINDINGS:
            directory = work / mode
            directory.mkdir()
            manifest = copy.deepcopy(base)
            changed_sources = []
            for module in manifest["modules"]:
                original = PILOT / "cases/aggregate" / module["file"]
                data = original.read_bytes()
                if mode == "source" and module["logical_path"] == "sample":
                    require(data.count(b"y:9,x:7") == 1, "unique source edit")
                    data = data.replace(b"y:9,x:7", b"y:8,x:7")
                if mode == "dependency" and module["logical_path"] == "bank.values":
                    require(data.count(b"// different foreign offsets") == 1, "unique dependency edit")
                    data = data.replace(b"// different foreign offsets", b"// different foreign offsetX")
                target = directory / module["file"]
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(data)
                require(len(data) == original.stat().st_size, "same source length")
                if identity(target) != identity(original):
                    changed_sources.append(dict(module=module["logical_path"], original=identity(original), altered=identity(target)))
            if mode == "cfg":
                manifest["options"]["cfg_flags"] = ["whole_proof_changed"]
            if mode == "job-limit":
                manifest["limits"]["reductions"] -= 1
            if mode == "job-frames":
                manifest["limits"]["evaluator_frames"] -= 1
            document(directory / "package.json", manifest)
            program = alternate_compiler if mode == "compiler" else compiler
            input_job = directory / "job.dag"
            run("pack-" + mode, [JOY, "pack-job", "--compiler", program, "--manifest", directory / "package.json",
                                "--output", input_job, *host], "pack",
                [program, directory / "package.json", *(directory / m["file"] for m in manifest["modules"])])
            admission = load(ROOT / "attempts" / ("pack-" + mode) / "stdout")["package"]
            require(admission["job_particle"] != original_admission["job_particle"], "different admitted JOB")
            require(admission["limits"] == manifest["limits"], "exact requested admitted limits")
            context = derive(coordinates(program), admission, input_job.read_bytes()[8:40].hex())
            direct = verify("binding-" + mode, proof, program, input_job, "format/context mismatch")
            rebound = construct("rebound-" + mode, "rebind", context)
            semantic = verify("rebound-" + mode, rebound, program, input_job, "semantic record: Key")
            report["bindings"].append(dict(mode=mode, context=context, admission=admission,
                                           source_changes=changed_sources, direct=direct, rebound=semantic))
            if mode in ("job-limit", "job-frames"):
                wrong = list(context)
                wrong[4 if mode == "job-limit" else 5] = str(base["limits"]["reductions" if mode == "job-limit" else "evaluator_frames"])
                old = construct("old-hardcoded-" + mode, "rebind", wrong)
                report["regressions"].append(verify("old-hardcoded-" + mode, old, program, input_job,
                                                     "format/context mismatch"))
            document(work / "receipt.json", report)
        for mode, error in MUTATIONS:
            altered = construct(mode, mode)
            report["mutations"].append(verify(mode, altered, error=error))
            document(work / "receipt.json", report)
        require(len(report["controls"]) == 2 and len(report["bindings"]) == 6 and
                len(report["mutations"]) == 13 and len(report["regressions"]) == 2, "complete fixture matrix")
        require(all(identity(p) == value for p, value in sources.items()), "all original inputs/source unchanged")
        report.update(status="passed", unchanged_inputs=True, original_matrix_rejections=23,
                      added_frame_coordinate_rejections=2, old_hardcoded_reproductions=2)
    except BaseException:
        report.update(status="failed", error=traceback.format_exc())
        raise
    finally:
        document(work / "receipt.json", report)
    print(json.dumps({k: report[k] for k in ("status", "original_matrix_rejections",
                     "added_frame_coordinate_rejections", "old_hardcoded_reproductions")}))


if __name__ == "__main__":
    main()
