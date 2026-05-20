# scan

`cntrdct scan <PATH>` is the primary entry point. It walks a directory
or file, parses each supported-language source through tree-sitter,
runs all six Layer 1 detectors, applies the Layer 2 ranker, optionally
routes the top-N findings through the Layer 3 LLM adjudicator, and
emits the result as JSON or SARIF 2.1.0.

## Synopsis

```sh
cntrdct scan ./src                              # JSON to stdout
cntrdct scan ./src --format sarif > out.sarif   # SARIF to file
cargo cntrdct scan ./src                        # cargo subcommand shim
```

| Flag | Default | Effect |
|---|---|---|
| `--format <json\|sarif>` | `json` | Output format. JSON pretty-prints `Vec<RankedFinding>` (carries `rank_score`, `posterior_tp`, `wilson_lower`). SARIF emits OASIS 2.1.0 ordered by descending `rank_score`. |
| `--config <PATH>` | scan-root `cntrdct.toml` | Override the project config. Missing file is silently treated as an empty config. |
| `--priors <PATH>` | per-user cache → embedded | Override the Layer 2 priors JSON. The full fallback chain is `--priors` → `<cache_dir>/cntrdct/priors.json` → embedded default → uncalibrated. |
| `--no-calibration` | off | Force the uncalibrated ranker (sibling-count ordering) regardless of priors availability. |
| `--adjudicate` | off | Route the top-N findings through Layer 3 LLM. Requires `ANTHROPIC_API_KEY`. Absence of the key prints a stderr note and continues without adjudication. |
| `--adjudicate-top <N>` | `5` | Number of top-ranked findings to send to the adjudicator. |

## Output shapes

JSON is the default. Each `RankedFinding` carries:

- `detector_id`, `primary`, `related`, `message`, `severity`,
  `anomaly_class`, `evidence` (verbatim from Layer 1).
- `rank_score` (Wilson / Jeffreys precision × log-sibling-count),
  `prior_method` (`"wilson"` or `"jeffreys"`), `posterior_tp`, and
  `wilson_lower` (Layer 2).
- `adjudication` (verdict, raw confidence, optional Platt-calibrated
  confidence, calibration_tag) when `--adjudicate` ran.

SARIF emission is validated against the OASIS 2.1.0 schema on every
CI run; `tool.driver.rules` carries one entry per detector ID per the
`cntrdct::ALL_DETECTOR_IDS` single source of truth (Q-4).

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Scan completed successfully (regardless of finding count). |
| 1 | Invalid arguments, path not found, or any `ScanError`. |

`scan` never panics on bad input; parse failures inside a single file
are logged to stderr and the file is skipped silently.

## Network boundary

`scan` itself is fully offline. Only the optional `--adjudicate` flag
opens a socket — and even then, only from
`src/adjudicator.rs::ReqwestClient`. The CI `network-isolation` job
runs `cntrdct scan` inside a Linux network namespace
(`sudo unshare --net`) on every push and PR; any unintended socket
open fails the job with `ENETUNREACH` / `EAI_*`. See
[Network access policy (P3)](../concepts/network.md) for the full
enforcement story.

## Worked example

```sh
$ cntrdct scan ./src --format json | jq '.[0]'
{
  "detector_id": "comment-code",
  "primary": { "file": "src/parser.rs", "start_line": 42, ... },
  "message": "doc comment claims 'panics' but implementation does not match",
  "severity": "Note",
  "anomaly_class": "Documentation",
  "rank_score": 0.657,
  "prior_method": "jeffreys",
  "posterior_tp": 0.71,
  "evidence": { "raw": { "pattern": "B", "trigger": "panics" }, ... }
}
```

## See also

- [calibrate](./calibrate.md) — rebuild the Layer 2 priors against
  your own labelled corpus.
- [eval](./eval.md) — precision / recall / F1 reports.
- [SARIF output](../integrations/sarif.md) — how to wire the SARIF
  emission into GitHub Code Scanning.
- Spec:
  [`docs/spec/cli-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/cli-v0.md).
