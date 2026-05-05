# cntrdct implementation roadmap

Last updated: 2026-05-03 (commit `6e1cf21`; Phase A complete)

Engineering roadmap for shipping cntrdct as a usable open-source Rust
tool. Separate from the academic research tracks under
`research/projects/A_*`, `research/projects/B_*`,
`research/projects/C_*` (see `research/projects/PLAN.md` for the
strategic context that produced this roadmap). The research-side
workspace at `research/Cargo.toml` is independent of the technical
workspace and never blocks technical CI.

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

- Status: `[x]`
- Goal: verify that `cntrdct scan --format sarif` produces output
  that passes `sarif-multitool validate` against the OASIS 2.1.0
  schema on every CI run.
- Acceptance: a GitHub Actions step downloads `sarif-multitool`,
  pipes a sample scan through it, and fails the build on any schema
  violation.
- Effort: 1-2 days.
- Depends on: T1-1 (GitHub Actions CI).

P-4. Layer 2 ranker recalibration on the β corpus

- Status: `[x]`
- Goal: rerun `cntrdct calibrate` on the new β corpus and ship the
  resulting `priors.json` as the default cache.
- Acceptance: a `priors.json` file is committed under
  `benchmarks/priors-default.json` (or an analogous canonical
  location), `cntrdct scan` picks it up automatically, and the
  ranked output ordering changes meaningfully relative to the
  uncalibrated fallback on the seed corpus.
- Effort: 2-4 days.
- Depends on: P-1.
- Delivered (v0):
  - `scripts/build_priors_corpus.py` derives a labelled JSONL from
    `(benchmarks/corpus, benchmarks/wild-corpus-python)` by running
    `cntrdct scan --no-calibration` and matching findings against
    each manifest's `expected` array. Output committed at
    `benchmarks/labelled-findings.jsonl` (87 rows: 69 TP / 18 FP).
  - `cntrdct calibrate benchmarks/labelled-findings.jsonl` produces
    `benchmarks/priors-default.json`. Per-detector wilson_lower_95
    spread: comment-code 0.32 (16 FP from attrs idioms) at the low
    end; arg-swap and unreachable-after-terminator at 0.80; clone-
    drift 0.67; config-interaction 0.68.
  - `cntrdct-cli` embeds `priors-default.json` at compile time via
    `include_str!`. New `pick_ranker` fallback chain:
    explicit `--priors` (kept silent on missing path for backwards
    compat) → per-user cache → embedded priors → uncalibrated.
  - Acceptance caveat: on the v0 seed corpus, sort permutation does
    NOT change between uncalibrated and calibrated rankers — every
    detector's wilson_lower happens to align monotonically with its
    findings' sibling-counts on this corpus, so the formula
    preserves the relative order. rank_scores DO differ
    (calibrated attaches posterior_tp / wilson_lower to every
    finding; uncalibrated leaves both `None`). The test
    `calibrated_ranker_reorders_when_wilson_disagrees_with_related_count`
    constructs the adversarial case explicitly to demonstrate the
    formula IS sensitive to wilson when the alignment breaks.
  - Tests added: `embedded_priors_are_used_when_no_override_or_cache`
    and the reorder demonstration above. The existing
    `pick_ranker_with_missing_priors_path_falls_back_silently`
    contract is preserved.

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

- Status: `[x]`
- Goal: GitHub Releases automatically attach binaries for
  Linux x86_64, Linux aarch64, macOS x86_64, macOS aarch64,
  Windows x86_64.
- Acceptance: a release tag triggers a workflow that uploads the
  binaries; `curl | sh` install instructions in the README work
  end-to-end.
- Effort: 2-3 days using `cargo-dist`. Hand-rolled is 1-2 weeks.
- Depends on: T1-1.

T2-9. GitHub Action wrapper

- Status: `[x]`
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

- Status: `[x]`
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

## Multi-language track (M-series)

Promotes cntrdct from a Rust-only linter to a multi-language one.
Strategic rationale: the differentiator (peer-reviewed citations on
every finding) is language-agnostic, and the commercial market for a
single-language linter is bounded. Pilot language is Python; the
architecture is built so subsequent languages (TypeScript, Go, Java)
plug in without rework.

This track interrupts Phase D — `P-2 pr-miner-rust detector` and
`P-4 ranker recalibration` are deferred until M-series completes so
new detectors are designed multi-language from day one rather than
retrofitted.

Constraint extension: P1 still binds. Each new language added to a
detector requires at least one peer-reviewed citation grounded in
empirical work on that target language; the existing Rust citation
does not transfer automatically. See `docs/spec/citations-policy.md`.

M-1. Language abstraction foundation

