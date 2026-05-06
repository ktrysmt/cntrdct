# Track B — Rice 2017 replication in Rust

## Research question

Rice et al., "Detecting Argument Selection Defects" (ICSE 2017),
reported industrial-scale measurements of argument-name swap defects in
Google's C++ and Java monorepo. Does the same defect class appear at a
similar rate in Rust code, and does cntrdct's `arg-swap` detector
capture it with comparable precision?

The replication question matters because Rust's stronger type system
might naturally suppress some swap defects (e.g. when the two parameters
have distinct types, the type checker rejects the swap before it
reaches a code reviewer). The Rice 2017 paper notes that 38 percent of
their swap candidates were ruled out by type checking; in Rust the rate
should be higher. By how much?

## Existing infrastructure to lean on

- `cntrdct-detector-arg-swap` already implements the Rice 2017
  detection rule (parameter-name vs argument-name match, exactly two
  parameters, simple identifier arguments).
- `docs/spec/arg-swap-v0.md` records the detection contract.
- The seed corpus has 10 positive arg-swap cases (`arg_swap_001` ...
  `arg_swap_010`) which can serve as a sanity test set during
  replication.
- `crates/eval` and the manifest format support precision/recall/F1
  computation directly.

What is missing:

- Rice 2017's evaluation methodology spelled out at the level of
  "exactly which numbers we will compute". The paper reports several
  metrics; the replication should pick a faithful subset and document
  the choice.
- A Rust corpus large enough to surface a non-trivial number of
  arg-swap candidates. Rice 2017 used a Google-internal corpus with
  hundreds of millions of lines. We will need at least 10K crates to
  get a comparable per-defect-class density. (Or a smaller corpus
  with bootstrap intervals to acknowledge low precision in the
  estimate.)

## Current status (2026-05-06)

Step 1 (formalise replication scope) has produced two artefacts:

- `replication-spec-v0.md` — DRAFT scaffold. Contains 12
  `[verify Rice §X]` placeholders that are unresolved without
  paper access. Tracked as USR-3 in `PLAN.md`. Once resolved by
  a reader with paper access, the scaffold is promoted to
  `replication-spec-v1.md` with a `Supersedes:` header pointing
  at v0; v0 is retained verbatim as the audit trail of "what was
  claimed before reading the paper".
- `replication-spec-v1.1-rust-analyzer.md` — DRAFT addendum
  fixing the optional rust-analyzer-based type filter sketched
  in v0 §4.1. Specifies LSP-over-stdio integration (over
  ra_ap_* direct invocation), display-string equivalence
  predicate with documented normalisation, and per-corpus
  type-distinct fraction with Wilson 95% CI as the headline
  output. Composes with v1 once v1 lands.

Step 2 (corpus assembly) is unblocked: the Track A fetcher
(`scripts/fetch_rust_corpus.py` at repo root) is reusable.

Steps 3-5 wait on USR-3 (paper read + v1 promote). Once v1 lands,
the technical-side patch series specified in v1.1 §5 begins:

1. `feat(detector-arg-swap)`: RICE_TRACE trace path.
2. `feat(corpus)`: 5 type-distinct seed fixtures.
3. `promote(track-b)`: `cntrdct-research rice-types` LSP client.
4. `promote(track-b)`: `cntrdct-research rice-aggregate` (also
   covers the v0 §9 deferred aggregator scope).

## Method outline

Step 1 — formalise replication scope.

- Re-read Rice 2017 in detail. Pick a subset of their reported
  metrics that are reproducible without their proprietary tooling.
  Candidates: candidate-call count per KLOC, candidate-call density
  per file size bucket, post-type-check survival rate, manual
  precision on a stratified sample.
- Decide which Rice 2017 measurements are NOT applicable to Rust
  (e.g. their language-specific filters for Java/C++) and document
  the substitution.

Step 2 — assemble the Rust corpus.

- Reuse the Track A fetcher if available, otherwise duplicate it.
- Aim for 1000-5000 crates depending on time budget. The defect
  density is low, so the absolute corpus size matters more than for
  Track A.

Step 3 — run cntrdct's `arg-swap` detector across the corpus.

- Capture all candidate calls (not just the swap-confirmed ones).
  This requires extending the detector with a debug-mode emission
  path that surfaces "I considered this call but it didn't match the
  swap rule because [reason]" entries. The reason taxonomy is
  important for replication.
- The detector additions are non-invasive: the public Detector
  interface stays the same, the candidate-emission path is gated
  behind a `RICE_TRACE=1` env flag or a separate API
  (`detect_with_trace`).

Step 4 — manual validation.

- Stratified sample of 100-200 swap-confirmed findings.
- Two raters classify TP / FP / Uncertain.
- Compute Wilson 95 percent precision.

Step 5 — write up.

- Compare per-detector firing rates with Rice 2017's published
  numbers as exploratory context (different languages, different
  corpora — direct comparison is invalid for confirmatory claims).
- Quantify the type-check survival rate and contrast with Rice 2017's
  38 percent.

## Effort estimate

- Step 1: 1-2 weekends.
- Step 2: 1-2 weekends (or zero, if Track A's fetcher is reused).
- Step 3: 1-2 weeks of part-time effort to add the trace path and
  run the corpus.
- Step 4: 2-4 weeks of manual labelling.
- Step 5: 1-2 months for a draft.

Total: 3-5 months part-time.

## Venue targets

- ICSE Replication track — explicitly receptive to replication
  studies.
- EMSE (Empirical Software Engineering journal) Replication and
  Reproducibility section — patient venue, longer page budget.
- MSR — also accepts replications.
- Rust Verification Workshop — short-form report option.

## First concrete step (when this track is picked up)

Re-read Rice et al. (ICSE 2017) and write a 2-page "replication
specification" that lists, for each metric in the original paper,
either the Rust-applicable analogue or the substitution rule. Save as
`research/projects/B_rice_replication/replication-spec-v0.md`. This document
becomes the contract under which the replication runs and is the
single most leveraged artefact in this track.

## Dependencies on other tracks

- Track A's fetcher is reusable.
- The practical track's release of arg-swap stays unchanged through
  this replication; the trace path is gated behind a flag and does
  not affect default behaviour.
- Track C (position paper) and Track B can be paired — replication
  reinforces the methodological argument.
