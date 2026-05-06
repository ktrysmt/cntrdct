"""Unit tests for validate_phase1_csv.py."""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from validate_phase1_csv import format_errors, main, validate


def _write(case: unittest.TestCase, body: str) -> Path:
    td = Path(tempfile.mkdtemp())
    case.addCleanup(
        lambda: [f.unlink() for f in td.iterdir()] and td.rmdir()
    )
    p = td / "labels.csv"
    p.write_text(body)
    return p


HEADER_REQUIRED = "id,detector_id,rater1_label,rater2_label\n"
HEADER_FULL = (
    "id,detector_id,rater1_label,rater2_label,"
    "consensus_label,round,tiebreak_rater,"
    "failure_mode,failure_mode_notes\n"
)


class RequiredSchemaTests(unittest.TestCase):
    def test_minimum_valid_csv_passes(self):
        p = _write(
            self,
            HEADER_REQUIRED
            + "1,clone-drift,TP,TP\n"
            + "2,arg-swap,FP,FP\n",
        )
        self.assertEqual(validate(p), [])

    def test_missing_required_column_raises_R0(self):
        p = _write(
            self,
            "id,detector_id,rater1_label\n"
            "1,clone-drift,TP\n",
        )
        errs = validate(p)
        self.assertEqual(len(errs), 1)
        self.assertEqual(errs[0].rule, "R0")
        self.assertIn("rater2_label", errs[0].message)


class RowLevelTests(unittest.TestCase):
    def test_empty_id_or_detector_violates_R1(self):
        p = _write(
            self,
            HEADER_REQUIRED
            + ",clone-drift,TP,TP\n"
            + "2,,FP,FP\n",
        )
        errs = validate(p)
        rules = {e.rule for e in errs}
        self.assertIn("R1", rules)
        self.assertEqual(sum(1 for e in errs if e.rule == "R1"), 2)

    def test_duplicate_id_violates_R2(self):
        p = _write(
            self,
            HEADER_REQUIRED
            + "1,clone-drift,TP,TP\n"
            + "1,arg-swap,FP,FP\n",
        )
        errs = validate(p)
        self.assertTrue(any(e.rule == "R2" for e in errs))

    def test_invalid_rater_label_violates_R3(self):
        p = _write(
            self,
            HEADER_REQUIRED
            + "1,clone-drift,YES,TP\n"
            + "2,clone-drift,TP,NO\n",
        )
        errs = [e for e in validate(p) if e.rule == "R3"]
        self.assertEqual(len(errs), 2)


class OptionalColumnTests(unittest.TestCase):
    def test_invalid_consensus_label_violates_R4(self):
        p = _write(
            self,
            HEADER_FULL
            + "1,clone-drift,TP,FP,YES,2,,,\n",
        )
        errs = [e for e in validate(p) if e.rule == "R4"]
        self.assertEqual(len(errs), 1)

    def test_invalid_round_violates_R5(self):
        p = _write(
            self,
            HEADER_FULL
            + "1,clone-drift,TP,FP,TP,4,,,\n",
        )
        errs = [e for e in validate(p) if e.rule == "R5"]
        self.assertEqual(len(errs), 1)

    def test_round_3_without_tiebreaker_violates_R6(self):
        p = _write(
            self,
            HEADER_FULL
            + "1,clone-drift,TP,FP,TP,3,,,\n",
        )
        errs = [e for e in validate(p) if e.rule == "R6"]
        self.assertEqual(len(errs), 1)
        self.assertIn("empty tiebreak_rater", errs[0].message)

    def test_tiebreaker_without_round_3_violates_R6(self):
        p = _write(
            self,
            HEADER_FULL
            + "1,clone-drift,TP,FP,TP,2,bob,,\n",
        )
        errs = [e for e in validate(p) if e.rule == "R6"]
        self.assertEqual(len(errs), 1)

    def test_round_3_with_tiebreaker_passes(self):
        p = _write(
            self,
            HEADER_FULL
            + "1,clone-drift,TP,FP,TP,3,bob,,\n",
        )
        errs = [e for e in validate(p) if e.rule == "R6"]
        self.assertEqual(errs, [])

    def test_consensus_without_round_violates_R7(self):
        p = _write(
            self,
            HEADER_FULL
            + "1,clone-drift,TP,FP,TP,,,,\n",
        )
        errs = [e for e in validate(p) if e.rule == "R7"]
        self.assertEqual(len(errs), 1)

    def test_failure_mode_on_non_fp_violates_R8(self):
        p = _write(
            self,
            HEADER_FULL
            + "1,clone-drift,TP,TP,TP,2,,boilerplate-shape-only,\n",
        )
        errs = [e for e in validate(p) if e.rule == "R8"]
        self.assertEqual(len(errs), 1)

    def test_failure_mode_on_fp_passes_R8(self):
        p = _write(
            self,
            HEADER_FULL
            + "1,clone-drift,FP,FP,FP,2,,boilerplate-shape-only,\n",
        )
        errs = [e for e in validate(p) if e.rule == "R8"]
        self.assertEqual(errs, [])

    def test_other_without_notes_violates_R9(self):
        p = _write(
            self,
            HEADER_FULL
            + "1,clone-drift,FP,FP,FP,2,,other,\n",
        )
        errs = [e for e in validate(p) if e.rule == "R9"]
        self.assertEqual(len(errs), 1)

    def test_other_with_notes_passes_R9(self):
        p = _write(
            self,
            HEADER_FULL
            + "1,clone-drift,FP,FP,FP,2,,other,unclassified pattern\n",
        )
        errs = [e for e in validate(p) if e.rule == "R9"]
        self.assertEqual(errs, [])


class IntegrationTests(unittest.TestCase):
    def test_main_returns_zero_on_clean_input(self):
        p = _write(
            self,
            HEADER_REQUIRED + "1,clone-drift,TP,TP\n",
        )
        self.assertEqual(main([str(p)]), 0)

    def test_main_returns_one_on_validation_error(self):
        p = _write(
            self,
            HEADER_REQUIRED + "1,clone-drift,YES,TP\n",
        )
        self.assertEqual(main([str(p)]), 1)

    def test_main_returns_two_on_missing_input(self):
        td = Path(tempfile.mkdtemp())
        self.addCleanup(td.rmdir)
        self.assertEqual(main([str(td / "nope.csv")]), 2)

    def test_format_errors_includes_rule_id_and_line(self):
        from validate_phase1_csv import ValidationError

        out = format_errors(
            [
                ValidationError(line=5, rule="R3", message="bad label"),
                ValidationError(line=7, rule="R6", message="bad round"),
            ]
        )
        self.assertIn("line 5 [R3]: bad label", out)
        self.assertIn("line 7 [R6]: bad round", out)
        self.assertIn("2 validation error(s)", out)

    def test_format_errors_empty_returns_ok(self):
        self.assertEqual(format_errors([]), "OK\n")


if __name__ == "__main__":
    unittest.main()
