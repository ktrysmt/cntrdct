# cross-model-kappa

`cntrdct cross-model-kappa <CORPUS>` (Q-13) routes the same finding
set through Claude Code's `claude --print` and the Gemini CLI's
`gemini -p`, then reports pairwise Cohen's κ per `(detector_id,
anomaly_class)` cell.

```sh
cntrdct cross-model-kappa findings.jsonl
cntrdct cross-model-kappa findings.json --output audit.json
```

Cells with κ < 0.6 (Landis & Koch substantial-agreement floor) are
flagged as low-reliability adjudication regions. Both CLIs must be
installed and logged in (`claude auth`, `gemini auth`); a missing CLI
surfaces as a `skipped` provider in the audit JSON.

## Why on-demand, not nightly

Continuous monitoring was dropped for measurement-stationarity
reasons: commercial LLMs version-bump silently, sampler stochasticity
at temperature 0 still produces variance, and the time-series κ would
capture noise more than any cntrdct-side property. The audit ships as
an on-demand snapshot only — see the "Design rationale" section of
the spec.

## Auth model

cntrdct does not read API keys for this subcommand. Both CLIs handle
their own auth via OAuth. The two providers spawn with
`current_dir = <tempdir>` to suppress CLAUDE.md / GEMINI.md
auto-discovery and strip the agentic persona via
`--system-prompt` / `GEMINI_SYSTEM_MD`.

Spec:
[`docs/spec/cross-model-kappa-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/cross-model-kappa-v0.md).
