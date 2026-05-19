# Statistical priors (P4)

The Layer 2 ranker scores findings by a Wilson / Jeffreys lower bound
on detector precision, multiplied by a log-scaled sibling-finding
count. The priors come from labelled corpora — never from prompts or
hardcoded constants.

- Default priors are fit by `cntrdct calibrate` against
  `benchmarks/labelled-findings.jsonl` and embedded into the binary
  at `benchmarks/priors-default.json` via `include_str!`.
- The fallback chain at runtime is: explicit `--priors` → per-user
  cache → embedded default → uncalibrated.
- Below n = 30 the calibrator switches from Wilson to a Beta(1, 1)
  Bayes-Laplace 2.5% quantile, with a boundary modification at
  `tp = 0` (see Brown, Cai, DasGupta 2001 §4). The chosen method is
  recorded on each finding as `prior_method`.

The Q-12 LLM-confidence post-processing helper
(`apply_llm_calibration`) is a Layer 2 / Layer 4 concern, not Layer 3
— it is deterministic and only post-processes whatever the
adjudicator returned. v0 ships an empty Platt registry.

See also:
[`docs/spec/ranker-v1.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/ranker-v1.md),
[`docs/spec/llm-calibration-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/llm-calibration-v0.md).
