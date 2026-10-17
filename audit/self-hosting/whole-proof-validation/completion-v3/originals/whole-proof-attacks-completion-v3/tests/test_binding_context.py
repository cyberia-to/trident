"""Focused input-contract tests; actual authentication is exercised by run.py."""
import copy
from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import unittest
from binding_context import derive, particle


class ContextTests(unittest.TestCase):
    def setUp(self):
        self.coords = dict(program_particle="01" + "00" * 31,
                           formula_particle="02" + "00" * 31, profile=1)
        self.job = "03" + "00" * 31
        self.admission = dict(compiler_particle=self.coords["program_particle"],
                              job_particle=self.job,
                              limits=dict(reductions=99_999_999, evaluator_frames=65_535))

    def call(self):
        return derive(self.coords, self.admission, self.job)

    def test_admitted_limits_are_exact(self):
        self.assertEqual(self.call(), [self.coords["program_particle"],
                                      self.coords["formula_particle"], self.job,
                                      "1", "99999999", "65535"])

    def test_unsigned_positive_boundaries(self):
        for key, bits in (("reductions", 64), ("evaluator_frames", 32)):
            for value in (1, (1 << bits) - 1):
                self.admission["limits"][key] = value
                self.assertIn(str(value), self.call())

    def test_bad_limits_fail_closed(self):
        original = copy.deepcopy(self.admission)
        for key, bits in (("reductions", 64), ("evaluator_frames", 32)):
            for value in (0, -1, 1 << bits, True, 1.0, "1", None):
                self.admission = copy.deepcopy(original)
                self.admission["limits"][key] = value
                with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                    self.call()

    def test_profile_is_exact_integer_one(self):
        for profile in (0, 2, 255, True, "1", None):
            self.coords["profile"] = profile
            with self.subTest(profile=profile), self.assertRaises(ValueError):
                self.call()

    def test_source_compiler_coordinate_mismatch(self):
        self.admission["compiler_particle"] = self.job
        with self.assertRaisesRegex(ValueError, "compiler coordinate"):
            self.call()

    def test_source_job_coordinate_mismatch(self):
        self.admission["job_particle"] = self.coords["program_particle"]
        with self.assertRaisesRegex(ValueError, "JOB coordinate"):
            self.call()

    def test_particles_are_exact_canonical_bytes(self):
        for value in ("0" * 63, "g" * 64, "AB" * 32,
                      (0xFFFFFFFF00000001).to_bytes(8, "little").hex() + "00" * 24):
            with self.subTest(value=value), self.assertRaises(ValueError):
                particle(value)

    def test_missing_admission_coordinates_refused(self):
        del self.admission["compiler_particle"]
        with self.assertRaises(KeyError):
            self.call()


if __name__ == "__main__":
    unittest.main()
