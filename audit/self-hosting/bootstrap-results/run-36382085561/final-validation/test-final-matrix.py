"""Negative acceptance guards and small archive fixtures; no synthetic CI success."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import zipfile

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("final_phase_validation", HERE / "verify-final-matrix.py")
V = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(V)


def write_json(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data) + "\n")


class ArchiveGuards(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.tree = self.root / "tree"
        self.tree.mkdir()
        (self.tree / "tiny.txt").write_bytes(b"fixture\n")
        self.archive = self.root / "fixture.zip"
        with zipfile.ZipFile(self.archive, "w") as archive:
            archive.writestr("tiny.txt", b"fixture\n")

    def test_restoration_preserves_exact_bytes(self):
        self.assertEqual(V.exact_tree(self.archive, self.tree), dict(files=1, raw_bytes=8))
        (self.tree / "tiny.txt").write_bytes(b"mutated\n")
        with self.assertRaisesRegex(ValueError, "exact restored ZIP bytes"):
            V.exact_tree(self.archive, self.tree)

    def test_additional_restored_files_are_rejected(self):
        (self.tree / "extra").write_bytes(b"")
        with self.assertRaisesRegex(ValueError, "exact restored file set"):
            V.exact_tree(self.archive, self.tree)

    def test_restored_symlinks_are_rejected_before_open(self):
        (self.tree / "alias").symlink_to(self.tree / "tiny.txt")
        with self.assertRaisesRegex(ValueError, "ordinary restored tree entry"):
            V.exact_tree(self.archive, self.tree)

    def test_expanded_bytes_are_bounded(self):
        with patch.object(V, "MAX_BYTES", 7):
            with self.assertRaisesRegex(ValueError, "expanded ZIP byte bound"):
                V.exact_tree(self.archive, self.tree)

    def test_directory_entries_are_bounded(self):
        (self.tree / "one").mkdir()
        (self.tree / "two").mkdir()
        with patch.object(V, "MAX_FILES", 1):
            with self.assertRaisesRegex(ValueError, "bounded restored tree entries"):
                V.exact_tree(self.archive, self.tree)

    def test_archive_symlinks_are_rejected(self):
        member = zipfile.ZipInfo("tiny.txt")
        member.create_system = 3
        member.external_attr = 0o120777 << 16
        with zipfile.ZipFile(self.archive, "w") as archive:
            archive.writestr(member, b"fixture\n")
        with self.assertRaisesRegex(ValueError, "ordinary ZIP member path"):
            V.exact_tree(self.archive, self.tree)

    def test_pinned_manifest_prevents_coordinated_archive_api_substitution(self):
        api = self.root / "original-api.json"
        write_json(api, {"scope": "unit-test fixture"})
        provenance = dict(zip=V.identity(self.archive), metadata=V.identity(api))
        manifest = dict(schema=1, id=17, name="fixture", run_id=V.RUN, head_sha=V.HEAD,
                        zip={k: provenance["zip"][k] for k in ("bytes", "sha256")}, metadata_sha256=V.sha(api))
        raw = (json.dumps(manifest) + "\n").encode()
        digest = hashlib.sha256(raw).hexdigest()
        path = self.root / "manifests" / (digest + ".json")
        path.parent.mkdir()
        path.write_bytes(raw)
        meta = self.root / "metadata" / (V.sha(api) + ".json")
        meta.parent.mkdir()
        shutil.copyfile(api, meta)
        entry = dict(id=17, manifest_sha256=digest)
        V.store_binding(self.root, entry, "fixture", provenance)
        # The index/manifest stay pinned while all unanchored inputs change together.
        with zipfile.ZipFile(self.archive, "w") as archive:
            archive.writestr("tiny.txt", b"mutated\n")
        (self.tree / "tiny.txt").write_bytes(b"mutated\n")
        write_json(api, {"scope": "substituted fixture", "digest": V.sha(self.archive)})
        V.exact_tree(self.archive, self.tree)
        substituted = dict(zip=V.identity(self.archive), metadata=V.identity(api))
        with self.assertRaisesRegex(ValueError, "store original ZIP binding"):
            V.store_binding(self.root, entry, "fixture", substituted)
        with self.assertRaisesRegex(ValueError, "store original API binding"):
            V.store_binding(self.root, entry, "fixture", dict(zip=provenance["zip"], metadata=substituted["metadata"]))
        meta.write_bytes(b"{}\n")
        with self.assertRaisesRegex(ValueError, "retained original API bytes"):
            V.store_binding(self.root, entry, "fixture", provenance)

    def test_job_success_cannot_be_relabelled_as_another_generation(self):
        path = self.root / "job-17-direct.json"
        name = "Bootstrap aarch64-apple-darwin repeat 1 / Corpus C2"
        job = dict(id=17, run_id=V.RUN, head_sha=V.HEAD, run_attempt=1, status="completed", conclusion="success", name=name)
        write_json(path, job)
        V.job_identity(path, name)
        with self.assertRaisesRegex(ValueError, "original successful job"):
            V.job_identity(path, name.replace("C2", "C3"))
        for key, value in (("run_attempt", 2), ("conclusion", "failure"), ("status", "in_progress")):
            write_json(path, job | {key: value})
            with self.subTest(key=key), self.assertRaises(ValueError):
                V.job_identity(path, name)

    def test_reused_job_id_cannot_count_as_an_independent_phase(self):
        selected = set()
        V.distinct_job(dict(id=17), selected)
        V.distinct_job(dict(id=18), selected)
        with self.assertRaisesRegex(ValueError, "distinct native job IDs"):
            V.distinct_job(dict(id=17), selected)

    def test_job_filename_and_positive_integer_id_are_bound(self):
        path = self.root / "job-17-direct.json"
        for job_id in (True, 0, -17, "17", 18):
            write_json(path, dict(id=job_id))
            with self.subTest(job_id=job_id), self.assertRaisesRegex(ValueError, "direct job file identity"):
                V.job_identity(path, "fixture")

    def test_equal_compilers_keep_distinct_receipt_roles(self):
        rows = [{"path": f"/input/{role}/receipt.json", "sha256": "a" * 64} for role in ("c2", "c3")]
        self.assertEqual(set(V.receipt_map(rows, 2)), {"c2", "c3"})
        with self.assertRaisesRegex(ValueError, "uniqueness"):
            V.receipt_map([rows[0], rows[0]], 2)
        rows[0]["path"] = "/input/c2/unrelated.json"
        with self.assertRaisesRegex(ValueError, "receipt role"):
            V.receipt_map(rows, 2)


class PinnedInputGuards(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.expected = INPUTS / "expected-inputs.json"
        self.runner = INPUTS / "frozen/audit/self-hosting"
        self.data = V.load(self.expected)
        self.assertEqual(V.sha(self.expected), V.EXPECTED)
        self.output = self.root / "output"
        self.stores = self.root / "stores"
        self.indices = self.root / "indices.json"
        index_hashes = {}
        for role in V.ROLES:
            path = self.stores / role / "index.json"
            write_json(path, dict(schema=1, run_id=V.RUN, head_sha=V.HEAD, artifacts={}))
            index_hashes[role] = V.sha(path)
        write_json(self.indices, index_hashes)

    def command(self):
        args = dict(restored=self.root / "restored", stores=self.stores, indices=self.indices,
                    inputs=INPUTS, expected=self.expected, output=self.output)
        args.update({"runner-directory": self.runner, "aggregate-job": self.root / "aggregate.json",
                     "final-run": self.root / "run.json", "indices-sha256": V.sha(self.indices), "aggregate-id": 1})
        return [sys.executable, "-B", *(["-O"] if sys.flags.optimize else []), "-W", "error",
                str(HERE / "verify-final-matrix.py"), *[item for key, value in args.items() for item in ("--" + key, str(value))]]

    def rejected(self, message, env=None):
        result = subprocess.run(self.command(), env=env, capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(message, result.stderr)
        self.assertFalse(self.output.exists(), "preflight failure must not publish output")

    def test_incomplete_stores_cannot_create_a_verdict(self):
        self.rejected("exact twelve artifact names per phase store")

    def test_missing_phase_store_is_rejected(self):
        data = V.load(self.indices)
        del data["c3"]
        write_json(self.indices, data)
        self.rejected("three distinct phase store roles")

    def test_frozen_contract_cannot_be_changed(self):
        self.expected = self.root / "changed.json"
        write_json(self.expected, self.data | {"run_attempt": 2})
        self.rejected("frozen expected source and implementation contract")

    def test_frozen_implementation_cannot_be_changed(self):
        copied = self.root / "runner"
        shutil.copytree(self.runner, copied)
        self.runner = copied
        with (copied / "bootstrap-phases.py").open("a") as stream:
            stream.write("\n# fixture mutation\n")
        self.rejected("frozen phase implementation")

    def test_ci_environment_cannot_be_used_for_local_replay(self):
        self.rejected("local replay must not impersonate CI", dict(os.environ, GITHUB_ACTIONS="true"))

    def test_output_cannot_overlap_retained_inputs(self):
        self.output = self.stores / "nested-output"
        self.rejected("input/output paths must be separate")

    def test_output_parent_symlink_cannot_hide_overlap(self):
        alias = self.root / "alias"
        alias.symlink_to(self.stores, target_is_directory=True)
        self.output = alias / "nested-output"
        self.rejected("input/output paths must be separate")

    def test_s1_binding_ignores_only_elapsed_time(self):
        for reference in self.data["local_reference_steps"]:
            path = REFERENCES / reference["file"]
            self.assertEqual(V.sha(path), reference["sha256"])
            step = V.load(path)
            with self.subTest(generation=reference["file"]):
                V.s1_binding(step, reference)
                step["execution"]["execution"]["elapsed_micros"] += 1
                V.s1_binding(step, reference)
                step["execution"]["execution"]["charged_reductions"] += 1
                with self.assertRaisesRegex(ValueError, "frozen S1 execution fields"):
                    V.s1_binding(step, reference)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inputs", type=Path, required=True)
    parser.add_argument("--references", type=Path, required=True)
    args, remaining = parser.parse_known_args()
    INPUTS = args.inputs.resolve()
    REFERENCES = args.references.resolve()
    unittest.main(argv=[sys.argv[0], *remaining])
