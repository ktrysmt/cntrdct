#!/usr/bin/env bash
#
# 01_scan.sh — scan the bundled benchmarks/corpus and pretty-print the first
# few ranked findings as JSON.
#
# Run from the workspace root:
#   ./examples/01_scan.sh
#
# Expected output: a JSON array of `RankedFinding` objects, ordered by
# `rank_score` descending, including at least one entry from the seed
# corpus' positive cases (e.g. a `clone-drift` or `arg-swap` finding).
#
# Exit code: 0 on success, non-zero if cntrdct itself fails.

set -euo pipefail

cd "$(dirname "$0")/.."

OUT=$(mktemp -t cntrdct_scan.XXXXXX.json)
trap 'rm -f "$OUT"' EXIT

cargo run --quiet --bin cntrdct -- \
    scan benchmarks/corpus/files \
    --format json \
    --no-calibration > "$OUT"

head -c 4000 "$OUT"
echo
