"""Unit tests for phase1_precision.py."""

from __future__ import annotations

import math
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from phase1_precision import (
    COUNTABLE,
    VALID_CONSENSUS,
    main,
    parse_rows,
    render_summary,
)


def _make_tempdir(case: unittest.TestCase) -> Path:
    td = Path(tempfile.mkdtemp())
    case.addCleanup(
        lambda: [f.unlink() for f in td.iterdir()] and td.rmdir()
    )
    return td


def _write(case: unittest.TestCase, body: str, name: str = "labels.csv") -> Path:
    td = _make_tempdir(case)
    p = td / name
    p.write_text(body)
    return p


class ParseTests(unittest.TestCase):
    def test_round_trip_minimal(self):
        p = _write(
            self,
            "id,detector_id,consensus_label\n"
            "1,clone-drift,TP\n"
            "2,clone-drift,FP\n"
            "3,arg-swap,TP\n"
            "4,arg-swap,Uncertain\n"
            "5,arg-swap,\n",
        )
        rows = parse_rows(p)
        self.assertEqual(len(rows), 5)
        labels = [r.consensus_label for r in rows]
        self.assertEqual(labels, ["TP", "FP", "TP", "Uncertain", ""])

    def test_invalid_consensus_label_raises(self):
        p = _write(
            self,
            "id,detector_id,consensus_label\n"
            "1,clone-drift,YES\n",
        )
        with self.assertRaises(ValueError):
            parse_rows(p)

    def test_missing_consensus_column_raises(self):
        p = _write(
            self,
            "id,detector_id\n"
            "1,clone-drift\n",
        )
        with self.assertRaises(ValueError):
            parse_rows(p)

    def test_missing_id_column_raises(self):
        p = _write(
            self,
            "detector_id,consensus_label\n"
            "clone-drift,TP\n",
        )
        with self.assertRaises(ValueError):
            parse_rows(p)

    def test_missing_detector_column_raises(self):
        p = _write(
            self,
            "id,consensus_label\n"
            "1,TP\n",
        )
        with self.assertRaises(ValueError):
            parse_rows(p)

    def test_extra_columns_ignored(self):
        p = _write(
            self,
            "id,detector_id,rater1_label,rater2_label,consensus_label,round\n"
            "1,clone-drift,TP,TP,TP,1\n"
            "2,arg-swap,TP,FP,TP,2\n",
        )
        rows = parse_rows(p)
        self.assertEqual(len(rows), 2)
        self.assertEqual(rows[0].detector_id, "clone-drift")
        self.assertEqual(rows[1].consensus_label, "TP")


class RenderTests(unittest.TestCase):
    def test_empty_input_emits_meta_block(self):
        p = _write(
            self,
            "id,detector_id,consensus_label\n",
        )
        rows = parse_rows(p)
        md = render_summary(rows, p)
        self.assertIn("Phase 1 per-detector precision summary", md)
        self.assertIn("total_rows: 0", md)
        self.assertIn("rows_with_consensus: 0", md)
        self.assertIn("input_sha256: `", md)

    def test_all_uncertain_yields_nan_precision(self):
        p = _write(
            self,
            "id,detector_id,consensus_label\n"
            "1,clone-drift,Uncertain\n"
            "2,clone-drift,Uncertain\n",
        )
        md = render_summary(parse_rows(p), p)
        self.assertIn("| clone-drift | 2 | 0 | 0 | 0 | NaN | NaN | NaN |", md)

    def test_all_empty_consensus_yields_nan_precision(self):
        p = _write(
            self,
            "id,detector_id,consensus_label\n"
            "1,clone-drift,\n"
            "2,clone-drift,\n",
        )
        md = render_summary(parse_rows(p), p)
        self.assertIn("| clone-drift | 2 | 0 | 0 | 0 | NaN | NaN | NaN |", md)

    def test_all_tp_perfect_precision(self):
        p = _write(
            self,
            "id,detector_id,consensus_label\n"
            "1,clone-drift,TP\n"
            "2,clone-drift,TP\n"
            "3,clone-drift,TP\n",
        )
        md = render_summary(parse_rows(p), p)
        self.assertIn("| clone-drift | 3 | 3 | 3 | 0 | 1.000 |", md)

    def test_all_fp_zero_precision(self):
        p = _write(
            self,
            "id,detector_id,consensus_label\n"
            "1,arg-swap,FP\n"
            "2,arg-swap,FP\n",
        )
        md = render_summary(parse_rows(p), p)
        self.assertIn("| arg-swap | 2 | 2 | 0 | 2 | 0.000 |", md)


class IntegrationTests(unittest.TestCase):
    def test_mixed_detectors_grouped_independently(self):
        p = _write(
            self,
            "id,detector_id,consensus_label\n"
            "1,clone-drift,TP\n"
            "2,clone-drift,TP\n"
            "3,clone-drift,FP\n"
            "4,clone-drift,Uncertain\n"
            "5,arg-swap,FP\n"
            "6,arg-swap,FP\n"
            "7,arg-swap,\n",
        )
        md = render_summary(parse_rows(p), p)
        self.assertIn("rows_with_consensus: 5", md)
        self.assertIn("| arg-swap | 3 | 2 | 0 | 2 | 0.000 |", md)
        self.assertIn("| clone-drift | 4 | 3 | 2 | 1 |", md)

    def test_main_writes_default_summary_path(self):
        p = _write(
            self,
            "id,detector_id,consensus_label\n"
            "1,clone-drift,TP\n"
            "2,clone-drift,FP\n",
        )
        rc = main([str(p)])
        self.assertEqual(rc, 0)
        out = p.with_name("phase1-precision-summary.md")
        self.assertTrue(out.exists())
        body = out.read_text()
        self.assertIn("| clone-drift | 2 | 2 | 1 | 1 | 0.500 |", body)


class ConstantsTests(unittest.TestCase):
    def test_valid_consensus_includes_empty(self):
        self.assertIn("", VALID_CONSENSUS)
        self.assertIn("TP", VALID_CONSENSUS)
        self.assertIn("FP", VALID_CONSENSUS)
        self.assertIn("Uncertain", VALID_CONSENSUS)

    def test_countable_excludes_uncertain_and_empty(self):
        self.assertEqual(set(COUNTABLE), {"TP", "FP"})

    def test_nan_format_is_three_decimals_or_NaN(self):
        from phase1_precision import _fmt

        self.assertEqual(_fmt(float("nan")), "NaN")
        self.assertEqual(_fmt(0.0), "0.000")
        self.assertEqual(_fmt(1.0), "1.000")
        self.assertTrue(math.isnan(float("nan")))


if __name__ == "__main__":
    unittest.main()
