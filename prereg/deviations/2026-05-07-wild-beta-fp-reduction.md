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
    inference), F5c (within-scope tightening via strict-majority
    + Jaccard ≥ NEAR_DUPLICATE_THRESHOLD = 0.7), and F5d
    (sibling-family discriminator: F5d-i multi-singleton suppression,
    F5d-ii weak-dominant length-imbalance gate, F5d-iii small-cluster
    floor) — F5d landed the same day as a follow-up residual cleanup
    (ROADMAP P-7). Spec: `docs/spec/clone-drift-v0.md` F5b / F5c /
    F5d. Tests: `tests/detector_clone_drift.rs` t20–t28 (F5b / F5c)
    and t29–t31 / t30b (F5d).
- §Variables / embedded priors: `cntrdct calibrate` is byte-stable
  (BTreeMap-sorted before write). Embedded priors recompute against
  the relabelled wild β corpus: `clone-drift` Wilson lower 0.073 →
  0.355 → 0.676 (the second jump from F5d removing 5 wild β FP rows
  and 0 TP rows, leaving 8 TP / 0 FP for clone-drift),
  `unreachable-after-terminator` 0.407 → 0.796, `comment-code`
  0.298 → 0.657. Source: `src/lib.rs::calibrate`.
- §Sampling Plan: wild β corpus manifests are relabelled per
  `prereg/2026-05-04-labelling-rubric-v0.md`.
  - `benchmarks/wild-corpus/manifest.jsonl`: no expected entries
    added (the previous manifest already had zero TPs). After F5d
    the 3 Rust clone-drift FP rows (`syn__lib.rs:961`,
    `tracing_subscriber__layer_mod.rs:1547`, `uuid__fmt.rs:280`)
    no longer appear in `labelled-findings.jsonl` because the
    underlying scan no longer fires.
  - `benchmarks/wild-corpus-python/manifest.jsonl`:
    `charset_normalizer_utils.py:27 clone-drift` reclassified TP →
    FP per rubric §5.1 FP-1 (different conceptual role: accent-
    property detector vs single-predicate script/category-detector
    siblings). The pre-relabel TP at this position is no longer
    flagged anyway because F5c-ii filters it. After F5d the 2
    Python clone-drift FP rows (`charset_normalizer_utils.py:70`,
    `:194`) similarly no longer appear in `labelled-findings.jsonl`
    — F5d-i suppresses the parent cluster.

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
0.073 → 0.676 is a 9.3× shift across the P-6 + P-7 chain) and the
ranker output is downstream of those priors; readers comparing pre-
and post-2026-05-07 reports need an explicit pointer to the
recalibration event. The single corpus relabel is in-protocol:
rubric §5.1 FP-1 applies and the labelling rubric was frozen on
2026-05-04 before the relabel was performed. The F5d sub-gates
(P-7) close the 5 documented v0 limitations carried by the
2026-05-06 prereg's "designed library shapes" footnote without
adding citations or new detectors; they are detector-internal
precision tightenings of the same shape as F5b / F5c.

## Evidence

- `docs/spec/unreachable-after-terminator-v0.md` F4b / F4c
- `docs/spec/comment-code-v0.md` F5b / F5c
- `docs/spec/clone-drift-v0.md` F5b / F5c / F5d
- `prereg/2026-05-04-labelling-rubric-v0.md` rubric §5.1 FP-1
- `benchmarks/priors-default.json` (post-recalibration values)
