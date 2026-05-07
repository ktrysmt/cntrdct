# Deviation log: 2026-05-06 — clone-drift Python extension

Prereg: `prereg/2026-05-06-osf-prereg.md`
Supersedes: `prereg/2026-05-05-osf-prereg.md`
Author: ktrysmt
Date: 2026-05-06

## Summary

Closes the M-3 sub-task by adding Python support to `clone-drift`.
The detector count is unchanged (five Layer 1 detectors); the language
coverage rises to four-of-five accepting Python. Hypotheses are
unchanged.

## Sections changed in `prereg/2026-05-05-osf-prereg.md`

- §Background: language coverage updated from "three-of-five accept
  Python" to "four-of-five accept Python" (`config-interaction`
  remains Rust-only by design — Pattern B per
  `docs/spec/multilang-v0.md`).
- §Hypotheses: H1–H5 statements unchanged in wording. Range over the
  same Rust corpus.
- §Variables: `LanguageCitationStatus` for `clone-drift` Python
  added with value `Confirmed`. Grounding citation:
  `assi-tosem-2025` (independent peer-reviewed application of NiCad
  and SourcererCC to nine Python deep-learning frameworks).
- §Variables / detector parameters: `MIN_FN_TOKENS = 22` introduced
  on `clone-drift` to filter trivially short utility functions.
  Multi-language and exposed as `pub const`. Documented in
  `docs/spec/clone-drift-v0.md`.
- §References: gains `assi-tosem-2025`. Existing clone-drift
  citations (`cordy-roy-icpc-2008`, `bettenburg-msr-2009`,
  `krinke-icsm-2007`) carry forward unchanged; per
  `docs/spec/citations-policy.md` clause (b) the new `assi-tosem-2025`
  paper grounds Python on the same algorithm family the Rust
  citations cover.

## Sections unchanged

- H1–H5 hypothesis statements (verbatim).
- Sampling plan: Rust corpus only; Python β corpus collection
  remains deferred to ROADMAP M-4 / a future prereg.
- Layer 2 / Layer 3 / Layer 4 architecture.
- Inference criteria (precision floor, recall floor, F1 reporting).
- The other four detectors' citations and language coverage.

## Rationale

`clone-drift` was the last detector still flagged Rust-only after the
2026-05-05 rollup. Closing M-3 in a single deviation entry keeps the
dated audit trail aligned with the multi-language rollout. The new
`MIN_FN_TOKENS` tunable matches industrial NiCad / SourcererCC
practice and is exposed as `pub const` so future calibration on the
Python β corpus (M-4) can adjust it without API churn.

## Evidence

- `docs/surveys/clone-drift-python-2026-05.md` (Python literature
  survey)
- `docs/spec/clone-drift-v0.md` (`MIN_FN_TOKENS` rationale)
- Citation: Assi, Hassan, Zou — ACM TOSEM 2025
  (DOI 10.1145/3721125)
