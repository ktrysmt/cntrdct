"""Unit tests for build_phase1_csv.py."""

from __future__ import annotations

import csv
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from build_phase1_csv import (
    ANCHOR_FIELDS_IN_FINDING,
    ANCHOR_FIELDS_IN_ITEM,
    BLIND_COLUMNS,
    extract_blind_row,
    extract_context_entry,
    load_findings,
    main,
    relativise,
)


def _sample_item(detector: str, file: str, line: int = 7) -> dict:
    return {
        "finding": {
            "detector_id": detector,
            "primary": {"file": file, "start_line": line},
            "evidence": {"citation_keys": ["nicad-2008", "rice-2017"]},
            "message": "synthetic finding for the test",
            "anomaly_class": "Logic",
        },
        "rank_score": 0.42,
        "posterior_tp": 0.81,
        "wilson_lower": 0.55,
    }


class LoadFindingsTests(unittest.TestCase):
    def test_loads_array(self):
        with tempfile.TemporaryDirectory() as td:
            p = Path(td) / "in.json"
            p.write_text(json.dumps([_sample_item("clone-drift", "/x.rs")]))
            result = load_findings(p)
            self.assertEqual(len(result), 1)
            self.assertEqual(result[0]["finding"]["detector_id"], "clone-drift")

    def test_rejects_non_array(self):
        with tempfile.TemporaryDirectory() as td:
            p = Path(td) / "in.json"
            p.write_text(json.dumps({"not": "an array"}))
            with self.assertRaises(ValueError):
                load_findings(p)


class ExtractBlindRowTests(unittest.TestCase):
    def test_omits_anchor_fields(self):
        item = _sample_item("clone-drift", "/x.rs")
        row = extract_blind_row(3, item, corpus_root=None)
        self.assertEqual(row["id"], 3)
        self.assertEqual(row["detector_id"], "clone-drift")
        self.assertEqual(row["file"], "/x.rs")
        self.assertEqual(row["line"], 7)
        self.assertEqual(row["citation_keys"], "nicad-2008;rice-2017")
        for col in (
            "rater1_label",
            "rater1_rubric",
            "rater1_notes",
            "rater2_label",
            "rater2_rubric",
            "rater2_notes",
        ):
            self.assertEqual(row[col], "")
        for forbidden in ("message", "anomaly_class", "rank_score"):
            self.assertNotIn(forbidden, row)

    def test_handles_missing_evidence(self):
        item = {
            "finding": {
                "detector_id": "arg-swap",
                "primary": {"file": "/y.rs", "start_line": 1},
            }
        }
        row = extract_blind_row(1, item, corpus_root=None)
        self.assertEqual(row["citation_keys"], "")
        self.assertEqual(row["detector_id"], "arg-swap")

    def test_corpus_root_relativises(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            (root / "crate-1.0.0/src").mkdir(parents=True)
            f = root / "crate-1.0.0/src/lib.rs"
            f.write_text("")
            item = _sample_item("clone-drift", str(f))
            row = extract_blind_row(1, item, corpus_root=root)
            self.assertEqual(row["file"], "crate-1.0.0/src/lib.rs")

    def test_corpus_root_falls_back_when_not_under_prefix(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            item = _sample_item("clone-drift", "/somewhere/else.rs")
            row = extract_blind_row(1, item, corpus_root=root)
            self.assertEqual(row["file"], "/somewhere/else.rs")


class ExtractContextEntryTests(unittest.TestCase):
    def test_collects_all_anchor_fields(self):
        item = _sample_item("clone-drift", "/x.rs")
        ctx = extract_context_entry(5, item)
        self.assertEqual(ctx["id"], 5)
        for field in ANCHOR_FIELDS_IN_FINDING:
            self.assertIn(field, ctx)
        for field in ANCHOR_FIELDS_IN_ITEM:
            self.assertIn(field, ctx)
        self.assertEqual(ctx["rank_score"], 0.42)
        self.assertEqual(ctx["message"], "synthetic finding for the test")

    def test_skips_missing_optional_fields(self):
        item = {
            "finding": {
                "detector_id": "arg-swap",
                "primary": {"file": "/y.rs", "start_line": 1},
            }
        }
        ctx = extract_context_entry(1, item)
        self.assertEqual(ctx, {"id": 1})


class IntegrationTests(unittest.TestCase):
    def test_main_round_trip(self):
        with tempfile.TemporaryDirectory() as td:
            tdp = Path(td)
            input_path = tdp / "sample.json"
            findings = [
                _sample_item("clone-drift", "/x.rs"),
                _sample_item("arg-swap", "/y.rs", line=42),
                _sample_item("clone-drift", "/z.rs", line=100),
            ]
            input_path.write_text(json.dumps(findings))

            blind_path = tdp / "out/blind.csv"
            ctx_path = tdp / "out/context.json"

            rc = main(
                [
                    str(input_path),
                    "--blind-out",
                    str(blind_path),
                    "--context-out",
                    str(ctx_path),
                ]
            )
            self.assertEqual(rc, 0)

            with blind_path.open() as f:
                rows = list(csv.DictReader(f))
            self.assertEqual(len(rows), 3)
            self.assertEqual(rows[0]["id"], "1")
            self.assertEqual(rows[1]["id"], "2")
            self.assertEqual(rows[2]["id"], "3")
            self.assertEqual(rows[0]["detector_id"], "clone-drift")
            self.assertEqual(rows[1]["detector_id"], "arg-swap")
            self.assertEqual(set(rows[0].keys()), set(BLIND_COLUMNS))

            ctx = json.loads(ctx_path.read_text())
            self.assertEqual(len(ctx), 3)
            self.assertEqual(ctx[0]["id"], 1)
            self.assertEqual(ctx[2]["id"], 3)
            for entry in ctx:
                self.assertIn("rank_score", entry)
                self.assertIn("message", entry)

    def test_main_returns_2_on_missing_input(self):
        with tempfile.TemporaryDirectory() as td:
            rc = main(
                [
                    str(Path(td) / "nope.json"),
                    "--blind-out",
                    str(Path(td) / "blind.csv"),
                    "--context-out",
                    str(Path(td) / "ctx.json"),
                ]
            )
            self.assertEqual(rc, 2)

    def test_main_returns_1_on_invalid_json(self):
        with tempfile.TemporaryDirectory() as td:
            tdp = Path(td)
            input_path = tdp / "bad.json"
            input_path.write_text("not json {{{")
            rc = main(
                [
                    str(input_path),
                    "--blind-out",
                    str(tdp / "blind.csv"),
                    "--context-out",
                    str(tdp / "ctx.json"),
                ]
            )
            self.assertEqual(rc, 1)

    def test_main_returns_1_on_schema_mismatch(self):
        with tempfile.TemporaryDirectory() as td:
            tdp = Path(td)
            input_path = tdp / "sample.json"
            input_path.write_text(
                json.dumps([{"finding": {"primary": {"file": "/x.rs"}}}])
            )
            rc = main(
                [
                    str(input_path),
                    "--blind-out",
                    str(tdp / "blind.csv"),
                    "--context-out",
                    str(tdp / "ctx.json"),
                ]
            )
            self.assertEqual(rc, 1)


class RelativiseTests(unittest.TestCase):
    def test_no_corpus_root_returns_input(self):
        self.assertEqual(relativise("/anything", None), "/anything")


if __name__ == "__main__":
    unittest.main()
