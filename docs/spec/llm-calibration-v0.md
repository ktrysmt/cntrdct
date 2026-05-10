# cntrdct LLM calibration v0 spec

Status: active draft, approved for TDD implementation 2026-05-10.

Q-12 deliverable from `ROADMAP.md`. Replaces the LLM's self-reported
`calibration_tag: T<scaling factor>` (a per-response verbalised
calibration claim) with a post-hoc Platt scaling step fit per
`(detector_id, anomaly_class)` cell on labelled corpora.

Verbalised confidence is, at corpus scale, not better calibrated than
the raw LLM output (Spiess, Koohestani, Sergeyuk 2025; on the order of
24M IDE interactions). The companion Spiess et al. ICSE 2025 paper
shows that post-hoc calibration via Platt scaling lifts expected
calibration error (ECE) on code-LLM outputs without any model
retraining. The published evidence is what justifies removing the
verbalised `calibration_tag` from the prompt while replacing its role
with Platt scaling fit on labelled corpora.

## Scope

In scope:

- Module `src/llm_calibration.rs` exposing pure `fit_platt`,
  `apply_platt`, and `ece` helpers plus the serde shapes that back
  them.
- `cntrdct calibrate --fit-platt <CORPUS>` reading a JSONL of
  `LabelledLlmConfidence` rows and writing
  `benchmarks/llm-calibration/platt-default.json` (or `--output
  <PATH>`).
- `AdjudicationResult.calibrated_confidence: Option<f64>` plumbed
  through serde, the SARIF emitter, and the library orchestration
  helper `apply_llm_calibration`.
- Adjudicator prompt no longer asks the model for a
  `calibration_tag`; the response parser keeps reading the field as
  `Option<String>` for backwards compatibility with adjudication
  records produced before Q-12.
- `tests/calibration_ece.rs` end-to-end ECE acceptance test.
- Citations `platt-1999` and `spiess-koohestani-sergeyuk-2025` added
  to Layer 3.

Out of scope (v0):

- Isotonic regression / temperature scaling / beta calibration as
  alternative post-hoc methods (Q-13 may revisit comparators).
- Per-prompt-template recalibration tracking.
- Online updates from production traffic.
- Auto-detecting distribution shift in newly-collected
  adjudications.

## Functional requirements

### F1 — `AdjudicationResult.calibrated_confidence`

