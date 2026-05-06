#!/usr/bin/env python3
"""Phase 1 per-detector precision analyser.

Reads a Phase 1 labelling CSV with a ``consensus_label`` column populated
by the round-2 / round-3 adjudication protocol of rubric v1 section 7
and emits a Markdown summary covering per-detector precision
(TP / (TP + FP)) with a Wilson 95 percent confidence interval.

Schema (CSV columns, header row required):

    id, detector_id, consensus_label

Additional columns (e.g., ``rater1_label``, ``rater2_label``,
``consensus_rubric``, ``round``) may be present and are ignored.
``consensus_label`` cells must be one of {TP, FP, Uncertain, ""}.
The empty string is permitted to allow incremental fill-in during
ongoing adjudication; such rows are excluded from the denominator,
matching the rubric v1 section 10 rule that only adjudicated rows
contribute to the per-detector precision figure.

Conventions, fixed by this script:

- Per-detector precision is TP / (TP + FP) on rows where
  ``consensus_label`` is TP or FP. Rows with empty or ``Uncertain``
  ``consensus_label`` are excluded from the denominator (their ground
  truth is not yet established or has been declared indeterminate).
- Wilson 95 percent CI uses z = 1.959963984540054 (the standard normal
  97.5th percentile). Returns NaN when the denominator is zero,
  matching the divide-by-zero rule used in ``phase1_kappa_wilson.py``.
- Detector-level rows with zero countable rows are still emitted
  (n_total is shown but precision is NaN). This makes "no ground truth
  yet" visible in the report rather than silently dropping the
  detector.
"""

from __future__ import annotations

import argparse
import csv
import math
import sys
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

from phase1_kappa_wilson import file_sha256, wilson_ci

VALID_CONSENSUS = ("TP", "FP", "Uncertain", "")
COUNTABLE = ("TP", "FP")


@dataclass(frozen=True)
class Row:
    id: str
    detector_id: str
    consensus_label: str


def parse_rows(path: Path) -> list[Row]:
    with path.open() as f:
        reader = csv.DictReader(f)
        required = {"id", "detector_id", "consensus_label"}
        missing = required - set(reader.fieldnames or [])
        if missing:
            raise ValueError(f"missing required columns: {sorted(missing)}")
        rows: list[Row] = []
        for n, r in enumerate(reader, start=2):
            label = r["consensus_label"]
            if label not in VALID_CONSENSUS:
                raise ValueError(
                    f"row {n}: invalid consensus_label={label!r}; "
                    f"expected one of {VALID_CONSENSUS}"
                )
            rows.append(
                Row(
                    id=r["id"],
                    detector_id=r["detector_id"],
                    consensus_label=label,
                )
            )
        return rows


def _fmt(x: float) -> str:
    return "NaN" if math.isnan(x) else f"{x:.3f}"


def render_summary(rows: list[Row], input_path: Path) -> str:
    by_detector: dict[str, list[Row]] = defaultdict(list)
    for r in rows:
        by_detector[r.detector_id].append(r)

    n_consensus_all = sum(
        1 for r in rows if r.consensus_label in COUNTABLE
    )

    lines: list[str] = []
    lines.append("# Phase 1 per-detector precision summary")
    lines.append("")
    lines.append("## Meta")
    lines.append("")
    lines.append(f"- input: `{input_path.name}`")
    lines.append(f"- input_sha256: `{file_sha256(input_path)}`")
    lines.append(
        f"- generated_at_utc: {datetime.now(timezone.utc).isoformat()}"
    )
    lines.append("- script: `phase1_precision.py`")
    lines.append(f"- total_rows: {len(rows)}")
    lines.append(
        f"- rows_with_consensus: {n_consensus_all} "
        "(consensus_label in {TP, FP})"
    )
    lines.append("")
    lines.append("## Per-detector precision")
    lines.append("")
    lines.append(
        "| detector | n_total | n_consensus | tp | fp | "
        "precision | wilson_lower | wilson_upper |"
    )
    lines.append("| --- | --- | --- | --- | --- | --- | --- | --- |")
    for det in sorted(by_detector):
        d_rows = by_detector[det]
        countable = [r for r in d_rows if r.consensus_label in COUNTABLE]
        tp = sum(1 for r in countable if r.consensus_label == "TP")
        fp = sum(1 for r in countable if r.consensus_label == "FP")
        p, lo, hi = wilson_ci(tp, fp)
        lines.append(
            f"| {det} | {len(d_rows)} | {len(countable)} | "
            f"{tp} | {fp} | {_fmt(p)} | {_fmt(lo)} | {_fmt(hi)} |"
        )
    lines.append("")
    lines.append("## Conventions")
    lines.append("")
    lines.append(
        "- Precision is TP / (TP + FP) over rows where "
        "consensus_label is TP or FP."
    )
    lines.append(
        "- Rows with empty or `Uncertain` consensus_label are excluded "
        "from the denominator (no ground truth)."
    )
    lines.append(
        "- Wilson 95 percent CI uses z = 1.959963984540054. NaN when "
        "the denominator is zero."
    )
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(
        description="Phase 1 per-detector precision analyser",
    )
    ap.add_argument(
        "input",
        type=Path,
        help="Phase 1 labelling CSV with a consensus_label column",
    )
    ap.add_argument(
        "-o",
        "--output",
        type=Path,
        help=(
            "Output Markdown path "
            "(default: <input parent>/phase1-precision-summary.md)"
        ),
    )
    args = ap.parse_args(argv)
    if not args.input.exists():
        print(f"input not found: {args.input}", file=sys.stderr)
        return 2
    try:
        rows = parse_rows(args.input)
    except ValueError as e:
        print(f"parse error: {e}", file=sys.stderr)
        return 1
    out = args.output or args.input.with_name(
        "phase1-precision-summary.md"
    )
    out.write_text(render_summary(rows, args.input))
    print(f"wrote {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
