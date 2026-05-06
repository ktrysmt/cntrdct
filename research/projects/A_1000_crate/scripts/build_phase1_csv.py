#!/usr/bin/env python3
"""Build a Phase 1 labelling CSV from a stratified-sample JSON output.

Reads the JSON output of `cntrdct-research stratified-sample` and emits
two complementary artefacts:

- A blind labelling CSV with one row per finding and empty rater
  columns:
      id, detector_id, file, line, citation_keys,
      rater1_label, rater1_rubric, rater1_notes,
      rater2_label, rater2_rubric, rater2_notes

- A context sidecar JSON keyed by id, holding the finding fields that
  must NOT be visible to raters during labelling: message,
  anomaly_class, rank_score, posterior_tp, wilson_lower. These are
  the anchor-bias-prone signals identified by
  prereg/2026-05-04-labelling-rubric-v0.md §6 (Phase 0's three
  hidden columns plus the two ranker-internal probabilities).

The sequential id is the row's position in the input array (1-indexed),
so re-running on the same input is deterministic and the id column is a
stable join key with the sidecar.

Downstream: once raters fill rater1_label / rater2_label with values
from {TP, FP, Uncertain}, the resulting CSV can be re-shaped to the
schema expected by phase1_kappa_wilson.py (id, detector_id,
rater1_label, rater2_label) by selecting just those four columns.
"""

from __future__ import annotations

import argparse
import csv
import json
import sys
from pathlib import Path

ANCHOR_FIELDS_IN_FINDING = ("message", "anomaly_class")
ANCHOR_FIELDS_IN_ITEM = ("rank_score", "posterior_tp", "wilson_lower")

BLIND_COLUMNS = (
    "id",
    "detector_id",
    "file",
    "line",
    "citation_keys",
    "rater1_label",
    "rater1_rubric",
    "rater1_notes",
    "rater2_label",
    "rater2_rubric",
    "rater2_notes",
)


def load_findings(path: Path) -> list[dict]:
    body = path.read_text()
    value = json.loads(body)
    if not isinstance(value, list):
        raise ValueError(
            f"{path}: expected a JSON array at the top level, "
            f"got {type(value).__name__}"
        )
    return value


def relativise(file_str: str, corpus_root: Path | None) -> str:
    if corpus_root is None:
        return file_str
    try:
        rel = Path(file_str).resolve().relative_to(corpus_root.resolve())
    except (ValueError, OSError):
        return file_str
    return str(rel)


def extract_blind_row(
    idx: int, item: dict, corpus_root: Path | None
) -> dict:
    finding = item["finding"]
    primary = finding["primary"]
    evidence = finding.get("evidence", {})
    citation_keys = evidence.get("citation_keys", []) or []
    return {
        "id": idx,
        "detector_id": finding["detector_id"],
        "file": relativise(primary["file"], corpus_root),
        "line": primary.get("start_line", ""),
        "citation_keys": ";".join(citation_keys),
        "rater1_label": "",
        "rater1_rubric": "",
        "rater1_notes": "",
        "rater2_label": "",
        "rater2_rubric": "",
        "rater2_notes": "",
    }


def extract_context_entry(idx: int, item: dict) -> dict:
    finding = item["finding"]
    out: dict = {"id": idx}
    for field in ANCHOR_FIELDS_IN_FINDING:
        if field in finding:
            out[field] = finding[field]
    for field in ANCHOR_FIELDS_IN_ITEM:
        if field in item:
            out[field] = item[field]
    return out


def write_blind_csv(rows: list[dict], path: Path) -> None:
    parent = path.parent
    if str(parent):
        parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=list(BLIND_COLUMNS))
        writer.writeheader()
        for r in rows:
            writer.writerow(r)


def write_context_json(entries: list[dict], path: Path) -> None:
    parent = path.parent
    if str(parent):
        parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(entries, indent=2) + "\n")


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(
        description=(
            "Build a Phase 1 blind labelling CSV plus a context sidecar "
            "JSON from a stratified-sample JSON output."
        ),
    )
    ap.add_argument(
        "input",
        type=Path,
        help="JSON output of cntrdct-research stratified-sample",
    )
    ap.add_argument(
        "--blind-out",
        type=Path,
        required=True,
        help="Output blind CSV (no anchor fields)",
    )
    ap.add_argument(
        "--context-out",
        type=Path,
        required=True,
        help="Output sidecar JSON with anchor fields, keyed by id",
    )
    ap.add_argument(
        "--corpus-root",
        type=Path,
        help=(
            "Strip this prefix from file paths in the blind CSV. "
            "Falls back to the original path if a finding is not under "
            "the prefix."
        ),
    )
    args = ap.parse_args(argv)

    if not args.input.exists():
        print(f"input not found: {args.input}", file=sys.stderr)
        return 2
    try:
        findings = load_findings(args.input)
    except (ValueError, json.JSONDecodeError) as e:
        print(f"parse error: {e}", file=sys.stderr)
        return 1

    try:
        blind_rows = [
            extract_blind_row(i + 1, item, args.corpus_root)
            for i, item in enumerate(findings)
        ]
        context_entries = [
            extract_context_entry(i + 1, item)
            for i, item in enumerate(findings)
        ]
    except (KeyError, TypeError) as e:
        print(
            f"schema error: stratified-sample JSON missing expected key {e}",
            file=sys.stderr,
        )
        return 1

    write_blind_csv(blind_rows, args.blind_out)
    write_context_json(context_entries, args.context_out)
    print(f"wrote {len(blind_rows)} blind rows to {args.blind_out}")
    print(f"wrote {len(context_entries)} context entries to {args.context_out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
