# Addendum 1 to OSF Preregistration: Phase 0 single-rater pilot

Author: ktrysmt
Date: 2026-05-05
Project: cntrdct (Evidence-based contradiction linter for Rust)
Parent document: `prereg/2026-05-03-osf-prereg.md` (`osf-prereg-2026-05-03`)
Status: addendum, frozen on commit. Subsequent revisions go to a new
addendum file, not to in-place edits, so the audit trail of what was
committed when stays intact.

## 1. Scope

This addendum is a metadata supplement to the parent preregistration. It
does not modify the parent's hypotheses, sampling plan, metrics, or
inference criteria. It records two changes that arose between filing the
parent document and beginning the β corpus collection:

1. A separate, smaller pilot run (Phase 0) was carried out on the top-100
   crates by lifetime download count, using a single rater (the author).
   The pilot's role is methodological — to surface labelling ambiguities,
   exercise the harness end-to-end, and validate rater throughput — not
   to produce the figures the parent preregistration commits to.
2. A labelling rubric (`prereg/2026-05-04-labelling-rubric-v0.md`) was
   filed before any finding was rated. The rubric fixes per-detector
   TP / FP / Uncertain criteria for Phase 0 and forms the basis for the
   v1 rubric that the preregistered β study (50+ files, two raters,
   Cohen's κ) will use.

The β study figures referenced in §"Hypotheses" of the parent document
are produced under the parent's preregistered protocol. Phase 0 numbers
are exploratory and are reported as such.

## 2. Phase 0 vs preregistered β design

| Dimension                | Phase 0 pilot                                | β study (parent prereg)                       |
|--------------------------|-----------------------------------------------|------------------------------------------------|
| Corpus                   | top-100 crates.io by lifetime downloads       | 50+ labelled files, ≥8 positives per detector  |
| Rater configuration      | single rater (author)                         | author + Layer 3 LLM cross-check               |
| Disagreement protocol    | n/a (single rater)                            | written reconciliation under `benchmarks/disagreements.md` |
| Findings sampled         | 79 (per-detector cap = 30, seed = 42)         | full per-file labelling                        |
| Rubric                   | `2026-05-04-labelling-rubric-v0.md`           | v1 rubric derived from v0; two-rater wording   |
| Inter-rater agreement    | self-consistency on id ∈ {1..10}, target ≥ 90% | Cohen's κ ≥ 0.6                                 |
| Reporting role           | upper bound on precision; pilot diagnostics    | the figures preregistered in the parent       |

The Phase 0 LLM cross-check used in the β protocol is intentionally
deferred: the cntrdct codebase ships its own Layer 3 adjudicator and
running it during a pilot run by the same author who wrote the
detectors would produce a feedback loop the parent prereg's
"independently reviewed" wording is meant to forbid. The β study uses
the LLM as cross-check only, and only after the pilot's rubric and
sampling cap have been frozen.

## 3. Single-rater limitation (Phase 0 only)

The pilot's precision figures are an upper bound. The dominant residual
risk is rater-author confirmation bias: the rater wrote the detectors
under evaluation. Mitigations recorded in
`prereg/2026-05-04-labelling-rubric-v0.md` §6 are:

- detector-id is unavoidable (rubric is per-detector); rater commits to
  applying the rubric clauses rather than the detector message;
- `rank_score`, `message`, and `anomaly_class` columns are pushed to the
  far right of `data/phase0/labelling.csv` so they can be hidden in the
  spreadsheet view during rating;
- post-pass self-consistency check on id ∈ {1..10} with the original
  labels masked, target ≥ 90% identical labels, reported as a footnote
  in the Phase 0 summary.

Phase 0 numbers will be published in the β release notes as
"Phase 0 pilot, single-rater, upper-bound precision" alongside the
preregistered β figures, never as a substitute for them.

## 4. Reproducibility artefacts

Pinned paths, all relative to the repository root:

- `prereg/2026-05-03-osf-prereg.md` — parent preregistration.
- `prereg/2026-05-04-labelling-rubric-v0.md` — Phase 0 rubric, frozen on
  the first non-empty `label` row in `labelling-rated.csv`.
- `prereg/2026-05-05-osf-prereg-phase0-addendum.md` — this document.
- `scripts/phase0/build_labelling_csv.py` — produces
  `data/phase0/labelling.csv` (79 rows, sorted by detector_id then
  rank_score desc, anchor-bias columns rightmost) from
  `data/phase0/labelling.json`.
- `scripts/phase0/aggregate_labels.py` — reads
  `data/phase0/labelling-rated.csv` and emits the per-detector summary
  Markdown plus optional JSON, with input SHA-256, generation timestamp,
  and rubric path embedded in the metadata block. Validates labels
  against `{TP, FP, Uncertain}` and exits non-zero on missing labels.

`data/` is gitignored. The scripts under `scripts/phase0/` are tracked;
the data they consume and produce is regenerable from the public
crates.io daily DB dump and from `cntrdct scan` output.

## 5. What this addendum does not change

The parent preregistration's hypotheses (H1 – H5, H_overall), Wilson
lower-bound thresholds, sampling plan (50+ files, ≥8 positives per
detector, ≤30% true negatives), and inference criteria are unchanged.
Citation keys committed to in the parent's References section remain
authoritative. The Phase 0 pilot does not contribute observations to
the β corpus; the seed corpus exclusion in the parent's "Existing data"
section extends to Phase 0 by the same reasoning.

## 6. Stability

This addendum is frozen on commit. If a subsequent change to the Phase 0
protocol is needed before β collection begins, it is recorded as
`prereg/2026-MM-DD-osf-prereg-phase0-addendum-2.md` with a diff section
explaining what addendum 1 was insufficient to cover. The committed
copy of this file remains untouched so that the audit trail of what
was preregistered when is preserved.
