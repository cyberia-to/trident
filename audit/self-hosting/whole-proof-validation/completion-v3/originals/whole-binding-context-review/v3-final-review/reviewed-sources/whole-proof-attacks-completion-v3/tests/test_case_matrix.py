"""Exercise only pure terminal admission in both prepared generation modules."""
import copy
import importlib.util
from pathlib import Path
import sys
import unittest

BASE = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(BASE / "whole-proof-attacks-completion-v3"))
sys.path.insert(0, str(BASE / "whole-proof-attacks-v3-c1"))
import prior_cases


class CompletionMatrix(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.suites = []
        for generation in (1, 2):
            path = BASE / f"whole-proof-attacks-v3-c{generation}/whole_suite.py"
            spec = importlib.util.spec_from_file_location(f"pure_matrix_c{generation}", path)
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            cls.suites.append(module)

    def setUp(self):
        self.prior = dict(status="passed-selected-cases",
                          controls=[dict(name="original-fresh-verification"), dict(name="rechain")],
                          rejections=[dict(name=name) for name in prior_cases.PRIOR_NAMES])
        self.fresh = [dict(name="rebound-job-limit"),
                      *(dict(name=name) for name, _ in self.suites[0].MUTATIONS)]

    def accept(self):
        for suite in self.suites:
            suite.validate_case_matrix(self.prior, self.fresh)

    def reject(self):
        for suite in self.suites:
            with self.subTest(module=suite.__name__), self.assertRaises(ValueError):
                suite.validate_case_matrix(self.prior, self.fresh)

    def test_exact_disjoint_nine_plus_fourteen_accepted(self):
        self.accept()
        names = [r["name"] for r in self.prior["rejections"] + self.fresh]
        self.assertEqual(len(names), len(set(names)))
        self.assertEqual(len(names), 23)

    def test_missing_fresh_case_rejected(self):
        self.fresh.pop()
        self.reject()

    def test_duplicate_fresh_case_rejected(self):
        self.fresh[1] = copy.deepcopy(self.fresh[0])
        self.reject()

    def test_unknown_fresh_case_rejected(self):
        self.fresh[-1]["name"] = "unrelated-rejection"
        self.reject()

    def test_inherited_case_cannot_replace_fresh_case(self):
        self.fresh[0]["name"] = "binding-job-limit"
        self.reject()

    def test_unrecorded_case_order_rejected(self):
        self.fresh[0], self.fresh[1] = self.fresh[1], self.fresh[0]
        self.reject()

    def test_duplicate_prior_case_rejected(self):
        self.prior["rejections"][1] = copy.deepcopy(self.prior["rejections"][0])
        self.reject()

    def test_incomplete_prior_cases_rejected(self):
        self.prior["rejections"].pop()
        self.reject()

    def test_whole_suite_status_cannot_replace_selected_case_status(self):
        self.prior["status"] = "passed"
        self.reject()

    def test_missing_prior_control_rejected(self):
        self.prior["controls"].pop()
        self.reject()


if __name__ == "__main__":
    unittest.main(verbosity=2)
