# cntrdct implementation roadmap

Last updated: 2026-05-03 (commit `6e1cf21`; Phase A complete)

Engineering roadmap for shipping cntrdct as a usable open-source Rust
tool. Separate from the academic research tracks under `projects/A_*`,
`projects/B_*`, `projects/C_*` (see `projects/PLAN.md` for the strategic
context that produced this roadmap).

## Status legend

- `[x]` completed
- `[~]` in progress
- `[ ]` pending

## Practical track

Items that are not strictly OSS-readiness work but are critical
engineering deliverables that shape the public-facing v1. Done in
parallel with Tier 1 OSS readiness.

P-1. β corpus collection (real-world Rust crates)

- Status: `[ ]`
- Goal: replace the synthetic 58-file seed corpus with a labelled
  collection of at least 200 files drawn from public crates, kept
  separately under `benchmarks/wild-corpus/` so the regression seed
  stays untouched.
- Acceptance: `cntrdct eval benchmarks/wild-corpus` runs cleanly,
  per-detector precision and recall are both reported with non-trivial
  values (i.e. not pinned at 1.0), and the manifest cites source crate
  + license + SHA-256 for every file.
- Effort: 2-4 weeks part-time. The crawler can be reused for
  Track A's empirical study.
- Depends on: a fetcher (also useful for Track A).

P-2. pr-miner-rust detector

