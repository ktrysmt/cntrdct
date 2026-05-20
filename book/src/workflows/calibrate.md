# calibrate

`cntrdct calibrate` fits statistical artefacts against a labelled
corpus. It has three modes, all behind the same subcommand:

| Mode | Flag | Spec |
|---|---|---|
| Detector priors (default) | none | [`ranker-v1.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/ranker-v1.md) |
| LLM-confidence Platt fit | `--fit-platt` | [`llm-calibration-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/llm-calibration-v0.md) |
| Recall audit | `--audit-recall` | [`recall-audit-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/recall-audit-v0.md) |

## Default mode — detector priors (P4)

```sh
cntrdct calibrate benchmarks/labelled-findings.jsonl \
    --output benchmarks/priors-default.json
```

Reads a JSONL of labelled findings (each row carries
`detector_id`, `is_true_positive`, optionally `anomaly_class` and
`evidence`) and writes one prior per detector. The Q-11 small-N
switch picks the lower-bound method automatically:

- `tp + fp ≥ 30`: Wilson 95% lower bound (`prior_method = "wilson"`).
- `tp + fp < 30`: Beta(1, 1) Bayes-Laplace 2.5% lower bound
  (`prior_method = "jeffreys"`), with the BCD 2001 §4 boundary
  modification at `tp = 0`.

Default `--output` is `<cache_dir>/cntrdct/priors.json` (the per-user
cache). The shipped binary embeds `benchmarks/priors-default.json`
via `include_str!`, so a fresh `cargo install cntrdct` carries
calibrated priors out of the box. The fallback chain at scan time is
`--priors` → per-user cache → embedded default → uncalibrated.

## `--fit-platt` mode — LLM-confidence calibration (Q-12)

```sh
cntrdct calibrate --fit-platt llm-confidence.jsonl \
    --output benchmarks/llm-calibration/platt-default.json
```

Fits post-hoc Platt scaling parameters `(a, b)` per
`(detector_id, anomaly_class)` cell from a JSONL of
`LabelledLlmConfidence` rows (`raw_confidence`, `is_correct`,
`detector_id`, `anomaly_class`). The fitted registry is consumed by
the in-binary helper `apply_llm_calibration`, which populates
`AdjudicationResult.calibrated_confidence` and the SARIF
`result.properties.adjudication.calibrated_confidence` field.

v0 ships an empty registry; the helper is a no-op fallback until a
real labelled adjudication corpus is fit. On the constructed
pathology fixture under `tests/calibration_ece.rs` (over-confidence
at 0.95 / 0.85 / 0.75 raw with empirical accuracy ≈ 0.5), raw ECE
0.256 drops to ≈ 0.001 after Platt.

## `--audit-recall` mode — Q-14 recall audit

```sh
cntrdct calibrate --audit-recall benchmarks/audit-corpus
```

The positional argument is a *directory* in this mode (not a JSONL
file). The directory must contain a `manifest.jsonl` listing
expected findings sourced from external bug catalogues (NVD / OSV /
Semgrep / CodeQL / Clippy / rustc lint testset / paper-appendix /
upstream bug-fix commits). Output defaults to stdout; pass
`--output <PATH>` to write to disk.

The audit is recall-bias-counter-selected per Heckman & Williams
(IST 2011), sitting alongside the self-selected
`benchmarks/wild-corpus/` whose provenance measures the false-
positive rate. The v0.4.3 audit closed at overall recall_upper_bound
0.66 raw 0.6557, with `comment-code` saturating all three Tan SOSP
2007 patterns at 34 / 0 / 1.00 across twenty-three permissive-
licensed upstreams. Per `CLAUDE.md`'s release procedure, re-running
this audit at every release tag is good hygiene when detector logic
has changed.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Calibration completed; output written. |
| 1 | Invalid arguments, missing corpus / manifest, or fit failure. |

## See also

- [scan](./scan.md) — consumes the priors at runtime via the
  fallback chain.
- [Statistical priors (P4)](../concepts/priors.md) — concepts-level
  explainer of what the calibrator produces and why.
- [eval](./eval.md) — for measuring detector quality against a
  labelled corpus without rebuilding priors.
