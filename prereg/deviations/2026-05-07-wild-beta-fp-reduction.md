# Deviation log: 2026-05-07 — wild β corpus FP reduction pass

Prereg: `prereg/2026-05-07-osf-prereg.md`
Supersedes: `prereg/2026-05-06-osf-prereg.md`
Author: ktrysmt
Date: 2026-05-07

## Summary

Records the wild β corpus FP reduction pass run on 2026-05-07 (P-6 in
ROADMAP). No new detectors and no new citations; the H1–H5 hypothesis
statements are unchanged. The deviations are detector-internal
precision tightenings backed by the same prior art set, plus the
resulting recalibration of the embedded priors and one corpus
relabel.

## Sections changed in `prereg/2026-05-06-osf-prereg.md`

- §Hypotheses: H1–H5 statements unchanged in wording. Range over the
  same Rust corpus and the same five-detector surface.
- §Detector specifications: three detectors gain new precision
  filters. None change the detector's IEEE 1044-2009 anomaly class,
  citations, or anomaly-detection contract.
  - `unreachable-after-terminator` adds F4b (cfg-gated terminator
    suppression) and F4c (hoisted item suppression). Spec:
    `docs/spec/unreachable-after-terminator-v0.md` F4b / F4c. Tests:
    `tests/detector_unreachable_after_terminator.rs` T29–T37.
  - `comment-code` adds F5b (Sphinx `:raises:` factory-shape
    suppression) and F5c (parameter-level `.. deprecated::`
    directive peeking). Spec: `docs/spec/comment-code-v0.md` F5b /
    F5c. Tests: `tests/detector_comment_code.rs` t21–t28b.
  - `clone-drift` adds F5b (scope-bounded clustering via path-only
    inference) and F5c (within-scope tightening via strict-majority
    + Jaccard ≥ NEAR_DUPLICATE_THRESHOLD = 0.7). Spec:
    `docs/spec/clone-drift-v0.md` F5b / F5c. Tests:
    `tests/detector_clone_drift.rs` t20–t28.
- §Variables / embedded priors: `cntrdct calibrate` is byte-stable
  (BTreeMap-sorted before write). Embedded priors recompute against
  the relabelled wild β corpus: `clone-drift` Wilson lower 0.073 →
  0.355, `unreachable-after-terminator` 0.407 → 0.796,
  `comment-code` 0.298 → 0.657. Source:
  `src/lib.rs::calibrate`.
- §Sampling Plan: wild β corpus manifests are relabelled per
  `prereg/2026-05-04-labelling-rubric-v0.md`.
  - `benchmarks/wild-corpus/manifest.jsonl`: no expected entries
    added (the previous manifest already had zero TPs).
  - `benchmarks/wild-corpus-python/manifest.jsonl`:
    `charset_normalizer_utils.py:27 clone-drift` reclassified TP →
    FP per rubric §5.1 FP-1 (different conceptual role: accent-
    property detector vs single-predicate script/category-detector
    siblings). The pre-relabel TP at this position is no longer
    flagged anyway because F5c-ii filters it.

## Sections unchanged

- H1–H5 hypothesis statements (verbatim).
- The five-detector Layer 1 surface and the language coverage matrix
  (`config-interaction` Rust-only, the other four Rust + Python).
- Layer 2 ranker selection, Layer 3 adjudicator selection, Layer 4
  SARIF emission shape.
- Inference criteria (precision floor, recall floor, F1 reporting).
- Layer 1 citation key set.

## Rationale

P-6 was an empirical re-tuning pass — the underlying anomaly-class
contract for each detector is identical to the 2026-05-06 revision.
The deviation is recorded here rather than left implicit because
small-n recalibration moves Wilson lower bounds visibly (clone-drift
0.073 → 0.355 is a 4.9× shift) and the ranker output is downstream
of those priors; readers comparing pre- and post-2026-05-07 reports
need an explicit pointer to the recalibration event. The single
corpus relabel is in-protocol: rubric §5.1 FP-1 applies and the
labelling rubric was frozen on 2026-05-04 before the relabel was
performed.

## Evidence

- `docs/spec/unreachable-after-terminator-v0.md` F4b / F4c
- `docs/spec/comment-code-v0.md` F5b / F5c
- `docs/spec/clone-drift-v0.md` F5b / F5c
- `prereg/2026-05-04-labelling-rubric-v0.md` rubric §5.1 FP-1
- `benchmarks/priors-default.json` (post-recalibration values)
