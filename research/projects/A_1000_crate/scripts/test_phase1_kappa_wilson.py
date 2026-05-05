"""Unit tests for phase1_kappa_wilson.py."""

from __future__ import annotations

import math
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from phase1_kappa_wilson import (
    NON_UNCERTAIN,
    cohen_kappa,
    parse_rows,
    render_summary,
    wilson_ci,
)


class CohenKappaTests(unittest.TestCase):
    def test_perfect_agreement_returns_one(self):
        k = cohen_kappa(
            ["TP", "TP", "FP", "FP"], ["TP", "TP", "FP", "FP"], NON_UNCERTAIN
        )
        self.assertAlmostEqual(k, 1.0)

    def test_chance_agreement_returns_zero(self):
        k = cohen_kappa(
            ["TP", "TP", "FP", "FP"], ["TP", "FP", "TP", "FP"], NON_UNCERTAIN
        )
        self.assertAlmostEqual(k, 0.0)

    def test_total_disagreement_yields_minus_one(self):
        k = cohen_kappa(
            ["TP", "TP", "FP", "FP"], ["FP", "FP", "TP", "TP"], NON_UNCERTAIN
        )
        self.assertAlmostEqual(k, -1.0)

    def test_empty_input_returns_nan(self):
        self.assertTrue(math.isnan(cohen_kappa([], [], NON_UNCERTAIN)))

    def test_single_category_collapse_returns_nan(self):
        k = cohen_kappa(["TP", "TP"], ["TP", "TP"], NON_UNCERTAIN)
        self.assertTrue(math.isnan(k))

    def test_label_outside_categories_raises(self):
        with self.assertRaises(ValueError):
            cohen_kappa(["TP"], ["Uncertain"], NON_UNCERTAIN)


class WilsonTests(unittest.TestCase):
    def test_zero_denominator_returns_nan(self):
        for v in wilson_ci(0, 0):
            self.assertTrue(math.isnan(v))

    def test_known_interval_50_50(self):
        p, lo, hi = wilson_ci(50, 50)
        self.assertAlmostEqual(p, 0.5)
        self.assertAlmostEqual(lo, 0.4038, places=3)
        self.assertAlmostEqual(hi, 0.5962, places=3)

    def test_all_tp_lower_bound_strictly_below_one(self):
        p, lo, hi = wilson_ci(30, 0)
        self.assertAlmostEqual(p, 1.0)
        self.assertLess(lo, 1.0)
        self.assertGreater(lo, 0.85)
        self.assertAlmostEqual(hi, 1.0)

    def test_lower_bound_clamped_at_zero(self):
        _, lo, _ = wilson_ci(0, 5)
        self.assertEqual(lo, 0.0)


class ParseTests(unittest.TestCase):
    def _write(self, body: str) -> Path:
        td = Path(tempfile.mkdtemp())
        p = td / "labels.csv"
        p.write_text(body)
        self.addCleanup(lambda: [f.unlink() for f in td.iterdir()] and td.rmdir())
        return p

    def test_round_trip_minimal(self):
        p = self._write(
            "id,detector_id,rater1_label,rater2_label\n"
            "1,clone-drift,TP,TP\n"
            "2,clone-drift,TP,FP\n"
            "3,arg-swap,FP,FP\n"
            "4,arg-swap,Uncertain,FP\n"
        )
        rows = parse_rows(p)
        self.assertEqual(len(rows), 4)
        md = render_summary(rows, p)
        self.assertIn("Phase 1 inter-rater + precision summary", md)
        self.assertIn("| clone-drift |", md)
        self.assertIn("| arg-swap |", md)
        self.assertIn("rows_used_for_kappa: 3", md)
        self.assertIn("input_sha256: `", md)

    def test_invalid_label_raises(self):
        p = self._write(
            "id,detector_id,rater1_label,rater2_label\n"
            "1,clone-drift,YES,NO\n"
        )
        with self.assertRaises(ValueError):
            parse_rows(p)

    def test_missing_column_raises(self):
        p = self._write(
            "id,detector_id,rater1_label\n"
            "1,clone-drift,TP\n"
        )
        with self.assertRaises(ValueError):
            parse_rows(p)


if __name__ == "__main__":
    unittest.main()
