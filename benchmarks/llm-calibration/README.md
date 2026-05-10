# LLM calibration corpus and Platt parameters

Q-12 deliverable. Spec: `docs/spec/llm-calibration-v0.md`.

## `platt-default.json`

Per-cell Platt scaling parameters keyed by `<detector_id>:<anomaly_class>`.
Embedded into the binary via `include_str!` so a fresh
`cargo install cntrdct` ships with calibrated LLM confidence out of the
box, mirroring the P-4 priors pipeline.

v0 ships an empty object `{}`, signalling "no built-in Platt
parameters; gracefully fall through to raw confidence." A future tag
that fits Platt over a real labelled adjudication corpus replaces the
file contents in the same shape.

## Producing a registry

```sh
cntrdct calibrate --fit-platt <corpus.jsonl> \
  --output benchmarks/llm-calibration/platt-default.json
```

Input corpus JSONL shape (one row per labelled LLM adjudication):

```jsonc
{
  "detector_id": "clone-drift",
  "anomaly_class": "Logic",
  "raw_confidence": 0.85,
  "verdict": "TruePositive"
}
```

`verdict` reuses `calibration::Verdict`
(`TruePositive` / `FalsePositive`).
