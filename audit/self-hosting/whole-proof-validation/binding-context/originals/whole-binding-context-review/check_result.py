"""Replay retained bounded fixture identities and every actual command receipt."""
import json
from pathlib import Path
from guard import identity
from run import BINDINGS, MUTATIONS

ROOT = Path(__file__).resolve().parent


def load(path):
    return json.loads(Path(path).read_text())


def check():
    report = load(ROOT / "actual/receipt.json")
    assert report["status"] == "passed" and report["unchanged_inputs"] is True
    assert identity(ROOT / "sources.json") == report["sources"]
    for path, expected in load(ROOT / "sources.json").items():
        assert identity(path) == expected, path
    count = 0
    for path in sorted((ROOT / "attempts").glob("*/receipt.json")):
        r = load(path)
        assert r["status"] == "passed" and r["exit_code"] == r["expected_exit"]
        assert "resource_stop" not in r and r["environment"] == {"PATH": ""}
        assert r["cwd"] == str(path.parent)
        assert r["driver"] == identity(ROOT / "guard.py")
        assert r["inputs_before"] == r["inputs_after"]
        for source, expected in r["inputs_after"].items():
            assert identity(source) == expected, source
        assert set(r["files"]) == {"stdout", "stderr", "resources.jsonl"}
        for name, expected in r["files"].items():
            assert identity(path.parent / name) == expected
        assert r["caps"]["file"] == 64 << 20
        is_verify = r["argv"][1] == "verify-artifact"
        assert r["caps"]["wall"] == r["caps"]["cpu"] == (330 if is_verify else 150)
        assert r["elapsed_seconds"] <= r["caps"]["wall"] + 2
        assert r["sampled_peak_rss_bytes"] <= r["caps"]["rss"]
        assert "warning" not in (path.parent / "stderr").read_text().lower()
        count += 1
    assert count == 58
    assert [r["name"] for r in report["controls"]] == ["original", "rechain"]
    assert [r["mode"] for r in report["bindings"]] == list(BINDINGS)
    assert [r["name"] for r in report["mutations"]] == [x[0] for x in MUTATIONS]
    rows = [*report["controls"], *report["mutations"], *report["regressions"]]
    for row in report["bindings"]:
        rows += [row["direct"], row["rebound"]]
        assert row["context"][4:] == [str(row["admission"]["limits"]["reductions"]),
                                      str(row["admission"]["limits"]["evaluator_frames"])]
        assert row["direct"]["expected_error"] == "format/context mismatch"
        assert row["rebound"]["expected_error"] == "semantic record: Key"
    assert len(rows) == 29
    for row in rows:
        path = ROOT / row["receipt"]
        r = load(path)
        assert row["passed"] is True
        proof = Path(r["argv"][r["argv"].index("--proof") + 1])
        assert identity(proof) == row["certificate"]
        if row["expected_error"]:
            assert r["exit_code"] == 1
            assert (path.parent / "stdout").stat().st_size == 0
            assert row["expected_error"] in (path.parent / "stderr").read_text()
            output = Path(r["argv"][r["argv"].index("--output") + 1])
            assert identity(output) == row["protected_output"]
    assert len(report["regressions"]) == 2
    return dict(status="passed", command_receipts=count, fresh_verifier_commands=len(rows),
                controls=2, original_matrix_rejections=23, frame_rejections=2,
                old_hardcoded_reproductions=2, source=identity(__file__),
                fixture_receipt=identity(ROOT / "actual/receipt.json"))


if __name__ == "__main__":
    print(json.dumps(check(), indent=2))
