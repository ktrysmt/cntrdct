# cross-model-kappa

`cntrdct cross-model-kappa <CORPUS>` (Q-13) routes the same finding
set through Claude Code's `claude --print` and the Gemini CLI's
`gemini -p`, then reports pairwise Cohen's κ per
`(detector_id, anomaly_class)` cell. It is the on-demand audit that
measures inter-judge reliability of the Layer 3 adjudicator.

## Synopsis

```sh
cntrdct cross-model-kappa findings.jsonl
cntrdct cross-model-kappa findings.json --output audit.json
```

| Flag | Default | Effect |
|---|---|---|
| `--output <PATH>` | stdout | Write the audit JSON to disk. With this flag a one-line summary is also printed to stderr. |

The positional argument accepts either a JSONL of `RankedFinding`
rows or a JSON array — exactly the shape `cntrdct scan --format
json` emits, so the round-trip `scan | cross-model-kappa` composes
cleanly.

## Interpreting κ

κ is reported per `(detector_id, anomaly_class)` cell using the
Landis & Koch (1977) substantial-agreement floor:

| Range | Interpretation |
|---|---|
| κ < 0.0 | Worse than chance — judge disagreement systematic. |
| 0.0 – 0.20 | Slight agreement. |
| 0.21 – 0.40 | Fair agreement. |
| 0.41 – 0.60 | Moderate agreement. |
| 0.61 – 0.80 | Substantial agreement (the actionable floor). |
| 0.81 – 1.00 | Almost perfect agreement. |

Cells below 0.6 are flagged as low-reliability adjudication regions
and indicate the judges disagree often enough that downstream
consumers should treat the verdict as advisory rather than load-
bearing.

## Auth model

cntrdct does not read API keys for this subcommand. Both CLIs handle
their own auth via subscription / OAuth (`claude /login`, `gemini
auth`). At least two live providers are required to compute κ; a
missing or unauthenticated CLI surfaces as a `skipped` provider in
the audit JSON.

The CLI providers are spawned with `current_dir = <tempdir>` so
`CLAUDE.md` / `GEMINI.md` auto-discovery does not interfere, and the
agentic persona is stripped:

- `claude --print` is invoked with `--system-prompt` + `--tools ""`
  + `--strict-mcp-config` + `--no-session-persistence` +
  `--output-format json`.
- `gemini -p` is invoked with the `GEMINI_SYSTEM_MD` env override
  pointing at a temp file and `--output-format json`.

## Network boundary (P3)

`cross-model-kappa` is the one subcommand excluded from the
`network-isolation` CI gate. It does not open sockets *from*
cntrdct — it shells out to subprocesses that themselves talk to the
network. Same shape as `scan --adjudicate`. See
[Network access policy (P3)](../concepts/network.md).

## Why on-demand, not nightly

Continuous monitoring was dropped for measurement-stationarity
reasons. Commercial LLMs version-bump silently; sampler
stochasticity at temperature 0 still produces variance; and the
time-series κ would have captured noise more than any cntrdct-side
property. The audit ships as an on-demand snapshot only. The full
design rationale lives in
[`docs/spec/cross-model-kappa-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/cross-model-kappa-v0.md)
"Design rationale".

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Audit completed; report written. |
| 1 | Invalid arguments, fewer than two live providers, corpus parse error. |

## See also

- [scan](./scan.md) — produces input findings for the audit when
  combined with `--format json`.
- [calibrate](./calibrate.md) `--fit-platt` — the Q-12 LLM-
  confidence calibration is downstream of this κ audit (the audit
  measures reliability; the calibration corrects systematic
  miscalibration).
