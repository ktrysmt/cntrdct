#!/usr/bin/env python3
"""Phase 1 FP failure-modes aggregator.

Reads a Phase 1 labelling CSV with a ``failure_mode`` column populated
per ``failure-modes-v1.md`` section 2.1 and emits a Markdown summary
covering per-detector cross-tabs of failure-mode counts plus a flat
list of ``other`` rows so reviewers can spot patterns that may justify
a v1.x mode promotion (per failure-modes-v1.md section 5).

Schema (CSV columns, header row required):

    id, detector_id, consensus_label, failure_mode, failure_mode_notes

Additional columns may be present and are ignored. Only rows where
``consensus_label`` equals the literal ``FP`` contribute to the
aggregation; per failure-modes-v1.md section 2.1, ``failure_mode`` is
recorded only on FP rows.

Validation, fixed by this script:

- ``failure_mode`` must be either the empty string, the literal
  ``"other"``, or one of the values declared for the row's
  ``detector_id`` in
  ``phase1_failure_modes.FAILURE_MODES_BY_DETECTOR``. Any other value
  raises ``ValueError``.
- A row whose ``failure_mode`` is the empty string or ``"other"`` is
  permitted regardless of whether ``detector_id`` appears in the
  vocabulary mapping. This makes the aggregator robust to detectors
  that have not yet been promoted into the vocabulary while still
  flagging structural data errors.
- Rendering does NOT validate ``consensus_label`` (that is the
  precision analyser's responsibility); it filters by equality with
  the literal ``"FP"`` only.
"""

from __future__ import annotations

import argparse
import csv
import sys
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

from phase1_failure_modes import FAILURE_MODES_BY_DETECTOR
from phase1_kappa_wilson import file_sha256

OTHER = "other"
EMPTY_DISPLAY = "(empty)"
NOTES_EXCERPT_CHARS = 200


@dataclass(frozen=True)
class Row:
    id: str
    detector_id: str
    consensus_label: str
    failure_mode: str
    failure_mode_notes: str


def _validate(detector: str, mode: str, line: int) -> None:
    if mode == "" or mode == OTHER:
        return
    allowed = FAILURE_MODES_BY_DETECTOR.get(detector)
    if allowed is None:
        raise ValueError(
            f"row {line}: detector_id={detector!r} not in controlled "
            "vocabulary; failure_mode must be empty or 'other' for "
            "unknown detectors"
        )
    if mode not in allowed:
        raise ValueError(
            f"row {line}: failure_mode={mode!r} not permitted for "
            f"detector_id={detector!r}; expected one of "
            f"{sorted(allowed) + [OTHER]} or empty"
        )


def parse_rows(path: Path) -> list[Row]:
    with path.open() as f:
        reader = csv.DictReader(f)
        required = {
            "id",
            "detector_id",
            "consensus_label",
            "failure_mode",
            "failure_mode_notes",
        }
        missing = required - set(reader.fieldnames or [])
        if missing:
            raise ValueError(f"missing required columns: {sorted(missing)}")
        rows: list[Row] = []
        for n, r in enumerate(reader, start=2):
            mode = r["failure_mode"]
            _validate(r["detector_id"], mode, n)
            rows.append(
                Row(
                    id=r["id"],
                    detector_id=r["detector_id"],
                    consensus_label=r["consensus_label"],
                    failure_mode=mode,
                    failure_mode_notes=r["failure_mode_notes"],
                )
            )
        return rows


def _excerpt(notes: str, limit: int = NOTES_EXCERPT_CHARS) -> str:
    flat = notes.replace("\n", " ").replace("\r", " ").strip()
    if len(flat) <= limit:
        return flat
    return flat[: limit - 1] + "…"


def render_summary(rows: list[Row], input_path: Path) -> str:
    fp_rows = [r for r in rows if r.consensus_label == "FP"]

    by_det_mode: dict[str, dict[str, int]] = defaultdict(
        lambda: defaultdict(int)
    )
    for r in fp_rows:
        bucket = r.failure_mode if r.failure_mode != "" else EMPTY_DISPLAY
        by_det_mode[r.detector_id][bucket] += 1

    lines: list[str] = []
    lines.append("# Phase 1 FP failure-modes summary")
    lines.append("")
    lines.append("## Meta")
    lines.append("")
    lines.append(f"- input: `{input_path.name}`")
    lines.append(f"- input_sha256: `{file_sha256(input_path)}`")
    lines.append(
        f"- generated_at_utc: {datetime.now(timezone.utc).isoformat()}"
    )
    lines.append("- script: `phase1_failure_modes_aggregate.py`")
    lines.append(f"- total_rows: {len(rows)}")
    lines.append(
        f"- fp_rows: {len(fp_rows)} "
        "(consensus_label == FP; only these contribute)"
    )
    lines.append("")
    lines.append("## Per-detector cross-tab")
    lines.append("")
    if not by_det_mode:
        lines.append("(no FP rows)")
        lines.append("")
    else:
        for det in sorted(by_det_mode):
            lines.append(f"### {det}")
            lines.append("")
            lines.append("| failure_mode | count |")
            lines.append("| --- | --- |")
            modes = by_det_mode[det]
            for mode in sorted(modes):
                lines.append(f"| {mode} | {modes[mode]} |")
            lines.append("")

    lines.append("## Other rows (candidates for v1.x promotion)")
    lines.append("")
    other_rows = [r for r in fp_rows if r.failure_mode == OTHER]
    if not other_rows:
        lines.append("(no rows with failure_mode = other)")
    else:
        lines.append("| id | detector | notes excerpt |")
        lines.append("| --- | --- | --- |")
        for r in other_rows:
            lines.append(
                f"| {r.id} | {r.detector_id} | "
                f"{_excerpt(r.failure_mode_notes)} |"
            )
    lines.append("")
    lines.append("## Conventions")
    lines.append("")
    lines.append(
        "- Only rows where consensus_label == FP contribute. Other "
        "rows are excluded per failure-modes-v1.md section 2.1."
    )
    lines.append(
        "- Empty failure_mode on an FP row indicates round-2/3 "
        "adjudication has not assigned a mode yet; it is shown as "
        f"{EMPTY_DISPLAY} in the cross-tab."
    )
    lines.append(
        "- Rows with failure_mode = other are listed flat with their "
        "notes excerpts. Per failure-modes-v1.md section 2.3, a v1.x "
        "file should be opened when 5 or more 'other' rows describe "
        "the same pattern."
    )
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(
        description="Phase 1 FP failure-modes aggregator",
    )
    ap.add_argument(
        "input",
        type=Path,
        help=(
            "Phase 1 labelling CSV with failure_mode and "
            "failure_mode_notes columns"
        ),
    )
    ap.add_argument(
        "-o",
        "--output",
        type=Path,
        help=(
            "Output Markdown path "
            "(default: <input parent>/phase1-failure-modes-summary.md)"
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
        "phase1-failure-modes-summary.md"
    )
    out.write_text(render_summary(rows, args.input))
    print(f"wrote {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
