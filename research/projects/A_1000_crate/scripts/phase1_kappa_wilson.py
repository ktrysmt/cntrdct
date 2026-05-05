#!/usr/bin/env python3
"""Phase 1 inter-rater agreement and per-detector precision analyser.

Reads a Phase 1 labelling CSV with two rater labels per finding and emits a
Markdown summary covering Cohen's kappa across the two raters (overall and
per detector) plus per-detector precision (TP / (TP + FP)) with a Wilson
95 percent confidence interval.

Schema (CSV columns, header row required):

    id,detector_id,rater1_label,rater2_label

Each label cell must be one of: TP, FP, Uncertain.

Conventions, fixed by this script:

- Cohen's kappa is computed over the binary {TP, FP} category set after
  dropping rows where either rater wrote Uncertain. This matches the most
  common publication convention for two-rater coding with a residual
  Uncertain bucket.
- Per-detector precision is TP / (TP + FP) on rows where both raters
  agreed on a non-Uncertain label. Disagreement rows have no ground truth
  and are excluded from precision (but counted toward kappa).
- Wilson 95 percent CI uses z = 1.959963984540054 (the standard normal
  97.5th percentile). Returns NaN when the denominator is zero, matching
  the divide-by-zero rule used elsewhere in the cntrdct toolchain.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import math
import sys
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

LABELS = ("TP", "FP", "Uncertain")
NON_UNCERTAIN = ("TP", "FP")
Z_95 = 1.959963984540054


@dataclass(frozen=True)
class Row:
    id: str
    detector_id: str
    rater1: str
    rater2: str


def parse_rows(path: Path) -> list[Row]:
    with path.open() as f:
        reader = csv.DictReader(f)
        required = {"id", "detector_id", "rater1_label", "rater2_label"}
        missing = required - set(reader.fieldnames or [])
        if missing:
            raise ValueError(f"missing required columns: {sorted(missing)}")
        rows: list[Row] = []
        for n, r in enumerate(reader, start=2):
            for col in ("rater1_label", "rater2_label"):
                if r[col] not in LABELS:
                    raise ValueError(
                        f"row {n}: invalid {col}={r[col]!r}; expected one of {LABELS}"
                    )
            rows.append(
                Row(
                    id=r["id"],
                    detector_id=r["detector_id"],
                    rater1=r["rater1_label"],
                    rater2=r["rater2_label"],
                )
            )
        return rows


def cohen_kappa(
    rater1: list[str], rater2: list[str], categories: tuple[str, ...]
) -> float:
    n = len(rater1)
    if n == 0 or len(rater2) != n:
        return float("nan")
    confusion = {(c1, c2): 0 for c1 in categories for c2 in categories}
    for a, b in zip(rater1, rater2):
        if (a, b) not in confusion:
            raise ValueError(
                f"label outside declared categories: ({a!r}, {b!r})"
            )
        confusion[(a, b)] += 1
    p_o = sum(confusion[(c, c)] for c in categories) / n
    row_marg = {
        c: sum(confusion[(c, c2)] for c2 in categories) / n for c in categories
    }
    col_marg = {
        c: sum(confusion[(c1, c)] for c1 in categories) / n for c in categories
    }
    p_e = sum(row_marg[c] * col_marg[c] for c in categories)
    if p_e >= 1.0:
        return float("nan")
    return (p_o - p_e) / (1.0 - p_e)


def wilson_ci(tp: int, fp: int, z: float = Z_95) -> tuple[float, float, float]:
    n = tp + fp
    if n == 0:
        return float("nan"), float("nan"), float("nan")
    p = tp / n
    denom = 1.0 + z * z / n
    center = (p + z * z / (2.0 * n)) / denom
    half = (
        z * math.sqrt(p * (1.0 - p) / n + z * z / (4.0 * n * n)) / denom
    )
    return p, max(0.0, center - half), min(1.0, center + half)


def file_sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def _fmt(x: float) -> str:
    return "NaN" if math.isnan(x) else f"{x:.3f}"


def render_summary(rows: list[Row], input_path: Path) -> str:
    kappa_rows = [
        r for r in rows if r.rater1 != "Uncertain" and r.rater2 != "Uncertain"
    ]
    overall_kappa = cohen_kappa(
        [r.rater1 for r in kappa_rows],
        [r.rater2 for r in kappa_rows],
        NON_UNCERTAIN,
    )

    by_detector: dict[str, list[Row]] = defaultdict(list)
    for r in rows:
        by_detector[r.detector_id].append(r)

    lines: list[str] = []
    lines.append("# Phase 1 inter-rater + precision summary")
    lines.append("")
    lines.append("## Meta")
    lines.append("")
    lines.append(f"- input: `{input_path.name}`")
    lines.append(f"- input_sha256: `{file_sha256(input_path)}`")
    lines.append(
        f"- generated_at_utc: {datetime.now(timezone.utc).isoformat()}"
    )
    lines.append("- script: `phase1_kappa_wilson.py`")
    lines.append(f"- total_rows: {len(rows)}")
    lines.append(
        f"- rows_used_for_kappa: {len(kappa_rows)} "
        "(excluded Uncertain on either side)"
    )
    lines.append("")
    lines.append("## Overall Cohen's kappa")
    lines.append("")
    lines.append(f"kappa = {_fmt(overall_kappa)}")
    lines.append("")
    lines.append("Acceptance threshold for Phase 1: kappa >= 0.6.")
    lines.append("")
    lines.append("## Per-detector breakdown")
    lines.append("")
    lines.append(
        "| detector | n_total | n_kappa | kappa | tp_agreed | fp_agreed | "
        "precision | wilson_lower | wilson_upper |"
    )
    lines.append(
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    )
    for det in sorted(by_detector):
        d_rows = by_detector[det]
        d_kappa_rows = [
            r
            for r in d_rows
            if r.rater1 != "Uncertain" and r.rater2 != "Uncertain"
        ]
        d_kappa = cohen_kappa(
            [r.rater1 for r in d_kappa_rows],
            [r.rater2 for r in d_kappa_rows],
            NON_UNCERTAIN,
        )
        agreed = [r for r in d_kappa_rows if r.rater1 == r.rater2]
        tp = sum(1 for r in agreed if r.rater1 == "TP")
        fp = sum(1 for r in agreed if r.rater1 == "FP")
        p, lo, hi = wilson_ci(tp, fp)
        lines.append(
            f"| {det} | {len(d_rows)} | {len(d_kappa_rows)} | {_fmt(d_kappa)} | "
            f"{tp} | {fp} | {_fmt(p)} | {_fmt(lo)} | {_fmt(hi)} |"
        )
    lines.append("")
    lines.append("## Conventions")
    lines.append("")
    lines.append(
        "- Cohen's kappa is computed over the binary {TP, FP} category set "
        "after dropping rows where either rater wrote Uncertain."
    )
    lines.append(
        "- Per-detector precision is TP / (TP + FP) on rows where both "
        "raters agreed on a non-Uncertain label. Disagreement rows are "
        "excluded from precision but counted toward kappa."
    )
    lines.append(
        "- Wilson 95 percent CI uses z = 1.959963984540054. NaN when the "
        "denominator is zero."
    )
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(
        description="Phase 1 inter-rater agreement and per-detector precision analyser",
    )
    ap.add_argument(
        "input", type=Path, help="Phase 1 labelling CSV"
    )
    ap.add_argument(
        "-o",
        "--output",
        type=Path,
        help="Output Markdown path (default: <input-stem>-summary.md)",
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
        f"{args.input.stem}-summary.md"
    )
    out.write_text(render_summary(rows, args.input))
    print(f"wrote {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
