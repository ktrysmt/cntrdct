#!/usr/bin/env bash
#
# 04_baseline.sh — the B-1 ratchet workflow end to end: record a
# baseline, prove a clean rescan, then surface only a NEW finding
# through the baseline with `--fail-on` enforcement.
#
# Run from the workspace root:
#   ./examples/04_baseline.sh
#
# Expected output: three annotated stages ending with "exit=3" for the
# run that introduces a new contradiction, and "exit=0" for the
# baselined runs.
#
# Exit code: 0 on success (the demo asserts the expected exit codes
# itself), non-zero if cntrdct misbehaves.

set -euo pipefail

cd "$(dirname "$0")/.."

WORK=$(mktemp -d -t cntrdct_baseline.XXXXXX)
trap 'rm -rf "$WORK"' EXIT

# A file with one pre-existing contradiction (unreachable statement).
printf 'fn f() { return; bar(); }\n' > "$WORK/legacy.rs"

echo "== stage 1: adopt — record the baseline =="
cargo run --quiet --bin cntrdct -- \
    scan "$WORK" --write-baseline "$WORK/cntrdct-baseline.json" > /dev/null

echo "== stage 2: ratchet — rescan is clean, --fail-on passes =="
cargo run --quiet --bin cntrdct -- \
    scan "$WORK" --baseline "$WORK/cntrdct-baseline.json" --fail-on warning > /dev/null
echo "exit=0 (all findings are known)"

echo "== stage 3: regression — a NEW contradiction fails the run =="
printf 'fn g() { return; qux(); }\n' > "$WORK/fresh.rs"
set +e
cargo run --quiet --bin cntrdct -- \
    scan "$WORK" --baseline "$WORK/cntrdct-baseline.json" --fail-on warning > /dev/null
code=$?
set -e
echo "exit=$code (the new finding, and only it, fails the scan)"
test "$code" -eq 3
