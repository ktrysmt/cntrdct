"""Convert labelling.json (79 RankedFinding records) into a Google-Sheets-ready
CSV with code excerpts. One row per finding.

Output columns:
  id, detector_id, anomaly_class, crate, file_rel, start_line, end_line,
  related_count, rank_score, message, primary_excerpt, related_excerpts,
  label, notes

label and notes are intentionally empty - they're filled by the human rater.
"""
from __future__ import annotations

import csv
import json
import os
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
CORPUS_PREFIX = "data/phase0/corpus/wild/"
EXCERPT_BEFORE = 5
EXCERPT_AFTER = 5
RELATED_INCLUDE = 2  # how many related locations to embed inline


def load_lines(path: Path) -> list[str]:
    try:
        return path.read_text(errors="replace").splitlines()
    except OSError:
        return []


def crate_from_path(file_rel: str) -> str:
    """`anyhow-1.0.102/src/lib.rs` -> `anyhow`."""
    head = file_rel.split("/", 1)[0]
    # crate-version: strip trailing -X.Y.Z[-suffix]
    m = re.match(r"^(.*?)-\d+\.\d+", head)
    return m.group(1) if m else head


def make_excerpt(lines: list[str], start: int, end: int, mark: bool = True) -> str:
    if not lines:
        return "(file unreadable)"
    lo = max(1, start - EXCERPT_BEFORE)
    hi = min(len(lines), end + EXCERPT_AFTER)
    width = len(str(hi))
    out = []
    for i in range(lo, hi + 1):
        marker = ">" if mark and start <= i <= end else " "
        out.append(f"{marker} {i:>{width}} | {lines[i-1]}")
    return "\n".join(out)


def to_rel(path_str: str) -> str:
    if path_str.startswith(CORPUS_PREFIX):
        return path_str[len(CORPUS_PREFIX):]
    return path_str


def primary_path(path_str: str) -> Path:
    """Resolve to a real path the script can read."""
    if os.path.isabs(path_str):
        return Path(path_str)
    return REPO_ROOT / path_str


def build_row(idx: int, item: dict) -> dict:
    finding = item["finding"]
    primary = finding["primary"]
    file_rel = to_rel(primary["file"])
    crate = crate_from_path(file_rel)
    primary_lines = load_lines(primary_path(primary["file"]))
    primary_excerpt = make_excerpt(
        primary_lines, primary["start_line"], primary["end_line"]
    )

    related = finding.get("related", []) or []
    related_blocks = []
    for r in related[:RELATED_INCLUDE]:
        rel_file_rel = to_rel(r["file"])
        rel_lines = load_lines(primary_path(r["file"]))
        block = (
            f"-- {rel_file_rel}:{r['start_line']}-{r['end_line']} --\n"
            + make_excerpt(rel_lines, r["start_line"], r["end_line"])
        )
        related_blocks.append(block)
    if len(related) > RELATED_INCLUDE:
        related_blocks.append(f"... +{len(related) - RELATED_INCLUDE} more related locations")
    related_excerpts = "\n\n".join(related_blocks) if related_blocks else ""

    # Column order is rubric-driven: information the rater needs to apply
    # `prereg/2026-05-04-labelling-rubric-v0.md` is on the left; potential
    # anchor-bias columns (rank_score, message, anomaly_class) are pushed
    # to the far right so the rater can hide them in the spreadsheet view
    # before rating, and unhide them only at unblinding / write-up time.
    return {
        "id": idx,
        "detector_id": finding["detector_id"],
        "crate": crate,
        "file_rel": file_rel,
        "start_line": primary["start_line"],
        "end_line": primary["end_line"],
        "related_count": len(related),
        "primary_excerpt": primary_excerpt,
        "related_excerpts": related_excerpts,
        "label": "",
        "rubric_clause": "",
        "notes": "",
        # --- columns below this point are blind-mode hidden during rating ---
        "anomaly_class": finding.get("anomaly_class", ""),
        "message": finding.get("message", ""),
        "rank_score": item.get("rank_score"),
    }


def main() -> int:
    src = REPO_ROOT / "data/phase0/labelling.json"
    dst = REPO_ROOT / "data/phase0/labelling.csv"
    with src.open() as f:
        items = json.load(f)
    # Sort: detector_id then descending rank_score so the rater hits the
    # highest-priority finding per detector first.
    items.sort(key=lambda it: (it["finding"]["detector_id"], -(it.get("rank_score") or 0)))
    rows = [build_row(i + 1, it) for i, it in enumerate(items)]
    fieldnames = list(rows[0].keys())
    with dst.open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=fieldnames, quoting=csv.QUOTE_ALL)
        w.writeheader()
        w.writerows(rows)
    print(f"wrote {len(rows)} rows -> {dst.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
