# Statistical priors (P4)

The Layer 2 ranker scores findings by a Wilson or Jeffreys lower
bound on detector precision multiplied by a log-scaled
sibling-finding count. The priors come from labelled corpora —
never from prompts, never from hardcoded constants. This is the P4
design constraint.

## Where priors come from

The pipeline lives in `src/calibration.rs` plus `src/ranker.rs`:

- `cntrdct calibrate <CORPUS>` reads a JSONL of `LabelledFinding`
  rows and writes a `HashMap<String, DetectorPrior>` to disk.
- The default labelled corpus
  (`benchmarks/labelled-findings.jsonl`) is hand-labelled across
  the seed and wild β corpora.
- `benchmarks/priors-default.json` is the output of `cntrdct
  calibrate` against that corpus, embedded into the binary via
  `include_str!` so a fresh `cargo install cntrdct` ships with
  calibration ready.

The uncalibrated ranker continues to ship `None` for the
calibration columns when no corpus is available, rather than
guessed values — P4 explicitly rules out hand-authored numbers
even as defaults.

## Runtime fallback chain

`cntrdct scan` picks the ranker by:

1. If `--no-calibration` is set, use `UncalibratedRanker`.
2. Otherwise, if `--priors <PATH>` is set, load that file. A
   missing file errors out; a parsed file produces
   `CalibratedRanker`.
3. Otherwise, if the default user-cache path exists, use
   `CalibratedRanker`.
4. Otherwise, fall through to the embedded
   `benchmarks/priors-default.json`.
5. As a final fallback, `UncalibratedRanker` (silent, no warning).

## Wilson and Jeffreys, switched at n = 30

Below cell size `n = TP + FP = 30` the calibrator switches from the
Wilson 95% lower bound to a Beta(1, 1) Bayes-Laplace 2.5% quantile.
The boundary at `TP = 0` applies the Brown-Cai-DasGupta 2001 §4
modification (return 0) so Jeffreys agrees with Wilson at the most
common observation-free cell.

| Cell size | Method | Notes |
|---|---|---|
| `n >= 30` | Wilson | `PriorMethod::Wilson`. Byte-stable with pre-Q-11 priors files. |
| `n < 30`, `TP > 0` | Jeffreys (Beta credible interval) | `PriorMethod::Jeffreys`. 2.5% quantile of `Beta(TP+1, FP+1)`. |
| `n < 30`, `TP = 0` | Jeffreys with BCD 2001 boundary | Returns 0. |
| `n = 0` | Wilson convention | Returns 0. |

The chosen method is recorded on each finding as `prior_method`
(`RankedFinding.prior_method`, SARIF
`result.properties.priorMethod`) so the choice is auditable
downstream.

## Why Jeffreys at small N

The roadmap framing "Jeffreys is closer to nominal than Wilson at
n < 30" is a useful shorthand but does not hold robustly under
one-sided lower coverage averaged over `p`. With the BCD 2001
boundary correction, both methods sit similarly close to nominal at
small `n`. The actual reason for switching is methodological
coherence:

- `posterior_tp` is already a Beta(1, 1) Bayesian update; pairing it
  with a Beta(1, 1) credible-interval lower bound at small `n`
  keeps both columns in one regime. Mixing the two would surface
  incompatible uncertainty semantics within the same
  `DetectorPrior`.
- At `n >= 30` Wilson and the Bayes-Laplace bound agree to several
  decimals, so the choice is irrelevant; Wilson is retained for
  byte-stability with pre-Q-11 priors files.

See [`docs/spec/ranker-v1.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/ranker-v1.md)
"Q-11 design notes" for the full argument.

## Rank score formula

For each finding `f`, the calibrated ranker computes:

```text
rank_score = wilson_or_jeffreys_lower(detector_id)
             * (1.0 + log2(1.0 + related.len()))
```

The lower bound replaces the raw TP rate, so a rare-but-precise
detector cannot dominate before enough evidence accumulates
(Z-Ranking, Kremenek & Engler SAS 2003). The log-sibling factor is
monotone and sub-linear in group size, rewarding corroboration
without letting one huge clone group flood the top of the ranking.

## Q-12 LLM confidence calibration

The Q-12 helper (`apply_llm_calibration` in
`src/llm_calibration.rs`) applies post-hoc Platt scaling to the
Layer 3 adjudicator's verdict confidence, per `(detector_id,
anomaly_class)` cell. Platt parameters are fit by
`cntrdct calibrate --fit-platt <CORPUS>` against a labelled
adjudication corpus and embedded as
`benchmarks/llm-calibration/platt-default.json`.

This module sits in the deterministic Layer 2 / Layer 4 surface,
not Layer 3 — `apply_llm_calibration` only post-processes whatever
the adjudicator returned, with no LLM call. v0 ships the registry
empty (`{}`), so the helper is a no-op until a real labelled
adjudication corpus is fit. The motivating evidence is Spiess,
Koohestani & Sergeyuk (2025), which shows that verbalised
confidence is not better calibrated than raw output at corpus
scale, and the companion Spiess et al. ICSE 2025 paper, which
demonstrates that post-hoc Platt scaling lifts expected calibration
error on code-LLM outputs.

See also:

- [`docs/spec/ranker-v1.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/ranker-v1.md)
  — ranker contract and Q-11 design notes.
- [`docs/spec/llm-calibration-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/llm-calibration-v0.md)
  — Q-12 Platt calibration spec.
- [Workflows: calibrate](../workflows/calibrate.md)
  — CLI usage for both priors and Platt modes.
