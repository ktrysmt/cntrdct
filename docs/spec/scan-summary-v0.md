# Scan summary v0 spec (S-1)

Status: implemented. Owner of the S-1 track. This document is the spec
referenced by `src/lib.rs::render_scan_summary`.

## Background

A linter's perceived return is "what did it just do for me, and why
should I trust it". Before S-1, `cntrdct scan` answered neither on a
normal run: stdout is a machine-oriented JSON/SARIF document and the
calibrated precision priors that ranked the findings (P4) were
invisible unless the user opened the priors file. S-1 surfaces both,
per run, at zero cost.

## Behaviour

Every `cntrdct scan` prints a summary to STDERR (stdout stays a clean
JSON / SARIF document for pipes):

```
scan summary: 3 finding(s) across 2 detector(s) in 450 file(s)
baseline: 9 known finding(s) suppressed; 3 new
  arg-swap                        2  est. precision >= 0.79 (jeffreys lower bound, n=17 labelled)
  clone-drift                     1  (no calibration data)
```

- Line 1 — total findings, distinct detectors, files scanned (the
  parsed-file count, i.e. after language filtering).
- The `baseline:` line appears only when `--baseline` was given
  (B-1), so an empty output cannot be misread as a clean codebase.
- One line per detector with at least one finding, sorted by
  detector id. The precision column is read VERBATIM from the
  resolved `DetectorPrior` map — the same map handed to the Layer 2
  ranker — showing `wilson_lower_95` with its `prior_method` label
  (`wilson 95% lower bound` at n >= 30, `jeffreys lower bound`
  below) and `n = tp + fp` labelled corpus rows.

## P4 discipline

The summary never derives or hardcodes a probability. `main.rs` calls
`resolve_priors` exactly once and feeds the identical map to both
`ranker_from_priors` and `render_scan_summary`, so the numbers shown
are the numbers that ranked the findings. Runs without priors
(`--no-calibration`, missing files, uncovered detector) render
`(no calibration data)` rather than a guess.

## Non-goals (v0)

- A `--no-summary` flag: stderr is the conventional channel for notes
  (`note:`, adjudication messages already live there) and no consumer
  parses cntrdct's stderr.
- Recall numbers: per-detector recall upper bounds exist (Q-14) but
  are audit-corpus artefacts, not per-run facts; conflating them with
  a per-run summary would overstate what a single scan measured.

## Tests

- Unit: `src/lib.rs` tests (`summary_shows_corpus_derived_precision_per_detector`,
  baseline line, zero-findings shape).
- Integration: `tests/scan_summary.rs` (stderr content with a pinned
  priors file, `--no-calibration` fallback, clean-scan summary,
  stdout stays parseable).