- Status: `[ ]`
- Goal: a sixth Layer 1 detector that mines implicit programming
  rules within a crate (e.g. "function `foo` is always called before
  `bar`") and flags violations. Cites Li & Zhou (FSE 2005) faithfully
  rather than a simplified single-pattern lift.
- Acceptance: `docs/spec/pr-miner-rust-v0.md` exists and is approved,
  the detector crate ships with an integration test suite mirroring
  the spec's test plan, `crates/cli/tests/citations_consistency.rs`
  is green with the new detector registered, and the seed corpus
  contains at least 8 positive cases for the new detector.
- Effort: 3-6 weeks part-time. Heavier than the existing five
  detectors because the detection rule is statistical (frequent
  itemset mining or a similar approximation) rather than syntactic.
- Depends on: nothing strictly; can be done in parallel with P-1.

P-3. SARIF output validation in CI

- Status: `[ ]`
- Goal: verify that `cntrdct scan --format sarif` produces output
  that passes `sarif-multitool validate` against the OASIS 2.1.0
  schema on every CI run.
- Acceptance: a GitHub Actions step downloads `sarif-multitool`,
  pipes a sample scan through it, and fails the build on any schema
  violation.
- Effort: 1-2 days.
- Depends on: T1-1 (GitHub Actions CI).

P-4. Layer 2 ranker recalibration on the β corpus

- Status: `[ ]`
- Goal: rerun `cntrdct calibrate` on the new β corpus and ship the
  resulting `priors.json` as the default cache.
- Acceptance: a `priors.json` file is committed under
  `benchmarks/priors-default.json` (or an analogous canonical
  location), `cntrdct scan` picks it up automatically, and the
  ranked output ordering changes meaningfully relative to the
  uncalibrated fallback on the seed corpus.
- Effort: 2-4 days.
- Depends on: P-1.

P-5. β release tagging and crates.io publish

- Status: `[ ]`
- Goal: tag the workspace as `v0.2.0-beta`, publish all crates to
  crates.io, push a GitHub Release with pre-built binaries.
- Acceptance: `cargo install cntrdct` works on a clean machine,
  the release page on GitHub shows binaries for at least
  Linux x86_64 / macOS aarch64 / macOS x86_64.
- Effort: 1 week including bug-fix iterations.
- Depends on: T1-1, T1-2, T1-3, T2-8.

## Tier 1 — usable OSS (blocking for first announcement)

T1-1. GitHub Actions CI

- Status: `[x]`
- Goal: every push to `master` and every PR runs the workspace
  test suite, clippy with warnings as errors, and rustfmt check on
  Linux and macOS.
- Acceptance: `.github/workflows/ci.yml` exists, runs
  `cargo test --workspace`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo fmt --all -- --check` on a Linux + macOS matrix; the
  workflow is green on the next master push.
- Effort: half a day.
- Depends on: nothing.

T1-2. crates.io metadata for every crate

- Status: `[x]`
- Goal: each `crates/*/Cargo.toml` has the metadata required for
  crates.io publish (`description`, `repository`, `keywords`,
  `categories`, `readme`, `license`).
- Acceptance: `cargo publish --dry-run -p <crate>` succeeds for
  every workspace member.
- Effort: half a day.
- Depends on: nothing.

T1-3. README polish for OSS audience

- Status: `[x]`
- Goal: the top-level `README.md` opens with a one-paragraph
  pitch, shows badges (CI, crates.io, docs.rs, license), gives a
  copy-pasteable quickstart, and links to a sample SARIF or
  screenshot. Include explicit constraint declarations (P1-P5
  one-liners) so readers immediately understand the project's
  stance.
- Acceptance: a fresh visitor can install and run cntrdct in
  under five minutes without reading any other file.
- Effort: half a day.
- Depends on: T1-1, T1-2.

T1-4. examples directory

- Status: `[x]`
- Goal: at least three runnable examples under `examples/`:
  scanning a crate, calibrating a corpus, adjudicating with a
  mock API key.
- Acceptance: each example is self-contained, has a top-of-file
  comment explaining purpose and expected output, and is invoked
  by a CI smoke-test step.
- Effort: half a day.
- Depends on: nothing.

T1-5. rustdoc on cntrdct-core public surface

- Status: `[x]`
- Goal: every public item in `cntrdct-core` carries a doc comment
  with at least a one-line summary and, where relevant, an example
  block.
- Acceptance: `cargo doc -p cntrdct-core --no-deps` produces clean
  output with no `missing_docs` warnings, after enabling
  `#![deny(missing_docs)]` at the crate root.
- Effort: 1-2 days.
- Depends on: nothing.

T1-6. LICENSE coverage review

- Status: `[x]`
- Goal: confirm the workspace is consistently licensed (currently
  MIT-only). If we add dependencies under different licenses,
  surface them in `LICENSES/` or a NOTICE file.
- Acceptance: `cargo deny check licenses` (or equivalent) passes.
- Effort: a few hours.
- Depends on: nothing.

T1-7. GitHub Pages essay site

- Status: `[~]`
- Goal: a Jekyll-based site at `https://<owner>.github.io/cntrdct/`
  that hosts the design-philosophy essays (Track C output) and serves
  as a permanent, citable home for them. Source under `docs/site/`,
  auto-deployed via GitHub Actions on every push to `master` that
  touches the site directory.
- Acceptance: a master push deploys the site within minutes, the
  landing page lists at least one essay, and the canonical URL is
  linked from the top-level `README.md`. The essay URL is stable and
  citable from code comments.
- Effort: 1 day.
- Depends on: T1-1 (the workflow lives alongside CI).

## Tier 2 — adoption-grade (drives external usage)

T2-7. Suppression mechanism

- Status: `[x]`
- Goal: in-source suppression via
  `#[cntrdct::allow(<detector_id>)]` attribute, plus a project-wide
  `cntrdct.toml` for severity remapping, threshold overrides, and
  per-path allow/deny rules.
- Acceptance: an integration test suite covers (a) attribute
  suppression on a single item, (b) a `cntrdct.toml` that disables
  one detector entirely, (c) a `cntrdct.toml` that raises
  `clone-drift` to error severity.
- Effort: 1-2 weeks. The attribute parsing is the unfamiliar work;
  config-file plumbing is straightforward.
- Depends on: nothing.

T2-8. Pre-built release binaries

- Status: `[ ]`
- Goal: GitHub Releases automatically attach binaries for
  Linux x86_64, Linux aarch64, macOS x86_64, macOS aarch64,
  Windows x86_64.
- Acceptance: a release tag triggers a workflow that uploads the
  binaries; `curl | sh` install instructions in the README work
  end-to-end.
- Effort: 2-3 days using `cargo-dist`. Hand-rolled is 1-2 weeks.
- Depends on: T1-1.

T2-9. GitHub Action wrapper

- Status: `[ ]`
- Goal: `cntrdct/action` (or an analogous repository) so that
  users can add `uses: cntrdct/action@v1` to their own CI and get
  cntrdct findings as PR comments.
- Acceptance: a sample external repo demonstrates the action;
  comment formatting matches GitHub Annotations conventions so
  findings appear inline in the diff view.
- Effort: 1 week.
- Depends on: T2-8 (binaries) so the action does not need to
  build cntrdct from source on every run.

T2-10. Parallel detection via rayon

- Status: `[x]`
- Goal: per-file detector runs execute in parallel.
- Acceptance: scanning a 1000-file crate is at least 4x faster on
  an 8-core machine compared to the serial baseline. Output
  ordering remains deterministic (sorted post-hoc).
- Effort: 2-3 days.
- Depends on: nothing.

T2-11. cargo cntrdct subcommand

- Status: `[ ]`
- Goal: the binary can be invoked as `cargo cntrdct scan` in
  addition to plain `cntrdct scan`.
- Acceptance: a `cargo-cntrdct` shim (typically a renamed binary
  shipped alongside `cntrdct`) is installed by
  `cargo install cntrdct` and recognised by `cargo`.
- Effort: half a day.
- Depends on: T1-2.

## Tier 3 — polish (post-launch)

T3-12. LSP server

- Status: `[ ]`
- Goal: a `cntrdct-lsp` crate that exposes findings to IDEs
  (VS Code, Helix, Neovim) via the Language Server Protocol.
- Acceptance: a `vscode-cntrdct` extension or comparable
  client surfaces findings inline.
- Effort: 4-6 weeks.

T3-13. mdBook user guide

- Status: `[ ]`
- Goal: a `book/` directory hosting user-facing documentation
  (concepts, detector reference, configuration, FAQ) built with
  mdBook and published to GitHub Pages.
- Acceptance: `book/book.toml` builds without errors, all
  pages have at least placeholder content, and the published
  URL is linked from the README.
- Effort: 2-3 weeks.

T3-14. Distribution channels beyond crates.io

- Status: `[ ]`
- Goal: Homebrew tap, AUR package, `cargo-binstall` metadata.
- Effort: 1-2 days each.
- Depends on: T2-8.

T3-15. Auto-generated changelog

- Status: `[ ]`
- Goal: `git-cliff` or `cocogitto` produces `CHANGELOG.md` from
  conventional commit messages on every release.
- Effort: half a day plus retroactive commit cleanup if desired.

T3-16. Telemetry-free assurance

- Status: `[ ]`
- Goal: explicit documentation that the binary makes no network
  calls except via `--adjudicate`. Reinforced by a CI test that
  runs the binary under a network namespace with no internet
  access (or a mock that fails any unexpected outbound call) and
  confirms `cntrdct scan` succeeds.
- Effort: 1-2 days.

## Tier 4 — community (opens contribution funnel)

T4-17. Issue templates

- Status: `[ ]`
- Goal: `.github/ISSUE_TEMPLATE/{bug_report,feature_request,detector_proposal}.md`.
  The detector proposal template should require a citation field
  upfront so contributors internalise P1 from the first interaction.
- Effort: half a day.

T4-18. PR template

- Status: `[ ]`
- Goal: `.github/PULL_REQUEST_TEMPLATE.md` with checkboxes for
  citations updated, corpus cases added, tests passing.
- Effort: an hour.

T4-19. CONTRIBUTING.md

- Status: `[ ]`
- Goal: a contributor guide covering detector authoring (link to
  the per-detector spec template), corpus case authoring, the
  citation format, the DCO-or-CLA decision, and the local dev
  loop (`cargo test --workspace`, `cargo clippy`, `cargo fmt`).
- Effort: half a day.

T4-20. Code of Conduct

- Status: `[ ]`
- Goal: `CODE_OF_CONDUCT.md` based on Contributor Covenant 2.1.
- Effort: 15 minutes.

T4-21. Roadmap discussion pinned

- Status: `[ ]`
- Goal: a GitHub Discussion thread that surfaces this roadmap
  and invites community input on prioritisation.
- Effort: 15 minutes.

## Suggested execution order

Phase A (Tier 1, ~4-6 days total):

1. T1-1 GitHub Actions CI
2. T1-5 rustdoc on `cntrdct-core`
3. T1-2 crates.io metadata
4. T1-4 examples directory
5. T1-3 README polish
6. T1-6 LICENSE review
7. T1-7 GitHub Pages essay site

Phase B (small announcement; concurrent with Phase C):

7. Public read-only repository, no marketing yet.

Phase C (Tier 2 in parallel with Practical track):

8. T2-10 rayon parallelisation
9. T2-7 suppression mechanism
10. T2-11 cargo subcommand
11. T2-8 pre-built binaries
12. P-1 β corpus collection (Practical track)
13. P-3 SARIF validator integration
14. T2-9 GitHub Action wrapper

Phase D (Practical track items that depend on Phase C):

15. P-4 ranker recalibration
16. P-2 pr-miner-rust detector
17. P-5 v0.2.0-beta release

Phase E (Tier 3 / 4 organically after launch):

18. T4-19 / T4-20 / T4-17 / T4-18 / T4-21 community scaffolding
19. T3-13 mdBook
20. T3-12 LSP server
21. T3-14 / T3-15 / T3-16 polish

The split between Phase A (Tier 1, blocking) and later phases is the
single most important boundary in this roadmap. Everything in Phase A
should be done before any external announcement; everything after
Phase A can be sequenced based on signal from early users.
