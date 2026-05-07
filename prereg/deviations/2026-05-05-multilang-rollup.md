# Deviation log: 2026-05-05 — multi-language rollup

Prereg: `prereg/2026-05-05-osf-prereg.md`
Supersedes: `prereg/2026-05-03-osf-prereg.md`
Author: ktrysmt
Date: 2026-05-05

## Summary

Folds the M-1 / M-2 / M-3 (partial) multi-language milestones into the
β preregistration without changing the Rust-side hypothesis surface.
`unreachable-after-terminator`, `comment-code`, and `arg-swap` now
accept Python source in addition to Rust. The β corpus and all
hypothesis statements remain Rust-scoped at the date of this
revision.

## Sections changed in `prereg/2026-05-03-osf-prereg.md`

- §Background: extended to name the Rust + Python detector surface
  introduced by M-1 (language abstraction foundation) and M-2 (Python
  pilot for `unreachable-after-terminator`). Project tagline updated
  to "Rust + Python".
- §Hypotheses: H1–H5 statements unchanged in wording. The detector
  set they range over is unchanged; the Rust corpus they range over
  is unchanged.
- §Sampling Plan: explicit "Rust-only at this revision" note added.
  Python β corpus collection is deferred to ROADMAP M-4 and a future
  preregistration; this revision's hypotheses do not range over
  Python source.
- §Variables: per-finding `LanguageCitationStatus` field surfaced as
  a recorded variable. Values: `Confirmed` for `arg-swap` Python
  (Allamanis, Jackson-Flux, Brockschmidt — NeurIPS 2021);
  `Unconfirmed` for `unreachable-after-terminator` and `comment-code`
  Python per the surveys under `docs/surveys/`.
- §References: gains `allamanis-neurips-2021`. Existing Rust
  citations carry forward unchanged.

## Sections unchanged

- H1–H5 hypothesis statements (verbatim).
- Layer 2 / Layer 3 / Layer 4 architecture and metric choices.
- Inference criteria (precision floor, recall floor, F1 reporting).
- Eval harness reference (`docs/spec/eval-v0.md`).

## Rationale

The Phase D multi-language extensions ship before the β corpus is
collected. Without this rollup the most-recent dated preregistration
would describe a Rust-only surface that no longer matches the live
detector code, defeating the preregistration's purpose at β time.
The Rust hypotheses are kept in-place because the M-3 Python work
does not change Rust detection — only widens the language surface.

## Evidence

- `docs/spec/multilang-v0.md` (M-1 / M-2 / M-3 architecture)
- `docs/surveys/unreachable-after-terminator-python-2026-05.md`
- `docs/surveys/comment-code-python-2026-05.md`
- `docs/spec/citations-policy.md` clauses (a)/(b)/(c) for the
  per-language citation grounding tests