- Status: `[x]`
- Goal: introduce a `cntrdct-parsers` crate that owns the
  `Language` enum, the extension → language mapping, and a tree-sitter
  Parser provider. Migrate `ParsedFile.language` from a free-form
  `String` to the enum (or to a canonical-name set defined in
  `cntrdct-core`). Update CLI file walker to discover all supported
  languages, not just `.rs`.
- Acceptance: `cntrdct scan ./mixed-repo` parses files of every
  declared `Language` variant and produces a `ParsedFile` per file
  with the correct `language` field. Every existing Rust test stays
  green.
- Effort: 1-2 weeks part-time.
- Depends on: nothing.

M-2. Pilot Python detector

- Status: `[x]`
- Goal: extend the simplest detector
  (`unreachable-after-terminator`) to Python. Divergent terminators
  for Python: `raise`, `sys.exit()`, `os._exit()`, `assert False`,
  `return` followed by code in the same block. Add Python fixture
  cases under `benchmarks/corpus/files/` and at least one new
  citation grounded in Python static-analysis prior art.
- Acceptance: `cntrdct scan` over a Python source tree emits
  `unreachable-after-terminator` findings; the seed corpus contains
  ≥5 positive Python cases and ≥3 negative; the new citation key
  resolves in `CITATIONS.md`; `crates/cli/tests/citations_consistency.rs`
  is extended to enforce the policy from M-6.
- Effort: 1 week part-time.
- Depends on: M-1.
- Note: the literature survey
  (`docs/surveys/unreachable-after-terminator-python-2026-05.md`)
  did not surface a peer-reviewed Python application that satisfies
  `citations-policy.md` clause (a)/(b)/(c). Per policy, the Python
  extension ships with `LanguageCitationStatus::Unconfirmed` on each
  Python finding; CITATIONS.md records the gap by pointing at the
  survey notes. P1 itself remains satisfied by the two grandfathered
  Rust citations.

M-3. Cross-cutting detectors to Python

- Status: `[x]`
- Goal: extend the three cross-cutting detectors (`clone-drift`,
  `arg-swap`, `comment-code`) to Python via internal `Language`
  dispatch in their existing crates (parameterised, not duplicated
  per language).
- Acceptance: each detector accepts `ParsedFile`s of either
  language, emits findings with the correct `detector_id`, and
  carries a citation set that includes at least one Python-relevant
  reference per detector (M-6 enforcement).
- Effort: 1-2 weeks per detector, 3-6 weeks total part-time.
- Depends on: M-1, M-2.
- Progress:
  - `[x]` comment-code Python (M-3 first detector, lands together
    with F4 phase 4b per `docs/spec/multilang-v0.md` migration
    sequence). Patterns py-raises (doc claims raise but body lacks
    `raise_statement`) and py-deprecated (doc claims deprecated
    but no `@deprecated`-style decorator). Survey
    (`docs/surveys/comment-code-python-2026-05.md`) returned no
    qualifying Python citation; emits
    `LanguageCitationStatus::Unconfirmed` per
    `citations-policy.md`.
  - `[x]` arg-swap Python. Top-level `function_definition` (incl.
    `decorated_definition` and `async def`) plus bare-identifier
    `call` extraction; rejects keyword arguments, splats, and
    non-identifier expressions in v0. Survey
    (`docs/surveys/arg-swap-python-2026-05.md`) accepts Allamanis,
    Jackson-Flux, Brockschmidt (NeurIPS 2021, PyBugLab + PyPIBugs)
    under clauses (a) and (c) of `citations-policy.md`; emits
    `LanguageCitationStatus::Confirmed`.
  - `[x]` clone-drift Python. Top-level `function_definition`
    (including `decorated_definition` wrappers and `async def`)
    extraction with NiCad-style normalization (identifier and
    literal placeholders, comments stripped, n-gram clustering with
    Jaccard >= 0.5, partition-by-exact-form drift signal). New
    `MIN_FN_TOKENS = 22` size guard filters trivially short
    utility functions whose drift signal is too noisy in practice.
    Survey (`docs/surveys/clone-drift-python-2026-05.md`) accepts
    Assi, Hassan, Zou (ACM TOSEM 2025, DOI 10.1145/3721125) — an
    independent peer-reviewed application of NiCad and SourcererCC
    to nine open-source Python deep-learning frameworks — under
    clause (b) of `citations-policy.md`; emits
    `LanguageCitationStatus::Confirmed`.

M-4. Python β corpus

- Status: `[x]`
- Goal: Python-side analogue of P-1. Either reuse `corpus-fetch`
  with a Python source path or stand up a sibling crate
  (`cntrdct-corpus-fetch-python`) backed by PyPI. Ship under
  `benchmarks/wild-corpus-python/` with the same manifest format
  (license, SHA-256, source crate / package).
- Acceptance: `cntrdct eval benchmarks/wild-corpus-python` runs
  cleanly with non-trivial precision/recall numbers on the Python
  detectors.
