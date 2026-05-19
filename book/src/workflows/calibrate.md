# calibrate

`cntrdct calibrate` fits Layer 2 detector priors against a labelled
findings corpus and writes the result to a JSON file. It has two
modes:

## Default mode (detector priors)

```sh
cntrdct calibrate benchmarks/labelled-findings.jsonl \
    --output benchmarks/priors-default.json
```

Computes per-detector Wilson (n ≥ 30) or Jeffreys (n < 30) lower
bounds on precision. The default `--output` is the per-user cache
(`<cache_dir>/cntrdct/priors.json`). The shipped binary embeds
`benchmarks/priors-default.json` via `include_str!`, so a fresh
`cargo install cntrdct` carries calibrated priors out of the box.

## `--fit-platt` mode (LLM-confidence calibration, Q-12)

```sh
cntrdct calibrate --fit-platt <LABELLED_LLM_CONFIDENCE.jsonl> \
    --output benchmarks/llm-calibration/platt-default.json
```

Fits per-`(detector_id, anomaly_class)` Platt `(a, b)` parameters from
a labelled adjudication corpus. v0 ships an empty registry; the
in-binary `apply_llm_calibration` is a no-op fallback until a real
labelled corpus is fit.

## `--audit-recall` mode (Q-14)

```sh
cntrdct calibrate --audit-recall benchmarks/audit-corpus
```

Measures per-detector recall upper bounds against externally-sourced
bug catalogues (NVD / OSV / Semgrep / CodeQL / Clippy / rustc lint
testset / paper-appendix / upstream bug-fix commits). The audit is
recall-bias-counter-selected, sitting alongside `benchmarks/wild-corpus/`
whose self-selected provenance measures false-positive rate. Re-run at
release time per the release procedure in `CLAUDE.md`.

Specs:
[`docs/spec/ranker-v1.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/ranker-v1.md),
[`docs/spec/llm-calibration-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/llm-calibration-v0.md),
[`docs/spec/recall-audit-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/recall-audit-v0.md).
