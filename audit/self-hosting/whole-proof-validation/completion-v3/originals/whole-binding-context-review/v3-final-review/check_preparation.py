"""Read-only final source/evidence admission; never starts a proof runner."""
import hashlib
import json
from pathlib import Path
import stat

BASE = Path(__file__).resolve().parents[2]
ROOT = BASE / "whole-proof-attacks-completion-v3"


def load(path):
    return json.loads(Path(path).read_text())


def identity(path):
    path = Path(path)
    assert stat.S_ISREG(path.lstat().st_mode), path
    with path.open("rb") as source:
        digest = hashlib.file_digest(source, "sha256").hexdigest()
    return dict(bytes=path.stat().st_size, sha256=digest)


def check():
    assert identity(ROOT / "sources.json")["sha256"] == "e65580aa1fff39f3c1b28560d5e64c5675f79f75c1119a0de8809d056b8c08aa"
    assert identity(ROOT / "transition.json")["sha256"] == "497a39329cc9b6e7bc4de733cb5aa8d106cd09fa953ea45883990e690e2c7ecc"
    sources = load(ROOT / "sources.json")
    assert len(sources) == 33
    for path, expected in sources.items():
        assert identity(path) == expected, path
    plan = load(ROOT / "plan.json")
    old = load(BASE / "whole-proof-attacks-parallel-v2/plan.json")
    for key in ("proofs", "shared_disk_bytes", "free_floor_bytes", "combined_sampled_rss_bytes",
                "mutant_growth_ceiling_bytes", "canonical_and_output_bucket_per_generation",
                "metadata_bucket_per_generation", "per_stream_log_bytes", "readiness_seconds", "outer_schedule_seconds"):
        assert plan[key] == old[key], key
    assert plan["retained_scopes"] == ["whole-proof-attacks", "whole-proof-attacks-v2-c1",
                                        "whole-proof-attacks-v2-c2", "whole-proof-attacks-parallel-v2"]
    assert (plan["fresh_rejections_per_generation"], plan["replayed_rejections_per_generation"],
            plan["replayed_controls_per_generation"]) == (14, 9, 2)
    assert plan["generations"] == [1, 2]
    for name in ("guard.py", "whole_suite.py"):
        assert identity(BASE / "whole-proof-attacks-v3-c1" / name) == identity(BASE / "whole-proof-attacks-v3-c2" / name)
    copied = load(ROOT / "copied-inputs.json")
    copied_count = 0
    for generation, rows in copied.items():
        assert len(rows) == 1080
        scope = BASE / plan["roots"][generation]
        assert not (scope / "attempts").exists() and not (scope / f"whole-c{generation}").exists()
        for name, entry in rows.items():
            expected = {k: entry[k] for k in ("bytes", "sha256")}
            assert identity(entry["original"]) == expected
            if name in ("guard.py", "whole_suite.py"):
                assert str(scope / name) in sources
            else:
                assert identity(scope / name) == expected
            copied_count += 1
    transition = load(ROOT / "transition.json")
    assert transition["status"] == "prior-attempts-classified-and-quiescent"
    assert len(transition["files"]) == 208
    for path, expected in transition["files"].items():
        assert identity(path) == expected, path
    action_path = ROOT / "reclamation-action-v2/receipt.json"
    action = load(action_path)
    assert transition["reclamation_receipt"] == dict(path=str(action_path), **identity(action_path))
    assert action["status"] == "passed" and action["removed"] is True and action["original_failed_evidence_unchanged"] is True
    assert action["cwd"] == str(ROOT)
    review_path = ROOT / "reclamation-review-v2.json"
    assert action["command"][1:] == [str(ROOT / "reclaim_completed_v2.py"), "--review-sha256", identity(review_path)["sha256"]]
    assert action["source"] == identity(ROOT / "reclaim_completed_v2.py")
    assert action["plan"] == identity(ROOT / "reclamation-plan.json")
    assert action["review"] == identity(review_path)
    review = load(review_path)
    assert review["status"] == "passed-reclamation-source-review"
    assert review["sources"] == {name: identity(ROOT / name) for name in ("reclaim_completed_v2.py", "reclamation-plan.json")}
    prior_action = ROOT / "reclamation-action/receipt.json"
    assert transition["failed_reclamation_receipt"] == dict(path=str(prior_action), **identity(prior_action))
    assert load(prior_action)["status"] == "failed" and load(prior_action)["removed"] is False
    partials = transition["retained_partial_mutants"]
    assert len(partials) == 2
    for path, expected in partials.items():
        assert identity(path) == expected
    actual_partial_paths = {str(p) for name in plan["retained_scopes"]
                            for p in (BASE / name).glob("whole-c*/certificate-*.joysc")}
    assert actual_partial_paths == set(partials)
    reclaim_plan = load(ROOT / "reclamation-plan.json")
    assert not (BASE / reclaim_plan["target"]).exists()
    for generation in (1, 2):
        prior = load(BASE / f"whole-proof-attacks-v2-c{generation}/whole-c{generation}/receipt.json")
        assert prior["status"] == "failed" and len(prior["controls"]) == 2 and len(prior["rejections"]) == 9
    fixture = load(ROOT / "binding-fixture-acceptance.json")
    assert fixture["status"] == "passed" and fixture["context_helper"] == identity(ROOT / "binding_context.py")
    for key, name in (("fixture_review", "review.json"), ("fixture_receipt", "actual/receipt.json"), ("checker", "check_result.py")):
        assert fixture[key] == identity(BASE / "whole-binding-context-review" / name)
    root_test = BASE / "whole-reclamation-v2-root-review/receipt.json"
    validation = load(root_test)
    assert validation["status"] == "passed" and validation["exit_code"] == 0 and validation["sources_unchanged"] is True
    for path, expected in validation["sources"].items():
        assert identity(path) == expected, path
    for name, expected in validation["files"].items():
        assert identity(root_test.parent / name) == expected
    assert "Ran 75 tests" in (root_test.parent / "stderr").read_text()
    assert "warning" not in (root_test.parent / "stderr").read_text().lower()
    assert all(identity(path) == expected for path, expected in sources.items())
    return dict(status="passed-readonly-preparation-replay", source_files=len(sources),
                transition_files=len(transition["files"]), original_and_owned_copy_entries=copied_count,
                partials_rehashed=2, partial_bytes=sum(x["bytes"] for x in partials.values()),
                numeric_limits_unchanged=True, original_v2_suites_still_failed=True,
                complete_proof_rehash=False, compiler_or_verifier_executed=False,
                sources=identity(ROOT / "sources.json"), transition=identity(ROOT / "transition.json"),
                root_independent_tests=identity(root_test), source=identity(__file__))


if __name__ == "__main__":
    print(json.dumps(check(), indent=2))