`core::AdjudicationResult` gains:

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub calibrated_confidence: Option<f64>,
```

`None` when no Platt parameters were available for the finding's
`(detector_id, anomaly_class)` cell. Consumers gracefully fall back
to the raw `confidence`.

### F2 — `LabelledLlmConfidence` (input row)

Input JSONL shape, one row per labelled adjudication:

```jsonc
{
  "detector_id": "clone-drift",
  "anomaly_class": "Logic",
  "raw_confidence": 0.85,
  "verdict": "TruePositive"
}
```

`verdict` reuses `calibration::Verdict`
(`TruePositive` / `FalsePositive`) so the corpus can be co-authored
with existing P-4 calibration work without inventing a parallel
vocabulary.

### F3 — Platt scaling

Implements Platt 1999 §2 pseudo-code with the regularised target
shift:

- Positives: `t+ = (N+ + 1) / (N+ + 2)`.
- Negatives: `t- = 1 / (N- + 2)`.

Optimisation: Newton-Raphson on the 2×2 Hessian of the log-likelihood
in `(A, B)` space, with line-search backtracking on each step. The
original algorithm converges in tens of iterations on the cell sizes
Q-12 targets (`N` in the low hundreds per cell).

Output: `PlattParams { a: f64, b: f64 }`. Calibrated probability is
`sigmoid(a * raw + b)`. The numerically stable `softplus`-style form
is used so the Hessian builder stays accurate near the saturation
edges.

Cells with `tp == 0` or `fp == 0` are still fit — the regularised
target shift handles the boundary by construction.

### F4 — `PlattRegistry`

`HashMap<PlattKey, PlattParams>` keyed by

```rust
pub struct PlattKey {
    pub detector_id: String,
    pub anomaly_class: AnomalyClass,
}
```

Serialised as a flat JSON object with composite keys
`<detector_id>:<anomaly_class>`, e.g. `clone-drift:Logic`. Flat,
not nested, so a PR diff over the file is line-oriented. Output is
sorted by composite key on write so the artefact is byte-stable
across runs.

### F5 — CLI: `cntrdct calibrate --fit-platt`

`cntrdct calibrate` gains:

- `--fit-platt` (bool flag) — switches the corpus interpretation
  from `LabelledFinding` (P-4 priors) to `LabelledLlmConfidence`
  (Platt fit).
- `--output <PATH>` semantics extend to Platt mode: when set, writes
  the Platt JSON there; otherwise writes
  `benchmarks/llm-calibration/platt-default.json` relative to the
  current directory (spec'd for repo-root invocation).

Either mode errors when the corpus is empty for the chosen
interpretation, so a typo on `--fit-platt` does not silently produce
an empty artefact.

### F6 — Embedded Platt defaults

`benchmarks/llm-calibration/platt-default.json` is included in the
binary via `include_str!`. v0 ships an empty object `{}`, signalling
"no built-in Platt parameters; gracefully fall through to raw
confidence." A future tag bump that fits Platt over a real corpus
replaces the file contents in the same shape.

### F7 — `apply_llm_calibration`

```rust
pub fn apply_llm_calibration(
    ranked: &mut [RankedFinding],
    platt: &PlattRegistry,
);
```

Walks every `RankedFinding` whose `adjudication` is `Some`, looks up
`(finding.detector_id, finding.anomaly_class)` in `platt`, applies
the sigmoid, and writes `calibrated_confidence`. Findings without an
adjudication are untouched. Findings whose cell has no Platt entry
receive `calibrated_confidence = None`. Idempotent — re-running with
the same registry produces the same outputs.

### F8 — Adjudicator prompt change

`build_prompt` no longer instructs the model to emit a
`calibration_tag`. The schema in the response trailer becomes:

```json
{"verdict": "...", "confidence": <0.0-1.0>, "rationale": "..."}
```

`parse_response` keeps reading `calibration_tag` as `Option<String>`
so adjudication records produced before Q-12 (whether persisted by
the CLI or replayed in tests) still parse cleanly. The struct field
stays on `AdjudicationResult` and continues to be omitted from
serialisation when `None`.

### F9 — SARIF surfacing

`adjudication_to_value` emits an extra key when set:

```json
{
    "verdict": "...",
    "confidence": 0.7,
    "rationale": "...",
    "calibration_tag": "T1.5",
    "calibrated_confidence": 0.62
}
```

`calibration_tag` and `calibrated_confidence` are both omitted when
their respective `Option` is `None`. The keys are inserted in the
order above.

### F10 — ECE acceptance test

`tests/calibration_ece.rs` builds a synthetic LLM-confidence corpus
on a constructed-pathology distribution (raw confidence is heavily
over-stated relative to the empirical base rate), splits 70/30 into
train/holdout, fits Platt on train, computes 10-bin ECE on the
holdout for raw vs calibrated, and asserts
`ece_calibrated < ece_raw` by a non-trivial margin.

The fixture is over-confidence-shaped because that is the documented
LLM failure mode (Spiess et al. ICSE 2025 §6) post-hoc calibration is
specifically designed to repair.

## Non-functional requirements

- N1. P3 unchanged. The new code is allocation-only; no network, no
  filesystem reads beyond the supplied paths, no LLM calls. Layer
  boundaries stay intact.
- N2. Determinism. Platt fit is deterministic — same training set
  produces byte-identical `PlattParams`. The unit-test suite pins
  this.
- N3. Backwards compatibility. Pre-Q-12 JSON / SARIF consumers see
  no change unless `calibrated_confidence` is set; the field is
  `skip_serializing_if = Option::is_none` and reads default to
  `None`. Pre-Q-12 priors files are unaffected (they live on a
  different schema entirely).

## References

- `platt-1999` — J. Platt, "Probabilistic Outputs for Support Vector
  Machines and Comparisons to Regularized Likelihood Methods",
  Advances in Large Margin Classifiers (MIT Press), 1999.
  Methodology source for the post-hoc sigmoid fit.
- `spiess-icse-2025` — already cited under Layer 3. Motivates
  replacing verbalised confidence with a post-hoc step.
- `spiess-koohestani-sergeyuk-2025` — C. Spiess, P. Koohestani,
  A. Sergeyuk, "Verbalized Confidence in IDEs: A Large-Scale
  Empirical Study", arXiv:2510.22614, 2025. Empirical evidence (~24M
  IDE interactions) that verbalised confidence is not better
  calibrated than raw output.
