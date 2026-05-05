"""Aggregate `labelling-rated.csv` into a Phase-0 summary report.

Reads the rated CSV produced by following
`prereg/2026-05-04-labelling-rubric-v0.md`, computes per-detector
counts and Wilson 95% confidence intervals on precision, and emits a
Markdown report suitable for the Phase-0 go/no-go decision and as a
paper supplement. Pure stdlib so it runs on a fresh checkout.

Usage:
  python3 scripts/phase0/aggregate_labels.py
  python3 scripts/phase0/aggregate_labels.py --input X.csv --out Y.md
  python3 scripts/phase0/aggregate_labels.py --json data/phase0/summary.json

Exits non-zero on:
  - missing or empty `label` cells (the rubric requires every row labelled);
  - label values outside {TP, FP, Uncertain} (case-sensitive per rubric).
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import sys
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_INPUT = REPO_ROOT / "data/phase0/labelling-rated.csv"
DEFAULT_MD = REPO_ROOT / "data/phase0/labelling-rated-summary.md"
VALID_LABELS = ("TP", "FP", "Uncertain")
Z_95 = 1.959963984540054  # two-sided z for 95% CI


def wilson_ci(tp: int, fp: int, z: float = Z_95) -> tuple[float, float, float]:
    """Wilson score interval for the binomial proportion TP/(TP+FP).

    Returns (point_estimate, lower, upper). All three are NaN when the
    denominator is zero, matching the convention used by `cntrdct
    calibrate`.
    """
    n = tp + fp
    if n == 0:
        return float("nan"), float("nan"), float("nan")
    p = tp / n
    denom = 1.0 + z * z / n
    center = (p + z * z / (2.0 * n)) / denom
    half = z * math.sqrt(p * (1.0 - p) / n + z * z / (4.0 * n * n)) / denom
    return p, max(0.0, center - half), min(1.0, center + half)


def display_path(path: Path) -> str:
    """Return `path` relative to the repo root if it lives there, else
    fall back to its absolute form. The script accepts inputs from
    anywhere on disk (e.g. `/tmp` during tests) so we cannot assume the
    path is rooted in the workspace."""
    try:
        return str(path.relative_to(REPO_ROOT))
    except ValueError:
        return str(path)


def sha256_of(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def load_rows(path: Path) -> list[dict]:
    with path.open(newline="") as f:
        return list(csv.DictReader(f))


def validate(rows: list[dict]) -> list[str]:
    errors: list[str] = []
    for i, r in enumerate(rows, start=1):
        label = r.get("label", "").strip()
        if not label:
            errors.append(f"row {i} (id={r.get('id', '?')}): empty `label`")
            continue
        if label not in VALID_LABELS:
            errors.append(
                f"row {i} (id={r.get('id', '?')}): invalid label "
                f"`{label}` (must be one of {VALID_LABELS})"
            )
    return errors


def aggregate(rows: list[dict]) -> dict:
    by_det: dict[str, Counter] = defaultdict(Counter)
    clause_by_det: dict[str, Counter] = defaultdict(Counter)
    overall = Counter()
    uncertain_notes: list[dict] = []

    for r in rows:
        det = r["detector_id"]
        label = r["label"].strip()
        clause = r.get("rubric_clause", "").strip() or "(unspecified)"
        by_det[det][label] += 1
        overall[label] += 1
        clause_by_det[det][clause] += 1
        if label == "Uncertain":
            uncertain_notes.append({
                "id": r.get("id"),
                "detector_id": det,
                "crate": r.get("crate", ""),
                "clause": clause,
                "notes": r.get("notes", ""),
            })

    detectors: dict[str, dict] = {}
    for det, c in sorted(by_det.items()):
        tp, fp, unc = c["TP"], c["FP"], c["Uncertain"]
        n_decided = tp + fp
        n_total = tp + fp + unc
        p, lo, hi = wilson_ci(tp, fp)
        detectors[det] = {
            "tp": tp,
            "fp": fp,
            "uncertain": unc,
            "decided": n_decided,
            "total": n_total,
            "precision": p,
            "ci95_lower": lo,
            "ci95_upper": hi,
            "uncertain_rate": unc / n_total if n_total else float("nan"),
            "rubric_clauses": dict(clause_by_det[det].most_common()),
        }

    overall_p, overall_lo, overall_hi = wilson_ci(overall["TP"], overall["FP"])
    return {
        "overall": {
            "tp": overall["TP"],
            "fp": overall["FP"],
            "uncertain": overall["Uncertain"],
            "decided": overall["TP"] + overall["FP"],
            "total": sum(overall.values()),
            "precision": overall_p,
            "ci95_lower": overall_lo,
            "ci95_upper": overall_hi,
        },
        "detectors": detectors,
        "uncertain_audit": uncertain_notes,
    }


def fmt_pct(x: float) -> str:
    return "n/a" if math.isnan(x) else f"{100 * x:.1f}%"


def render_markdown(summary: dict, *, input_path: Path, input_sha: str) -> str:
    overall = summary["overall"]
    lines: list[str] = []
    lines.append("# Phase 0 labelling summary")
    lines.append("")
    lines.append(f"- Input: `{display_path(input_path)}`")
    lines.append(f"- Input SHA-256: `{input_sha}`")
    lines.append(
        f"- Generated: {datetime.now(timezone.utc).isoformat(timespec='seconds')}"
    )
    lines.append(
        "- Rubric: `prereg/2026-05-04-labelling-rubric-v0.md` "
        "(single-rater pilot; precision is an upper bound)"
    )
    lines.append("")
    lines.append("## Overall")
    lines.append("")
    lines.append(
        f"- Total labelled: **{overall['total']}** "
        f"(TP={overall['tp']}, FP={overall['fp']}, "
        f"Uncertain={overall['uncertain']})"
    )
    lines.append(
        f"- Decided (TP+FP): **{overall['decided']}** of "
        f"{overall['total']}"
    )
    if overall["decided"] > 0:
        lines.append(
            f"- Precision: **{fmt_pct(overall['precision'])}** "
            f"(Wilson 95% CI {fmt_pct(overall['ci95_lower'])} – "
            f"{fmt_pct(overall['ci95_upper'])})"
        )
    lines.append("")
    lines.append("## Per-detector precision")
    lines.append("")
    lines.append(
        "| Detector | n | TP | FP | Unc | Precision | "
        "Wilson 95% lower | Wilson 95% upper |"
    )
    lines.append(
        "|---|---:|---:|---:|---:|---:|---:|---:|"
    )
    for det, s in sorted(summary["detectors"].items()):
        lines.append(
            f"| `{det}` | {s['total']} | {s['tp']} | {s['fp']} | "
            f"{s['uncertain']} | {fmt_pct(s['precision'])} | "
            f"{fmt_pct(s['ci95_lower'])} | {fmt_pct(s['ci95_upper'])} |"
        )
    lines.append("")
    lines.append("## Rubric clause distribution")
    lines.append("")
    for det, s in sorted(summary["detectors"].items()):
        lines.append(f"### `{det}`")
        lines.append("")
        lines.append("| Clause | Count |")
        lines.append("|---|---:|")
        for clause, count in s["rubric_clauses"].items():
            lines.append(f"| `{clause}` | {count} |")
        lines.append("")
    lines.append("## Uncertain audit")
    lines.append("")
    if not summary["uncertain_audit"]:
        lines.append("(no findings labelled `Uncertain`)")
    else:
        lines.append("| id | detector | crate | clause | notes |")
        lines.append("|---|---|---|---|---|")
        for u in summary["uncertain_audit"]:
            notes = (u["notes"] or "").replace("|", "\\|").replace("\n", " ")
            lines.append(
                f"| {u['id']} | `{u['detector_id']}` | "
                f"`{u['crate']}` | `{u['clause']}` | {notes} |"
            )
    lines.append("")
    lines.append("## Notes")
    lines.append("")
    lines.append(
        "- Wilson 95% CI is the standard interval for binomial proportions; "
        "lower bound is what should drive Phase-1 go/no-go decisions."
    )
    lines.append(
        "- Uncertain rows are excluded from the precision denominator. "
        "A high uncertain rate reduces effective sample size and widens CI."
    )
    lines.append(
        "- Single-rater pilot: numbers above are an upper bound under "
        "rater-author confirmation bias. Phase 1 (top-1000, two raters, "
        "Cohen's κ) supersedes."
    )
    lines.append("")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--input",
        type=Path,
        default=DEFAULT_INPUT,
        help=f"Rated CSV (default: {DEFAULT_INPUT.relative_to(REPO_ROOT)})",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=DEFAULT_MD,
        help=f"Markdown output (default: {DEFAULT_MD.relative_to(REPO_ROOT)})",
    )
    parser.add_argument(
        "--json",
        type=Path,
        default=None,
        help="Optional JSON output path for downstream tooling",
    )
    parser.add_argument(
        "--allow-empty",
        action="store_true",
        help="Skip the rubric requirement that every row be labelled",
    )
    args = parser.parse_args(argv)

    if not args.input.exists():
        print(f"error: input not found: {args.input}", file=sys.stderr)
        return 2

    rows = load_rows(args.input)
    if not rows:
        print(f"error: input has no data rows: {args.input}", file=sys.stderr)
        return 2

    errors = validate(rows)
    if errors and not args.allow_empty:
        print("error: rated CSV has problems:", file=sys.stderr)
        for e in errors[:20]:
            print(f"  - {e}", file=sys.stderr)
        if len(errors) > 20:
            print(f"  ... and {len(errors) - 20} more", file=sys.stderr)
        print(
            "Re-run with --allow-empty to proceed anyway (excludes empty rows).",
            file=sys.stderr,
        )
        return 1

    if args.allow_empty:
        rows = [r for r in rows if r.get("label", "").strip() in VALID_LABELS]
        if not rows:
            print("error: no rows have valid labels", file=sys.stderr)
            return 2

    summary = aggregate(rows)
    summary["meta"] = {
        "input": display_path(args.input),
        "input_sha256": sha256_of(args.input),
        "generated_utc": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "rubric": "prereg/2026-05-04-labelling-rubric-v0.md",
    }

    md = render_markdown(
        summary,
        input_path=args.input,
        input_sha=summary["meta"]["input_sha256"],
    )
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(md)

    if args.json is not None:
        args.json.parent.mkdir(parents=True, exist_ok=True)
        args.json.write_text(json.dumps(summary, indent=2, ensure_ascii=False))

    print(f"wrote {display_path(args.out)}")
    if args.json is not None:
        print(f"wrote {display_path(args.json)}")
    print()
    print(
        "  overall: TP={tp} FP={fp} Uncertain={uncertain} "
        "precision={p} (95% CI {lo} - {hi})".format(
            tp=summary["overall"]["tp"],
            fp=summary["overall"]["fp"],
            uncertain=summary["overall"]["uncertain"],
            p=fmt_pct(summary["overall"]["precision"]),
            lo=fmt_pct(summary["overall"]["ci95_lower"]),
            hi=fmt_pct(summary["overall"]["ci95_upper"]),
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
