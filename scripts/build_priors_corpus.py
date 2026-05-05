#!/usr/bin/env python3
"""
Derive a labelled JSONL corpus for `cntrdct calibrate` from one or more
eval-style corpus directories (each containing `manifest.jsonl` plus a
`files/` subtree).

For each finding cntrdct produces on the corpus, we look up
`(detector_id, file, line)` in the manifest's `expected` array:
  - match → `verdict: TruePositive`
  - no match → `verdict: FalsePositive`

This is the same TP / FP convention `cntrdct eval` uses, so the
labelled corpus and the eval report stay consistent by construction.

Usage:
  python3 scripts/build_priors_corpus.py \\
      --corpus benchmarks/corpus \\
      --corpus benchmarks/wild-corpus-python \\
      --out benchmarks/labelled-findings.jsonl

Inputs are passed via repeatable `--corpus <dir>`; each dir must
contain `manifest.jsonl` and `files/`. The output is one JSONL row
per produced finding, in the order the scan emitted them.

Why a script rather than a Rust binary: the labelling is pure data
glue that does not need cntrdct's parser graph at runtime — `cntrdct
scan` already has been run by the time this script processes its JSON
output. Keeping it in Python avoids a circular dependency between the
detector workspace and the calibration training data.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Dict, Iterable, List, Set, Tuple


def load_expected(manifest_path: Path) -> Dict[str, Set[Tuple[str, int]]]:
    """Return per-file set of expected (detector_id, line) pairs."""
    out: Dict[str, Set[Tuple[str, int]]] = {}
    with manifest_path.open() as f:
        for raw in f:
            line = raw.strip()
            if not line or line.startswith("//"):
                continue
            entry = json.loads(line)
            file_rel = entry["file"]
            out.setdefault(file_rel, set())
            for exp in entry.get("expected", []):
                out[file_rel].add((exp["detector_id"], exp["line"]))
    return out


def run_scan(corpus_dir: Path) -> List[dict]:
    """Run `cntrdct scan --no-calibration --format json` on the corpus and
    return the parsed JSON array of RankedFinding records.

    `--no-calibration` is critical: we are building the corpus that
    feeds calibration, so the scanner must not consult any priors.
    """
    files_dir = corpus_dir / "files"
    if not files_dir.is_dir():
        raise SystemExit(f"corpus has no files/ subdir: {corpus_dir}")

    binary = "target/debug/cntrdct"
    if not Path(binary).exists():
        binary = "cntrdct"  # fall back to PATH

    proc = subprocess.run(
        [binary, "scan", str(files_dir), "--no-calibration", "--format", "json"],
        capture_output=True,
        text=True,
        check=True,
    )
    return json.loads(proc.stdout)


def label_for(
    rf: dict, expected: Dict[str, Set[Tuple[str, int]]], corpus_dir: Path
) -> Tuple[str, str, int, str]:
    """Return (detector_id, repo_relative_file, line, verdict)."""
    f = rf["finding"]
    detector_id = f["detector_id"]
    abs_file = Path(f["primary"]["file"])
    line = int(f["primary"]["start_line"])

    files_dir = (corpus_dir / "files").resolve()
    try:
        rel = abs_file.resolve().relative_to(files_dir)
        manifest_key = f"files/{rel.as_posix()}"
    except ValueError:
        manifest_key = abs_file.as_posix()

    pairs = expected.get(manifest_key, set())
    verdict = "TruePositive" if (detector_id, line) in pairs else "FalsePositive"
    return detector_id, manifest_key, line, verdict


def label_corpus(
    corpus_dir: Path, repo_name: str, anomaly_class_by_detector: Dict[str, str]
) -> Iterable[dict]:
    expected = load_expected(corpus_dir / "manifest.jsonl")
    for rf in run_scan(corpus_dir):
        f = rf["finding"]
        detector_id, file_rel, line, verdict = label_for(rf, expected, corpus_dir)
        anomaly_class = (
            f.get("anomaly_class") or anomaly_class_by_detector.get(detector_id)
        )
        row: dict = {
            "detector_id": detector_id,
            "repo": repo_name,
            "file": file_rel,
            "line": line,
            "verdict": verdict,
        }
        if anomaly_class:
            row["anomaly_class"] = anomaly_class
        yield row


def main(argv: List[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--corpus",
        action="append",
        required=True,
        help=(
            "Path to a corpus directory containing manifest.jsonl and files/. "
            "Repeatable; the rows are concatenated in order."
        ),
    )
    parser.add_argument(
        "--out",
        required=True,
        help="Output JSONL path (overwritten if it exists).",
    )
    args = parser.parse_args(argv[1:])

    # Detector → anomaly class mapping. Pulled from each detector's
    # registered metadata at compile time; for the data-only script we
    # mirror it explicitly. Drift between this map and the Rust source
    # is detected by `cargo test --test citations_consistency`.
    anomaly_class_by_detector = {
        "clone-drift": "Logic",
        "arg-swap": "Interface",
        "comment-code": "Documentation",
        "unreachable-after-terminator": "Logic",
        "config-interaction": "Logic",
    }

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)

    rows: List[dict] = []
    for raw in args.corpus:
        corpus_dir = Path(raw)
        repo_name = corpus_dir.name  # "corpus", "wild-corpus-python", ...
        print(f"labelling {corpus_dir}", file=sys.stderr)
        n_before = len(rows)
        rows.extend(label_corpus(corpus_dir, repo_name, anomaly_class_by_detector))
        print(f"  {len(rows) - n_before} findings", file=sys.stderr)

    with out_path.open("w") as f:
        for r in rows:
            f.write(json.dumps(r, sort_keys=True) + "\n")

    print(f"wrote {len(rows)} rows to {out_path}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
