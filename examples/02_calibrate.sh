#!/usr/bin/env bash
#
# 02_calibrate.sh — run `cntrdct calibrate` on a tiny labelled JSONL corpus
# and print the resulting per-detector priors.
#
# Run from the workspace root:
#   ./examples/02_calibrate.sh
#
# Expected output: a single line confirming "wrote priors for N detectors to
# <path>", followed by the JSON contents of that priors file. Each entry has
# a `posterior_tp` (Laplace-smoothed) and a `wilson_lower_95` (95% Wilson
# lower bound) per detector id.
#
# Exit code: 0 on success.

set -euo pipefail

cd "$(dirname "$0")/.."

OUT=$(mktemp -t cntrdct_priors.XXXXXX.json)
trap 'rm -f "$OUT"' EXIT

cargo run --quiet --bin cntrdct -- \
    calibrate examples/sample_corpus.jsonl --output "$OUT"

echo "--- $OUT ---"
cat "$OUT"
echo
