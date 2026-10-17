"""Strict composite admission using tiny, explicitly synthetic receipt trees."""
import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import prior_cases
from prior_fixture import PriorFixture, write


class PriorCases(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.fixture = PriorFixture(Path(self.temp.name).resolve(), prior_cases)

    def tearDown(self):
        self.temp.cleanup()

    def edit(self, path, mutate):
        value = json.loads(path.read_text())
        mutate(value)
        write(path, value)

    def test_exact_prior_nine_and_two_controls_are_admitted(self):
        result = self.fixture.review()
        self.assertEqual(result["status"], "passed-selected-cases")
        self.assertEqual([r["name"] for r in result["rejections"]], list(prior_cases.PRIOR_NAMES))
        self.assertEqual(len(result["controls"]), 2)
        self.assertEqual(json.loads(self.fixture.suite_path.read_text())["status"], "failed")

    def test_changed_producer_eligibility_refused(self):
        self.edit(self.fixture.producer / "receipt.json", lambda r: r.update(status="failed"))
        with self.assertRaisesRegex(ValueError, "complete actual producer"):
            self.fixture.review()

    def test_changed_fresh_verifier_eligibility_refused(self):
        self.edit(self.fixture.verifier / "receipt.json", lambda r: r.update(exit_code=1))
        with self.assertRaisesRegex(ValueError, "complete actual producer"):
            self.fixture.review()

    def test_fresh_verifier_must_use_exact_proof(self):
        self.edit(self.fixture.verifier / "receipt.json", lambda r: r["proof_input"].update(path="another-proof"))
        with self.assertRaisesRegex(ValueError, "fresh process consumed"):
            self.fixture.review()

    def test_caller_result_is_independently_bound(self):
        result = dict(self.fixture.result, charged_reductions=8)
        with self.assertRaisesRegex(ValueError, "caller result equals"):
            self.fixture.review(result)

    def test_prior_names_cannot_be_duplicated(self):
        self.edit(self.fixture.suite_path, lambda r: r["rejections"].__setitem__(1, r["rejections"][0]))
        with self.assertRaisesRegex(ValueError, "exact nine"):
            self.fixture.review()

    def test_prior_cases_cannot_be_omitted(self):
        self.edit(self.fixture.suite_path, lambda r: r["rejections"].pop())
        with self.assertRaisesRegex(ValueError, "exact nine"):
            self.fixture.review()

    def test_failed_job_limit_case_cannot_be_inherited(self):
        self.edit(self.fixture.suite_path, lambda r: r["rejections"][-1].update(name="rebound-job-limit"))
        with self.assertRaisesRegex(ValueError, "exact nine"):
            self.fixture.review()

    def test_failed_suite_cannot_be_relabelled_passed(self):
        self.edit(self.fixture.suite_path, lambda r: r.update(status="passed"))
        with self.assertRaisesRegex(ValueError, "original failed v2"):
            self.fixture.review()

    def test_wrong_diagnostic_class_refused(self):
        self.edit(self.fixture.suite_path, lambda r: r["rejections"][0].update(error="some rejection"))
        with self.assertRaisesRegex(ValueError, "specific negative rejection"):
            self.fixture.review()

    def test_protected_output_mutation_refused(self):
        (self.fixture.work / "binding-compiler.dag").write_bytes(b"changed output")
        with self.assertRaisesRegex(ValueError, "retained identity differs"):
            self.fixture.review()

    def test_changed_constructed_context_refused(self):
        self.edit(self.fixture.suite_path, lambda r: r["rejections"][1]["recipe"]["context"].__setitem__(4, "19999999999"))
        with self.assertRaisesRegex(ValueError, "exact construction request"):
            self.fixture.review()

    def test_changed_actual_verifier_argv_refused(self):
        p = self.fixture.attacks / "attempts/whole-c1-verify-binding-source/receipt.json"
        self.edit(p, lambda r: r["argv"].__setitem__(r["argv"].index("--budget") + 1, "1"))
        with self.assertRaisesRegex(ValueError, "exact diagnostic command"):
            self.fixture.review()

    def test_pinned_checker_source_change_refused_before_import(self):
        p = self.fixture.base / "whole-proof-final-review-v2/check.py"
        p.write_bytes(p.read_bytes() + b"\n")
        with self.assertRaisesRegex(ValueError, "immutable original checker"):
            self.fixture.review()


if __name__ == "__main__":
    unittest.main(verbosity=2)
