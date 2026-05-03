#!/usr/bin/env bash
#
# 03_adjudicate_mock.sh — exercise the Layer 3 adjudicator end-to-end against
# a local mock of the Anthropic Messages API.
#
# Run from the workspace root:
#   ./examples/03_adjudicate_mock.sh
#
# How it works:
# - Spawns a Python http.server that returns a canned `{"content":[{"text":...}]}`
#   body shaped like the real Anthropic Messages response.
# - Sets ANTHROPIC_API_KEY (any non-empty value, the mock ignores it) and
#   ANTHROPIC_API_URL_OVERRIDE so cntrdct's adjudicator hits the local mock.
# - Runs `cntrdct scan --adjudicate` over benchmarks/corpus/files and prints
#   the resulting JSON.
#
# Expected output: a JSON array where the top-N findings carry an
# `adjudication` object with verdict "LikelyTruePositive" and the canned
# rationale. The mock is intentionally unconditional; this example proves
# the wiring, not the model's judgement.
#
# Exit code: 0 on success.

set -euo pipefail

cd "$(dirname "$0")/.."

PORT="${PORT:-18081}"
MOCK_LOG=$(mktemp -t cntrdct_mock.XXXXXX.log)
trap 'kill "$MOCK_PID" 2>/dev/null || true; rm -f "$MOCK_LOG"' EXIT

python3 - <<'PY' "$PORT" >"$MOCK_LOG" 2>&1 &
import json, sys
from http.server import BaseHTTPRequestHandler, HTTPServer

PORT = int(sys.argv[1])

CANNED = {
    "content": [
        {
            "type": "text",
            "text": json.dumps({
                "verdict": "LikelyTruePositive",
                "confidence": 0.81,
                "rationale": "Mock adjudicator: the finding matches the cited drift pattern.",
                "calibration_tag": "T1.0"
            }),
        }
    ]
}

class H(BaseHTTPRequestHandler):
    def do_POST(self):
        body = json.dumps(CANNED).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_):
        pass

HTTPServer(("127.0.0.1", PORT), H).serve_forever()
PY
MOCK_PID=$!
disown "$MOCK_PID" 2>/dev/null || true

# Wait for the mock to bind.
for _ in $(seq 1 50); do
    if curl -s -o /dev/null -w '%{http_code}' \
        -X POST "http://127.0.0.1:${PORT}/" \
        -H 'content-type: application/json' --data '{}' \
        | grep -q '^200$'; then
        break
    fi
    sleep 0.1
done

ANTHROPIC_API_KEY="mock-key-not-validated" \
ANTHROPIC_API_URL_OVERRIDE="http://127.0.0.1:${PORT}/v1/messages" \
    cargo run --quiet --bin cntrdct -- \
        scan benchmarks/corpus/files \
        --format json \
        --no-calibration \
        --adjudicate \
        --adjudicate-top 2 \
    | head -c 6000
echo
