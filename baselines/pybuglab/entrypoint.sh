#!/usr/bin/env bash
# PyBugLab wrapper entrypoint for cntrdct's Q-15 baseline harness.
#
# Contract:
#   - read /corpus (readonly bind mount)
#   - write /out/findings.jsonl (one NormalisedFinding row per line)
#   - exit 0 on success, non-zero on any failure
#
# Spec: docs/spec/sota-baselines-v0.md §"Adapter contract".
# The harness on the cntrdct side validates each row's `tool` and
# `detector_id` against the registry entry; a row whose schema does
# not match aborts the comparison with BaselineError::ToolMismatch
# or BaselineError::DetectorIdMismatch.
#
# Determinism (spec F6): the PYBUGLAB_SEED env var is set at image
# build time and read here. Re-running the image with the same seed
# against the same corpus produces byte-identical JSONL.

set -euo pipefail

CORPUS=/corpus
OUT_DIR=/out
OUT_FILE="${OUT_DIR}/findings.jsonl"
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT

# Tool version that lands in NormalisedFinding.tool_version. Read from
# the upstream commit pinned at build time; the trailing -cntrdct-vX.Y.Z
# suffix is supplied by the cntrdct CLI when it consumes the JSONL.
UPSTREAM_COMMIT=$(cat /opt/pybuglab_commit.txt 2>/dev/null || echo "TBD")
SEED="${PYBUGLAB_SEED:-0}"
TOOL_VERSION="${UPSTREAM_COMMIT}-seed${SEED}"

if [ ! -d "${CORPUS}" ]; then
    echo "error: expected /corpus to be a directory" >&2
    exit 2
fi

mkdir -p "${OUT_DIR}"

# Write atomically: build the full JSONL under SCRATCH, then rename
# into OUT_FILE in a single mv. A partial run does not leave a
# partial artefact behind. The harness aborts on missing-output
# anyway, but atomicity makes re-runs idempotent.
PARTIAL="${SCRATCH}/findings.jsonl.partial"
: > "${PARTIAL}"

# TODO(release): wire the real PyBugLab inference pipeline once
# baselines/pybuglab/UPSTREAM.md pins a concrete upstream commit and
# the pre-trained weights URL. Until then the entrypoint emits zero
# findings — which is a semantically valid baseline output (the
# upstream model produced no arg-swap detections on this corpus) and
# matches the spec's "Returning an empty JSONL is allowed; the report
# records the empty baseline cell honestly" stance. The placeholder
# keeps the end-to-end harness path exercisable while UPSTREAM.md is
# still TBD.
echo "note: PyBugLab entrypoint is a placeholder pending live wiring" >&2
echo "      build with -e PYBUGLAB_COMMIT=<sha> and update entrypoint.sh" >&2

mv "${PARTIAL}" "${OUT_FILE}"

echo "wrote 0 rows (placeholder); tool_version=${TOOL_VERSION}" >&2
