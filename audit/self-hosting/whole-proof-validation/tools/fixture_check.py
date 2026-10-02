"""Check whole-file tooling on an existing bounded compiler certificate first."""
import json
from pathlib import Path
from guard import document, identity, run

ROOT = Path(__file__).resolve().parent
PILOT = ROOT.parent / "compiler-pilot"
JOY = ROOT.parent / "production-install/installed/bin/joy"
HELPER = ROOT / "target-helper/release/whole-proof-mutator"


def require(ok, message):
    if not ok:
        raise ValueError(message)


def main():
    work = ROOT / "fixture-check"
    work.mkdir(exist_ok=False)
    proof = PILOT / "attempts/prove-aggregate-1/proof.joysc"
    compiler = PILOT / "inputs/compiler.dag"
    job = PILOT / "cases/aggregate/job.dag"
    original = json.loads((PILOT / "attempts/prove-aggregate-1/stdout").read_text())["verification"]
    require(identity(proof)["sha256"] == "1b379caa0f299549b32b5d8acb79e91c5b719db8ffd082890d66a34457075d64", "actual pilot proof")
    require(identity(JOY)["sha256"] == "8f42591ece35f192ff6f2328a8360fe0f0959f48a173248b211cd0d8f4d984f9", "installed production Joy")
    build = json.loads((ROOT / "helper-build-2/receipt.json").read_text())
    require(identity(HELPER) == build["binary"], "checked helper binary")
    index, noun = work / "index.json", work / "result.dag"
    run("fixture-index", [HELPER, "index", proof, index, noun, identity(proof)["sha256"]],
        "helper", [proof], metadata={"scope": "existing SH7 aggregate, no whole self-build certificate"})
    indexed = json.loads(index.read_text())
    require(indexed["records"] == original["records"], "record count")
    require(indexed["decoded_bytes"] == original["transport"]["decoded_bytes"], "decoded bytes")
    flags = json.loads((PILOT / "attempts/verify-aggregate-result-1/receipt.json").read_text())["argv"]
    flags = flags[flags.index("--budget"):]
    cases = [
        ("rechain", 0, ""),
        ("continuation", 1, "semantic record: Key"),
        ("generation", 1, "semantic record: Cache"),
        ("cost", 1, "semantic terminal: Claim"),
        ("valid-output-payload", 1, "certificate result identity mismatch"),
        ("valid-output-topology", 1, "certificate result identity mismatch"),
        ("valid-output-payload-rebound", 1, "semantic terminal: Claim"),
        ("valid-output-topology-rebound", 1, "semantic terminal: Claim"),
        ("omit-terminal", 1, "certificate read: failed to fill whole buffer"),
        ("drop-first", 1, "certificate frame order/chain mismatch"),
        ("swap-first-two", 1, "certificate frame order/chain mismatch"),
        ("omit-completion", 1, "certificate completion: failed to fill whole buffer"),
        ("truncate-last-byte", 1, "certificate completion: failed to fill whole buffer"),
        ("trailing-byte", 1, "certificate completion: certificate trailing bytes"),
    ]
    report = {"schema": "trident/whole-proof-tool-fixture-check/v1", "status": "running",
              "scope": "New helper against unchanged accepted SH7 aggregate certificate, no whole self-build certificate consumed",
              "source_proof": identity(proof), "helper": identity(HELPER), "driver": identity(__file__),
              "cases": []}
    document(work / "receipt.json", report)
    for mode, code, message in cases:
        altered = work / (mode + ".joysc")
        run("fixture-construct-" + mode, [HELPER, "mutate", proof, index, noun, altered, mode],
            "helper", [proof, index, noun])
        output = work / (mode + ".output")
        if code:
            output.write_bytes(b"existing result must remain unchanged\n")
        before = identity(output) if code else None
        argv = [JOY, "verify-artifact", compiler, "--input", job, "--proof", altered,
                "--output", output, *flags]
        if code:
            argv.append("--force")
        attempt = "fixture-verify-" + mode
        run(attempt, argv, "verify", [compiler, job, altered, *([output] if code else [])],
            expected_exit=code)
        logs = ROOT / "attempts" / attempt
        if code:
            require((logs / "stdout").stat().st_size == 0, "no successful response")
            require(message in (logs / "stderr").read_text(), "intended rejection: " + mode)
            require(identity(output) == before, "protected destination")
        else:
            require(identity(altered) == identity(proof), "byte-identical rebuilt control")
            result = json.loads((logs / "stdout").read_text())["verification"]
            for key, value in original.items():
                if key not in ("elapsed_micros", "prover_observations"):
                    require(result[key] == value, "positive control field: " + key)
            require(identity(output) == identity(noun), "complete output control")
        report["cases"].append({"mode": mode, "expected_exit": code, "identity": identity(altered),
                                 "receipt": f"attempts/{attempt}/receipt.json", "passed": True})
        document(work / "receipt.json", report)
    invalid = dict(indexed, terminal_offset=2**64 - 1)
    document(work / "overflow-index.json", invalid)
    bad = "fixture-reject-index-overflow"
    run(bad, [HELPER, "mutate", proof, work / "overflow-index.json", noun,
              work / "must-not-exist", "rechain"], "helper",
        [proof, work / "overflow-index.json", noun], expected_exit=1)
    require("index bounds" in (ROOT / "attempts" / bad / "stderr").read_text(), "overflow rejection")
    require(not (work / "must-not-exist").exists(), "overflow before publication")
    report.update(status="passed", index_overflow_rejected=True)
    document(work / "receipt.json", report)
    print(json.dumps({"status": "passed", "cases": len(cases), "index_overflow_rejected": True}))


if __name__ == "__main__":
    main()