- Effort: 2-3 weeks part-time.
- Depends on: M-2, M-3.
- Delivered (v0):
  - `scripts/fetch_python_corpus.py` — a stdlib-only PyPI fetcher
    (no third-party deps, no Rust crate). Pins
    `(package, version, file_path)` triples, downloads each sdist,
    verifies the tarball SHA-256 against PyPI's reported digest,
    extracts listed members, prepends a 3-line provenance header,
    writes to `benchmarks/wild-corpus-python/files/`. Idempotent;
    re-runs produce byte-identical output.
  - `cntrdct-eval` `ManifestEntry` extended with optional
    `source` / `license` / `sha256` fields. Existing seed-corpus
    manifest entries continue to parse unchanged (the new fields
    are `#[serde(default)]`).
  - 11-file v0 wild corpus from five packages (six, attrs, click,
    idna, charset-normalizer) with hand-labelled TP / FP triage in
    `benchmarks/wild-corpus-python/manifest.jsonl`.
  - `cntrdct eval benchmarks/wild-corpus-python` reports
    overall precision = 0.05, recall = 1.00, F1 = 0.10
    (per-detector breakdown in `benchmarks/wild-corpus-python/README.md`).
    Non-trivial precision is the M-4 acceptance signal — the
    near-1.0 numbers from the seed corpus would not have been.
  - Limitations and expansion path documented in the corpus README.

M-5. Surface multi-language across tooling

- Status: `[x]`
- Goal: extend the GitHub Action wrapper to accept multiple paths
  with per-path language hints; extend `cntrdct.toml` with an
  optional `[languages]` section to control discovery; verify SARIF
  emitter handles non-Rust files unchanged.
- Acceptance: a sample workflow scans a mixed Rust+Python repo,
  surfaces findings from both languages as inline annotations and as
  SARIF, and respects per-language suppression in `cntrdct.toml`.
- Effort: 2-3 days part-time.
- Depends on: M-2 (Python detector at minimum).
- Delivered:
  - `cntrdct.toml` `[languages.<canonical>]` section landed with
    fields `enabled: bool` (walker discovery control) and
    `suppress: [String]` (per-language detector suppression). Schema
    in `crates/config/src/lib.rs`; integration tests in
    `crates/cli/tests/multilang_config.rs`.
  - GitHub Action wrapper `paths:` input accepts a multi-line list
    where each entry is `<path>` or `<path>:<lang_csv>`. Per-path
    language hints synthesise an ephemeral `cntrdct.toml` via
    `.github/actions/scan/scripts/prepare_config.py`; multi-path
    merging happens through `merge_json.py` (JSON / annotations) and
    `merge_sarif.py` (SARIF `runs[]` concat).
  - SARIF mixed-language path verified by
    `sarif_emitter_handles_mixed_rust_and_python_unchanged` in the
    integration test above.
  - Sample workflow updated in `examples/github-action-usage.yml`.

M-6. Citation policy for multi-language detectors

- Status: `[x]`
- Goal: write `docs/spec/citations-policy.md` codifying P1 for the
  multi-language case (each language a detector supports must carry
  at least one citation grounded in empirical work on that
  language). Extend `crates/cli/tests/citations_consistency.rs` to
  enforce the rule structurally — a detector whose
  `supported_languages()` includes `Language::Python` must declare a
  citation tagged as Python-relevant, and so on.
- Acceptance: the policy doc is approved, the consistency test
  fails on a deliberately under-cited fixture detector, and passes
  on every shipped detector after M-2 / M-3 land their new
  citations.
- Effort: 1 day.
- Depends on: M-1.

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

Phase D (Multi-language; interrupts the original Practical-track
sequence so new detectors are designed multi-language from day one):

15. M-6 citation policy doc (cheap; locks in the P1 extension first)
16. M-1 language abstraction foundation
17. M-2 pilot Python detector (`unreachable-after-terminator-py`)
18. M-3 cross-cutting detectors to Python
19. M-5 multi-language tooling surface
20. M-4 Python β corpus

Phase E (Practical-track items, resumed once Phase D lands):

21. P-4 ranker recalibration (now over Rust + Python corpora)
22. P-2 pr-miner detector (multi-language from inception, supersedes
    the original `pr-miner-rust` framing)
23. P-5 v0.2.0-beta release

Phase F (Tier 3 / 4 organically after launch):

24. T4-19 / T4-20 / T4-17 / T4-18 / T4-21 community scaffolding
25. T3-13 mdBook
26. T3-12 LSP server
27. T3-14 / T3-15 / T3-16 polish

The split between Phase A (Tier 1, blocking) and later phases is the
single most important boundary in this roadmap. Everything in Phase A
should be done before any external announcement; everything after
Phase A can be sequenced based on signal from early users.
