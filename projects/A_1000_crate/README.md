# Track A — empirical study on crates.io top-N

## Research question

How often do the cntrdct Layer 1 patterns (`clone-drift`, `arg-swap`,
`comment-code`, `unreachable-after-terminator`, `config-interaction`,
and a future `pr-miner-rust`) actually fire on production Rust code,
and how does the firing distribution compare with `clippy`'s coverage?

The hypothesis is empirical, not methodological: Rust's borrow checker,
exhaustive pattern matching, and Result-based error handling are
expected to suppress some defect classes (e.g. data races) but leave
others untouched (e.g. argument-name swaps, comment/code drift across
refactors). The published data on this is for C / C++ / Java, not Rust.

## Existing infrastructure to lean on

- `cntrdct scan <path> --format json` already produces structured
  findings.
- `cntrdct eval` already computes precision/recall/F1 against a
  labelled manifest.
- `cntrdct calibrate` already produces Layer 2 priors from a labelled
  JSONL corpus.
- The seed corpus under `benchmarks/corpus/` documents the manifest
  format.
- The prereg `prereg/2026-05-03-osf-prereg.md` documents Wilson lower
  bound, divide-by-zero conventions, and exclusion rules. Most of its
  Methods section is reusable.

What is missing:

- A crawler / sampler that pulls the top-N crates from the crates.io
  index (db dump, or `crates.io` Sparse Index) and unpacks their
  source tarballs.
- A clippy harness that runs `cargo clippy --message-format=json` on
  each crate and ingests the diagnostic stream so cntrdct vs clippy
  overlap can be computed.
- A stratified sampling tool that picks N findings per detector for
  manual labelling.
- A labelling UI or workflow (Markdown checklist, Google Sheet, or a
  small Egui app — no decision yet).
- A statistical analysis script (Python / R) for Wilson intervals,
  Cohen's kappa across raters, agreement scores.

## Method outline

Phase 0 — pilot (1-2 weekends).

- Fetch top 100 crates by all-time download count from crates.io.
- Run cntrdct + clippy on each.
- Compute raw firing counts per detector and per overlapping clippy
  lint. NO precision claim yet.
- Manually skim 30-50 findings to gauge signal-to-noise. Decide
  whether to scale.

Phase 1 — full study (3-6 months part-time).

- Scale to top 1000 (or whatever pilot suggests is feasible).
- Stratified random sample 30-50 findings per detector (200-300 total).
- Two human raters label each as TP / FP / Uncertain. Compute kappa.
- Wilson 95 percent intervals on per-detector precision.
- For each detector, classify the sampled FPs into failure-mode
  categories (e.g. for clone-drift: "boilerplate without bug",
  "intentional drift", "cross-file context required"). Tabulate.
- Cross-reference with clippy: for each cntrdct finding, check whether
  any clippy lint flagged the same source location. Compute overlap
  matrix.

Phase 2 — write-up.

- Tool description (4-6 pages).
- Empirical results (4-6 pages: prevalence, precision, clippy
  overlap, per-crate distribution, threats to validity).
- Compare with published baselines (Cordy & Roy NiCad, Rice 2017,
  Tan et al. iComment) only as exploratory context per prereg.

## Effort estimate

- Phase 0: 1-2 weekends.
- Phase 1: 3-6 months part-time, dominated by manual labelling. Two
  raters reduce wall time but not total person-hours.
- Phase 2: 1-2 months part-time for a draft, plus revision cycles.

## Venue targets

Primary candidates (in descending fit):

- MSR (Mining Software Repositories) — perfect topical fit; tool +
  empirical results + replication package.
- ICSME — empirical study with tooling angle, more lenient on
  novelty than ICSE.
- SANER — tool track or Industry track.
- ESEM — empirical software engineering, methodology-friendly.
- Rust-specific: the Rust Verification Workshop (RVW) co-located
  with formal-methods conferences accepts tool reports.

ICSE / FSE main tracks are unlikely without a stronger novelty pitch
and ought not to be the first target.

## First concrete step (when this track is picked up)

Build the crates.io fetcher.

- Pin the crates.io index commit hash so the sample is reproducible.
- Use the Sparse Index (https://index.crates.io/) for metadata and
  the static.crates.io tarball server for sources.
- Output: a directory `corpus/wild/<crate>-<version>/src/**/*.rs`
  per crate, plus a `corpus/wild/manifest.csv` that records crate,
  version, license, download count, and SHA-256 of the source
  tarball.
- Keep license filtering (MIT, Apache-2.0, BSD-3-Clause, ISC) in
  the fetcher itself; never pull GPL crates into the analysis
  corpus.
- Cap per-crate file count (e.g. skip auto-generated files larger
  than 50 KB, exclude `target/` and `tests/` for v0).

After the fetcher, run a single end-to-end scan on the top 100 to
generate raw counts. Decide pilot direction from there.

## Open questions to resolve before scaling

- Two raters or one? Kappa requires two; budget says one.
- Do we include `tests/` directories? They have different defect
  distributions and can dominate the corpus.
- How do we handle generated code (build.rs output, derive macros'
  expansions)? Probably exclude.
- Does Layer 3 LLM adjudication get used for labelling, only for
  eval, or not at all in this study?
- How do we handle proprietary or mirrored crates that share source
  with public ones (deduplicate by source SHA)?

## Dependencies on other tracks

- Track C (position paper) provides the methodological argument that
  motivates this study. Pairing them strengthens both. Doing A first,
  then C, also works.
- Practical track item 2 (`pr-miner-rust`) is in scope: if it ships
  before phase 1 starts, it gets included in the empirical evaluation.
- Practical track items 3-4 (SARIF validator, ranker calibration) do
  not block A but the calibration step output (priors.json) is itself
  data that could feed the empirical study.
