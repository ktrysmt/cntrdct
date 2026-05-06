#!/usr/bin/env python3
"""Phase 1 labelling CSV pre-flight validator.

Catches structural and consistency errors in ``phase1-labels.csv``
before downstream tools (``phase1_kappa_wilson.py``,
``phase1_precision.py``, ``phase1_failure_modes_aggregate.py``)
consume it. Designed to surface as many issues as possible per
invocation rather than failing fast on the first one, so a rater
or adjudicator can fix a batch of issues in one editing pass.

Schema (CSV columns, header row required):

    id, detector_id, rater1_label, rater2_label  (REQUIRED)

The following columns are validated only when present:

    consensus_label, round, tiebreak_rater,
    failure_mode, failure_mode_notes

Other columns (rater1_rubric, rater1_notes, rater2_rubric,
rater2_notes, consensus_rubric, consensus_notes,
tiebreak_rationale) are passed through without validation; they
are free-form fields whose content is the adjudicator's
responsibility.

Validation rules:

R1. ``id`` and ``detector_id`` must be non-empty on every row.
R2. ``id`` must be unique across rows.
R3. ``rater1_label`` and ``rater2_label`` must be one of
    {TP, FP, Uncertain, ""}.
R4. ``consensus_label`` (if column present) must be one of
    {TP, FP, Uncertain, ""}.
R5. ``round`` (if column present) must be one of
    {"", "1", "2", "3"}.
R6. ``tiebreak_rater`` non-empty IFF ``round == "3"``. (Round 3
    rows must record a tiebreaker; non-round-3 rows must not.)
R7. ``consensus_label`` non-empty IMPLIES ``round`` non-empty.
    Round-empty rows have no adjudication record; setting
    consensus_label without round breaks the audit trail.
R8. ``failure_mode`` non-empty IMPLIES ``consensus_label == "FP"``.
    The failure_mode column is meaningful only on adjudicated FP
    rows (failure-modes-v1.md section 2.1).
R9. ``failure_mode == "other"`` IMPLIES
    ``failure_mode_notes`` non-empty (failure-modes-v1.md
    section 2.3 mandates a note explaining why no controlled-
    vocabulary mode applied).

Vocabulary validation of ``failure_mode`` against the
per-detector controlled set is left to
``phase1_failure_modes_aggregate.py`` to avoid duplicating the
mapping; this validator focuses on row-level structural and
cross-row consistency rules.
"""

from __future__ import annotations

import argparse
import csv
import sys
from dataclasses import dataclass
from pathlib import Path

REQUIRED_COLUMNS = ("id", "detector_id", "rater1_label", "rater2_label")
LABEL_VALUES = {"TP", "FP", "Uncertain", ""}
ROUND_VALUES = {"", "1", "2", "3"}
OPTIONAL_VALIDATED = (
    "consensus_label",
    "round",
    "tiebreak_rater",
    "failure_mode",
    "failure_mode_notes",
)


@dataclass(frozen=True)
class ValidationError:
    line: int
    rule: str
    message: str


def _check_row(
    n: int, r: dict[str, str], seen_ids: set[str], cols: set[str]
) -> list[ValidationError]:
    errs: list[ValidationError] = []

    # R1
    if not r["id"].strip():
        errs.append(ValidationError(n, "R1", "id is empty"))
    if not r["detector_id"].strip():
        errs.append(ValidationError(n, "R1", "detector_id is empty"))

    # R2
    rid = r["id"]
    if rid and rid in seen_ids:
        errs.append(ValidationError(n, "R2", f"duplicate id={rid!r}"))
    seen_ids.add(rid)

    # R3
    for col in ("rater1_label", "rater2_label"):
        if r[col] not in LABEL_VALUES:
            errs.append(
                ValidationError(
                    n,
                    "R3",
                    f"{col}={r[col]!r} not in {sorted(LABEL_VALUES)}",
                )
            )

    # R4
    if "consensus_label" in cols:
        if r["consensus_label"] not in LABEL_VALUES:
            errs.append(
                ValidationError(
                    n,
                    "R4",
                    f"consensus_label={r['consensus_label']!r} not in "
                    f"{sorted(LABEL_VALUES)}",
                )
            )

    # R5
    if "round" in cols:
        if r["round"] not in ROUND_VALUES:
            errs.append(
                ValidationError(
                    n,
                    "R5",
                    f"round={r['round']!r} not in {sorted(ROUND_VALUES)}",
                )
            )

    # R6
    if "round" in cols and "tiebreak_rater" in cols:
        is_r3 = r["round"] == "3"
        has_tb = bool(r["tiebreak_rater"].strip())
        if is_r3 and not has_tb:
            errs.append(
                ValidationError(
                    n,
                    "R6",
                    "round=3 row has empty tiebreak_rater",
                )
            )
        if has_tb and not is_r3:
            errs.append(
                ValidationError(
                    n,
                    "R6",
                    f"tiebreak_rater set but round={r['round']!r}",
                )
            )

    # R7
    if "consensus_label" in cols and "round" in cols:
        if r["consensus_label"] and not r["round"]:
            errs.append(
                ValidationError(
                    n,
                    "R7",
                    "consensus_label set but round is empty",
                )
            )

    # R8
    if "failure_mode" in cols and "consensus_label" in cols:
        if r["failure_mode"].strip() and r["consensus_label"] != "FP":
            errs.append(
                ValidationError(
                    n,
                    "R8",
                    f"failure_mode={r['failure_mode']!r} set but "
                    f"consensus_label={r['consensus_label']!r}",
                )
            )

    # R9
    if "failure_mode" in cols and "failure_mode_notes" in cols:
        if r["failure_mode"] == "other" and not r["failure_mode_notes"].strip():
            errs.append(
                ValidationError(
                    n,
                    "R9",
                    "failure_mode='other' requires non-empty failure_mode_notes",
                )
            )

    return errs


def validate(path: Path) -> list[ValidationError]:
    with path.open() as f:
        reader = csv.DictReader(f)
        cols = set(reader.fieldnames or [])
        missing = set(REQUIRED_COLUMNS) - cols
        if missing:
            return [
                ValidationError(
                    1,
                    "R0",
                    f"missing required columns: {sorted(missing)}",
                )
            ]
        seen_ids: set[str] = set()
        all_errs: list[ValidationError] = []
        for n, r in enumerate(reader, start=2):
            all_errs.extend(_check_row(n, r, seen_ids, cols))
        return all_errs


def format_errors(errors: list[ValidationError]) -> str:
    if not errors:
        return "OK\n"
    lines = [f"{len(errors)} validation error(s):"]
    for e in errors:
        lines.append(f"  line {e.line} [{e.rule}]: {e.message}")
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(
        description="Phase 1 labelling CSV pre-flight validator",
    )
    ap.add_argument(
        "input",
        type=Path,
        help="Phase 1 labelling CSV to validate",
    )
    args = ap.parse_args(argv)
    if not args.input.exists():
        print(f"input not found: {args.input}", file=sys.stderr)
        return 2
    errors = validate(args.input)
    sys.stdout.write(format_errors(errors))
    return 0 if not errors else 1


if __name__ == "__main__":
    sys.exit(main())
