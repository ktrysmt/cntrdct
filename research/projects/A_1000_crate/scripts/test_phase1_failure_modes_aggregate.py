"""Unit tests for phase1_failure_modes_aggregate.py."""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from phase1_failure_modes import FAILURE_MODES_BY_DETECTOR
from phase1_failure_modes_aggregate import (
    EMPTY_DISPLAY,
    OTHER,
    _excerpt,
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
            "id,detector_id,consensus_label,failure_mode,failure_mode_notes\n"
            "1,clone-drift,FP,boilerplate-shape-only,\n"
            "2,arg-swap,FP,commutative-callee,min in callee\n"
            "3,clone-drift,TP,,\n"
            "4,comment-code,FP,,round 1 only\n"
            "5,arg-swap,FP,other,need new mode\n",
        )
        rows = parse_rows(p)
        self.assertEqual(len(rows), 5)
        self.assertEqual(rows[0].failure_mode, "boilerplate-shape-only")
        self.assertEqual(rows[3].failure_mode, "")
        self.assertEqual(rows[4].failure_mode, "other")

    def test_invalid_failure_mode_for_detector_raises(self):
        p = _write(
            self,
            "id,detector_id,consensus_label,failure_mode,failure_mode_notes\n"
            "1,clone-drift,FP,commutative-callee,\n",
        )
        with self.assertRaisesRegex(ValueError, "not permitted for"):
            parse_rows(p)

    def test_unknown_detector_with_named_mode_raises(self):
        p = _write(
            self,
            "id,detector_id,consensus_label,failure_mode,failure_mode_notes\n"
            "1,nonsense-detector,FP,boilerplate-shape-only,\n",
        )
        with self.assertRaisesRegex(ValueError, "not in controlled"):
            parse_rows(p)

    def test_other_and_empty_accepted_for_unknown_detector(self):
        p = _write(
            self,
            "id,detector_id,consensus_label,failure_mode,failure_mode_notes\n"
            "1,nonsense-detector,FP,other,sketch\n"
            "2,nonsense-detector,FP,,\n",
        )
        rows = parse_rows(p)
        self.assertEqual(len(rows), 2)
        self.assertEqual(rows[0].failure_mode, "other")
        self.assertEqual(rows[1].failure_mode, "")

    def test_missing_required_column_raises(self):
        p = _write(
            self,
            "id,detector_id,consensus_label,failure_mode\n"
            "1,clone-drift,FP,other\n",
        )
        with self.assertRaisesRegex(ValueError, "missing required"):
            parse_rows(p)


class RenderTests(unittest.TestCase):
    def test_empty_input_emits_meta_and_no_fp(self):
        p = _write(
            self,
            "id,detector_id,consensus_label,failure_mode,failure_mode_notes\n",
        )
        md = render_summary(parse_rows(p), p)
        self.assertIn("Phase 1 FP failure-modes summary", md)
        self.assertIn("total_rows: 0", md)
        self.assertIn("fp_rows: 0", md)
        self.assertIn("(no FP rows)", md)
        self.assertIn(
            "(no rows with failure_mode = other)", md
        )

    def test_skips_non_fp_rows(self):
        p = _write(
            self,
            "id,detector_id,consensus_label,failure_mode,failure_mode_notes\n"
            "1,clone-drift,TP,,\n"
            "2,clone-drift,Uncertain,,\n"
            "3,clone-drift,,,\n"
            "4,clone-drift,FP,boilerplate-shape-only,\n",
        )
        md = render_summary(parse_rows(p), p)
        self.assertIn("total_rows: 4", md)
        self.assertIn("fp_rows: 1", md)
        self.assertIn("| boilerplate-shape-only | 1 |", md)

    def test_per_detector_crosstab_counts(self):
        p = _write(
            self,
            "id,detector_id,consensus_label,failure_mode,failure_mode_notes\n"
            "1,clone-drift,FP,boilerplate-shape-only,\n"
            "2,clone-drift,FP,boilerplate-shape-only,\n"
            "3,clone-drift,FP,metadata-only-drift,\n"
            "4,clone-drift,FP,,\n"
            "5,arg-swap,FP,commutative-callee,\n"
            "6,arg-swap,FP,other,sketch\n",
        )
        md = render_summary(parse_rows(p), p)
        self.assertIn("### clone-drift", md)
        self.assertIn("### arg-swap", md)
        self.assertIn("| boilerplate-shape-only | 2 |", md)
        self.assertIn("| metadata-only-drift | 1 |", md)
        self.assertIn(f"| {EMPTY_DISPLAY} | 1 |", md)
        self.assertIn("| commutative-callee | 1 |", md)
        self.assertIn("| other | 1 |", md)

    def test_other_rows_listed_with_excerpt_and_no_newlines(self):
        long_note = "first line\nsecond line\n" + ("x" * 250)
        p = _write(
            self,
            "id,detector_id,consensus_label,failure_mode,failure_mode_notes\n"
            f"42,arg-swap,FP,other,\"{long_note}\"\n",
        )
        md = render_summary(parse_rows(p), p)
        self.assertIn("| 42 | arg-swap |", md)
        self.assertNotIn("first line\nsecond line", md)
        self.assertIn("…", md)


class IntegrationTests(unittest.TestCase):
    def test_main_writes_default_summary_path(self):
        p = _write(
            self,
            "id,detector_id,consensus_label,failure_mode,failure_mode_notes\n"
            "1,clone-drift,FP,boilerplate-shape-only,\n"
            "2,arg-swap,FP,other,note\n",
        )
        rc = main([str(p)])
        self.assertEqual(rc, 0)
        out = p.with_name("phase1-failure-modes-summary.md")
        self.assertTrue(out.exists())
        body = out.read_text()
        self.assertIn("| boilerplate-shape-only | 1 |", body)
        self.assertIn("| 2 | arg-swap | note |", body)

    def test_main_returns_nonzero_on_parse_error(self):
        p = _write(
            self,
            "id,detector_id,consensus_label,failure_mode,failure_mode_notes\n"
            "1,clone-drift,FP,not-a-real-mode,\n",
        )
        rc = main([str(p)])
        self.assertEqual(rc, 1)


class VocabularyTests(unittest.TestCase):
    def test_vocabulary_covers_five_detectors_with_shared_mode(self):
        self.assertEqual(
            set(FAILURE_MODES_BY_DETECTOR),
            {
                "clone-drift",
                "arg-swap",
                "comment-code",
                "unreachable-after-terminator",
                "config-interaction",
            },
        )
        shared = "cross-file-context-resolved"
        self.assertIn(shared, FAILURE_MODES_BY_DETECTOR["clone-drift"])
        self.assertIn(shared, FAILURE_MODES_BY_DETECTOR["arg-swap"])
        self.assertNotIn(
            shared, FAILURE_MODES_BY_DETECTOR["comment-code"]
        )
        for modes in FAILURE_MODES_BY_DETECTOR.values():
            self.assertGreaterEqual(len(modes), 4)


class ExcerptTests(unittest.TestCase):
    def test_excerpt_short_text_unchanged(self):
        self.assertEqual(_excerpt("hello"), "hello")

    def test_excerpt_strips_and_truncates(self):
        s = "abcd" * 100
        out = _excerpt(s, limit=10)
        self.assertEqual(len(out), 10)
        self.assertTrue(out.endswith("…"))


if __name__ == "__main__":
    unittest.main()
