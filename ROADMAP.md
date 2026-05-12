# cntrdct implementation roadmap

Last updated: 2026-05-12 (Q-14 recall-audit Phase B batch 4
landed same day on top of batch 3 + Phase C. Batch 4 introduces
the `paper-appendix` source kind via three PyPIBugs (Allamanis
NeurIPS 2021) ArgSwap entries on permissive-licensed Python
repositories: `c137digital/unv_app@d217fa0d` MIT
(cross-file imported `update_dict_recur` call),
`mwouts/nbrmd@dfa96996` MIT (cross-file imported
`compare_notebooks` call in a pytest test), and
`markokr/rarfile@7fd6b2ca` ISC (`self._set_attrs(dst, inf)`
method call). All three are FN against cntrdct's narrow
Rice-2017 arg-swap detector by `docs/spec/arg-swap-v0.md`
F3 / F4 / F5 — qualified-path / method-call call sites are
out of scope, cross-file definitions are not resolved, and a
hypothetical method-resolving extension would still need a
reverse-permutation name match (which rarfile lacks). Updated
corpus 11 files / 19 expected entries / overall
recall_upper_bound 0.37 (down from 0.44), `arg-swap` 0 TP /
4 FN / 0.00 (up from 0/1/0.00), `comment-code` 4/0/1.00
unchanged, `unreachable-after-terminator` 3/8/0.27 unchanged.
The 0.44→0.37 drop is downward for the right reason: PyPIBugs
labels arg-swap bugs cntrdct's detector cannot catch by design,
so adding paper-appendix entries surfaces the gap rather than
inflating it. pr-miner was the originally chosen batch-4
detector but punted on the structural blocker that the
extractor (top-level `fn` / `def` only per
`src/detectors/pr_miner/extract_{rust,python}.rs`) collides
with modern Rust RAII and Python `with` idioms — paired-API
patterns survive almost exclusively in class methods that the
extractor drops, and the PR-Miner paper's published bugs
(Linux / PostgreSQL / Apache HTTPd) are all C and therefore
outside pr-miner's supported languages. Earlier 2026-05-12:
Q-14 recall-audit Phase B batch 3 broadens the corpus to a
third detector, `comment-code`, via the textbook Pattern C bug
(Tan SOSP 2007 §3.2 "bad comment": `/// Deprecated` prose
without the runtime `#[deprecated]` attribute). Seeded from
Apache-2.0 `sidan-lab/whisky-archive`
`packages/whisky-common/src/data/primitives/constructors.rs`
pinned at commit `99243766` — the `con_str` / `con_str0` /
`con_str1` / `con_str2` family contributes four expected TP
entries, so comment-code enters the corpus with single-source
recall_upper_bound 1.0. Clone-drift was investigated as the
original batch-3 target but punted to a later batch: the
published peer-reviewed clone-drift bug catalogues (Bettenburg
MSR 2009, Krinke ICSM 2007) target C/Java rather than
Rust/Python, and the Assi TOSEM 2025 deep-learning-framework
genealogies (mia1q/code-clone-DL-frameworks replication CSVs)
expose size-2 clone pairs that fall under cntrdct's
`MIN_GROUP_SIZE = 3` floor by construction. Further Phase B
batches still owe coverage for clone-drift, config-interaction,
and pr-miner, plus the semgrep / codeql / clippy source kinds. 2026-05-12 earlier: Q-14 recall-audit
Phase B batch 2 and Phase C release-tag refresh discipline both
landed. Phase B batch 2: six additional expected entries on
`unreachable-after-terminator` deepening the recall ceiling on
the existing detector via three rustc UI testset files
(`expr_return.rs`, `expr_call.rs`, `expr_loop.rs`) that probe
control-flow shapes cntrdct's statement-level scan misses by
construction; corpus before batch 3 was 7 files / 12 expected
entries / overall recall_upper_bound 0.25. The drop from 0.50
to 0.25 is intentional honest signal — closing the gaps is
detector-improvement work, not audit-harness work. Phase C:
`benchmarks/audit-corpus/README.md` adds a "Refresh discipline
(Phase C)" section enumerating the on-tag procedure;
`CLAUDE.md` "Release procedure" steps now include the audit
re-run between lockfile sync and commit, and the non-negotiables
pin the same-commit rule and a no-op refresh policy (figures
unchanged = no-op is fine; the discipline is the re-run, not the
delta). No CI enforcement on top — audit-recall is already gated
indirectly via the embedded priors and detector logic CI already
checks.
2026-05-11: Q-14 recall-audit Phase B first batch landed — six
expected entries / two detectors / two sources / overall
recall_upper_bound 0.50; Q-16 cargo-mutants nightly landed same day
— `.cargo/mutants.toml` scopes mutation testing to
`src/detectors/**/*.rs` via `examine_globs`,
`.github/workflows/mutants.yml` runs `cargo mutants --no-shuffle -j 2`
on a 06:00 UTC cron + `workflow_dispatch`, the post-run step tallies
`mutants.out/{caught,missed,unviable,timeout}.txt` and fails the job
when `caught / (caught + missed) < 0.80`, the missed-mutant list lands
in `$GITHUB_STEP_SUMMARY` and `mutants.out/` is archived as a 30-day
artifact for off-runner inspection; first nightly run on master is the
real signal for whether the codebase already meets the 80% gate since
local validation is multi-hour. Q-14 recall-audit harness Phase A
scaffolding landed same day — `cntrdct calibrate --audit-recall
<CORPUS_DIR>` flag wired with clap conflict against `--fit-platt`,
new `src/recall_audit.rs` module with `audit_recall(...) ->
RecallAuditReport` pure function and `external_source: {kind, ref,
url}` provenance per labelled finding, `cntrdct::run_recall_audit`
orchestrator, 12 tests (4 unit + 8 integration) pin loader / matcher
/ byte-stable JSON / CLI conflict, `benchmarks/audit-corpus/`
skeleton with per-detector seed targets documented, CITATIONS.md
adds `heckman-williams-ist-2011` under Layer 2; Q-14 Phase B first
batch landed same day — six expected entries across two detectors
and two sources (`rustc-lint-testset` ×5 from rust-lang/rust@4b0c9d76
`tests/ui/reachable/{unreachable-code-ret,expr_block,expr_if}.rs`,
`github-commit` ×1 from `wasserth/TotalSegmentator` PR #556
`statistics.py:58`), audit numbers
`unreachable-after-terminator` recall_upper_bound 0.60 (3 TP / 2 FN),
`arg-swap` 0.00 (0 TP / 1 FN), overall 0.50; the rustc-testset
copies strip the file-level `#![deny(unreachable_code)]` attribute
because cntrdct's SUPPRESSION_TOKEN scan would otherwise honour it.
README "Latest audit run" populated, ROADMAP Phase B entry
expanded, further batches broadening detector and source coverage
plus Phase C release-tag refresh discipline still pending. Spec:
`docs/spec/recall-audit-v0.md`. Q-13 cross-model κ
audit redesigned to
CLI-only same day — `PromptDispatch` trait + `ClaudeCliAdjudicator` +
`GeminiCliAdjudicator` ship alongside the existing
`AnthropicAdjudicator`; OpenAI / Google API-key paths and the
nightly workflow were dropped because (a) Codex CLI's system prompt
could not be cleanly replaced, (b) continuous monitoring was
unsupported by measurement stationarity given silent model drift +
sampler stochasticity. Audit is now on-demand: `cntrdct
cross-model-kappa <CORPUS>` shells out to `claude --print` and
`gemini -p` (auth via each CLI's OAuth, no API keys read by
cntrdct), prints to stdout by default. `src/cross_model_kappa.rs`
carries the pure κ math + per-`(detector_id, anomaly_class)`
aggregation; PR CI exercises `tests/cross_model_kappa.rs` with
`CannedDispatch` mocks plus stub-script tests for the CLI flag
sets; citations `wataoka-2024` and `zheng-neurips-2023` added to
Layer 3;
Q-11 small-N Wilson → Jeffreys interval
switching landed 2026-05-10; Q-12 LLM-calibration post-hoc Platt fit
landed 2026-05-10 — adjudicator prompt drops the verbalised
`calibration_tag`, post-hoc Platt scaling fit per `(detector_id,
anomaly_class)` cell takes its place, embedded
`benchmarks/llm-calibration/platt-default.json` ships empty in v0,
`AdjudicationResult.calibrated_confidence` + SARIF `properties.
adjudication.calibrated_confidence` plumbed end-to-end,
`tests/calibration_ece.rs` pins ≥ 0.05 holdout-ECE drop on a
constructed over-confidence fixture; Phase G RC1-blocker Q-series
Q-1..Q-5 landed; Phase H governance items Q-6..Q-10 all landed
2026-05-07, closing Phase H; P-7 clone-drift residual FP cleanup
landed 2026-05-07, taking wild β clone-drift FPs to 0 in both Rust
and Python; T3-15 git-cliff release-notes pipeline and T3-16
telemetry-free assurance both landed 2026-05-08; T3-14
cargo-binstall metadata and Homebrew tap
(`ktrysmt/homebrew-cntrdct`) landed 2026-05-08, with the AUR
sub-target dropped from scope; T3-12 LSP server Phase 1 scaffolding
(`cntrdct-lsp` binary behind the `lsp` Cargo feature) landed
2026-05-08, with Phase 1.b document events + Finding -> Diagnostic
mapping (`textDocument/{didOpen,didChange,didSave,didClose}` →
`publishDiagnostics`, sharing the Layer 1 detector battery with the
disk-walking scan path via a new `scan_buffer` + `run_detectors_on`
seam) landing the same day on top, Phase 1.c per-URI didChange
debouncing (250 ms quiet window backed by an
`Arc<tokio::sync::Mutex<HashMap<Url, JoinHandle>>>`, with
`didSave` / `didClose` draining the pending map for their URI before
acting) landing 2026-05-09, and Phase 1.c+ per-URI monotonic
generation counter (extends the per-URI map to a `UriState` carrying
`{handle, latest_generation}`, every event bumps and captures, every
publish is gated on `captured == latest`) closing the
`spawn_blocking`-cannot-be-aborted race the same day; v0.2.0-rc.1
tag cut 2026-05-08 — first end-to-end run of the git-cliff + Homebrew
bump pipelines, both green on first try; T4-17, T4-18, T4-19 landed
earlier; T4-20 / T4-21 deferred per maintainer decision; community
scaffolding minus the formal CoC, which is deferred until external
contributor activity warrants it)

Engineering roadmap for shipping cntrdct as a usable open-source Rust
tool.

## Status legend

- `[x]` completed
- `[~]` in progress
- `[ ]` pending

## Practical track

Items that are not strictly OSS-readiness work but are critical
engineering deliverables that shape the public-facing v1. Done in
parallel with Tier 1 OSS readiness.

P-1. β corpus collection (real-world Rust crates)

- Status: `[x]`
- Summary: `scripts/fetch_rust_corpus.py` (stdlib only) pins
  `(crate, version, file_path)` triples, pulls `.crate` tarballs
  from `static.crates.io`, verifies SHA-256 against the sparse-index
  `cksum`, and rejects `@generated` sources. Result:
  `benchmarks/wild-corpus/` with 36 crates / 270 files / ~13 MB
  under permissive licenses; 124 hand-labelled findings (all v0
  FPs). Per-detector precision = 0 here is the intentional
  non-trivial signal feeding P-4. Limitations enumerated in
  `benchmarks/wild-corpus/README.md`.

P-2. pr-miner detector (multi-language)

- Status: `[x]`
- Summary: sixth Layer 1 detector mining implicit programming
  rules via Apriori (`MAX_ITEMSET_SIZE = 2`) over per-function
  call-site transactions. Spec at `docs/spec/pr-miner-v0.md`;
  module ships under `src/detectors/pr_miner/` (`apriori.rs`,
  `extract_rust.rs`, `extract_python.rs`, `mod.rs`). Citation:
  `li-zhou-fse-2005` (Confirmed for Rust, grandfather clause);
  Python is `LanguageCitationStatus::Unconfirmed` per
  `docs/surveys/pr-miner-python-2026-05.md`. Eight positives + three
  negatives per language;
  `tests/corpus_shape.rs::pr_miner_corpus_meets_per_language_positives`
  enforces the per-language commitment.
- Followup: Q-1 wires pr-miner into the SARIF detectors array;
  Future Q-series candidates note the Apriori → FP-growth lift.

P-3. SARIF output validation in CI

- Status: `[x]`
- Summary: `.github/workflows/ci.yml:86-106` runs
  `Sarif.Multitool validate` against the OASIS 2.1.0 schema on
  every CI run.

P-4. Layer 2 ranker recalibration on the β corpus

- Status: `[x]`
- Summary: `scripts/build_priors_corpus.py` derives a labelled
  JSONL from `(benchmarks/corpus, benchmarks/wild-corpus-python)`
  (87 rows: 69 TP / 18 FP) at `benchmarks/labelled-findings.jsonl`.
  `cntrdct calibrate` writes `benchmarks/priors-default.json`,
  embedded into the binary via `include_str!`. Fallback chain:
  explicit `--priors` → per-user cache → embedded → uncalibrated.
  Reorder sensitivity covered by
  `calibrated_ranker_reorders_when_wilson_disagrees_with_related_count`.

P-5. β release tagging and crates.io publish

- Status: `[x]` 2026-05-06
- Summary: `v0.2.0-beta.1` shipped — GitHub Release
  <https://github.com/ktrysmt/cntrdct/releases/tag/v0.2.0-beta.1>
  (linux x86_64/aarch64, darwin aarch64, windows x86_64 with
  matching `.sha256`); crates.io
  <https://crates.io/crates/cntrdct/0.2.0-beta.1>. Install:
  `cargo install cntrdct --version 0.2.0-beta.1 --locked`
  (explicit version qualifier required for pre-releases per SemVer).
  Workspace consolidated from 15 crates into one package; P3 LLM
  gating preserved by module boundary (only `src/adjudicator.rs`
  references reqwest).
- Successor: `v0.2.0-rc.1` cut 2026-05-08 — GitHub Release
  <https://github.com/ktrysmt/cntrdct/releases/tag/v0.2.0-rc.1>,
  crates.io <https://crates.io/crates/cntrdct/0.2.0-rc.1>. Install:
  `cargo install cntrdct --version 0.2.0-rc.1 --locked` (still pre-
  release per SemVer). Bundles the Phase G/H Q-series, P-7
  clone-drift residual cleanup, T3-12 LSP scaffolding, T3-14
  Homebrew + cargo-binstall, T3-15 git-cliff release-notes pipeline,
  and T3-16 netns telemetry-free assurance. First end-to-end run of
  the git-cliff release-body pipeline and the Homebrew tap auto-
  bump workflow; both green on first execution.
- Followup landed 2026-05-10: `cntrdct --version` / `-V` now wired.
  `Cli` derive at `src/main.rs:13` carries `#[command(version)]`,
  which clap fills from `CARGO_PKG_VERSION`. Both flags emit
  `cntrdct <version>` and exit 0 before subcommand resolution, so
  the prior `error: unexpected argument` clap message is gone. The
  `cargo cntrdct --version` shim path inherits the behaviour for
  free since `src/cargo_subcommand.rs` is a verbatim arg forwarder.
  Two new integration tests
  (`cli_version_long_flag_prints_cargo_pkg_version`,
  `cli_version_short_flag_prints_cargo_pkg_version` in
  `tests/integration.rs`) pin the contract structurally.

P-6. v0 → v0.1 detector quality fixes (wild β FP reduction pass)

- Status: `[x]` 2026-05-07
- Summary: wild β FPs cut from Rust 124 → 24 (80.6 % reduction)
  and Python 19 → 4 (78.9 %) via structural fixes — not corpus-
  specific tweaks. `unreachable-after-terminator` F4b/F4c
  (cfg-gated terminator suppression + hoisted item filtering),
  `comment-code` F5b/F5c (Python factory-shape + parameter-level
  `.. deprecated::` peeking), `clone-drift` F5b/F5c (scope-bounded
  clustering + strict-majority + Jaccard ≥ 0.7 gate). Embedded
  priors recomputed (clone-drift Wilson 0.073→0.355,
  unreachable-after-terminator 0.407→0.796, comment-code
  0.298→0.657). `cntrdct calibrate` made byte-stable via sorted
  `BTreeMap`. New prereg `prereg/2026-05-07-osf-prereg.md`
  supersedes the 2026-05-06 file.

P-7. clone-drift within-scope residual cleanup

- Status: `[x]` 2026-05-07
- Summary: F5d sibling-family discriminator (3 sub-gates) closes
  all 5 P-6 residuals. F5d-i suppresses clusters carrying ≥ 2
  size-1 partitions (the Python `charset_normalizer.utils`
  `is_<script>` family at `:70` and `:194`); F5d-ii suppresses
  high-Jaccard / high-length-imbalance singletons when the
  dominant partition holds only 2 functions (uuid `encode_*` at
  `uuid__fmt.rs:280`, tracing-subscriber `*_is_none` twins at
  `tracing_subscriber__layer_mod.rs:1547`); F5d-iii suppresses
  3-fn clusters whose dominant exemplar normalises to within 2
  tokens of `MIN_FN_TOKENS` (syn parse-API family at
  `syn__lib.rs:961`). The dominant-floor conditioner on F5d-ii
  (`LENGTH_IMBALANCE_DOMINANT_FLOOR = 3`) is what keeps the seed-
  corpus `clone_drift_005` TP at length imbalance 0.258 from
  being suppressed alongside the wild β residuals at 0.186 and
  0.242 — empirically the FP / TP bands overlap on length
  imbalance alone and are distinguished only by dominant size.
  Wild β clone-drift FP count → 0 in both Rust and Python.
  `tests/detector_clone_drift.rs` t29 / t30 / t30b / t31 pin the
  new gates structurally; t1–t28 all pass. Spec:
  `docs/spec/clone-drift-v0.md` F5d. Embedded priors recompute:
  clone-drift Wilson lower 0.355 → 0.676 (8 TP / 0 FP).

## Tier 1 — usable OSS (blocking for first announcement)

T1-1. GitHub Actions CI

- Status: `[x]`
- Summary: `.github/workflows/ci.yml` runs `cargo test --workspace`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo fmt --all -- --check` on a Linux + macOS matrix.

T1-2. crates.io metadata for every crate

- Status: `[x]`
- Summary: root `Cargo.toml` carries `description`, `repository`,
  `keywords`, `categories`, `readme`, `license`;
  `cargo publish --dry-run` is green.

T1-3. README polish for OSS audience

- Status: `[x]`
- Summary: `README.md` opens with a one-paragraph pitch, badges
  (CI / crates.io / docs.rs / license), copy-pasteable quickstart,
  and explicit P1-P5 one-liners.

T1-4. examples directory

- Status: `[x]`
- Summary: ≥ 3 self-contained examples under `examples/` (scan,
  calibrate, adjudicate-with-mock-API); each is invoked by a CI
  smoke-test step.

T1-5. rustdoc on cntrdct-core public surface

- Status: `[x]`
- Summary: every public item carries a doc comment;
  `#![deny(missing_docs)]` is enforced and
  `cargo doc -p cntrdct --no-deps` is clean.

T1-6. LICENSE coverage review

- Status: `[x]`
- Summary: workspace is MIT-only; `cargo deny check licenses`
  passes. NOTICE / `LICENSES/` to be added if non-MIT deps land.

T1-7. GitHub Pages essay site

- Status: `[x]`
- Summary: Jekyll site under `docs/site/` (`_config.yml`, `index.md`,
  `essays/`) auto-deployed via `.github/workflows/pages.yml` on
  pushes touching the site directory. Canonical URL
  <https://ktrysmt.github.io/cntrdct/> is linked from `README.md`;
  the first essay ("The Linter that Cites Its Sources") is live at
  `essays/citation-as-api/`.

## Tier 2 — adoption-grade (drives external usage)

T2-7. Suppression mechanism

- Status: `[x]`
- Summary: in-source `#[cntrdct::allow(<detector_id>)]` plus
  project-wide `cntrdct.toml` for severity remapping, threshold
  overrides, and per-path allow/deny rules. Integration tests
  cover all three suppression paths.
- Followup landed in Q-9 (2026-05-07): the Python whole-file skip
  was replaced with a tree-sitter-python suppression scanner that
  recognises `# cntrdct: allow(<id>, ...)` line comments.

T2-8. Pre-built release binaries

- Status: `[x]`
- Summary: tag-driven release workflow uploads binaries for
  Linux x86_64 / aarch64, macOS aarch64, Windows x86_64;
  `curl | sh` install path works end-to-end.

T2-9. GitHub Action wrapper

- Status: `[x]`
- Summary: action consumes the pre-built binary and surfaces
  findings as PR comments matching GitHub Annotations
  conventions; sample workflow demonstrates inline findings.

T2-10. Parallel detection via rayon

- Status: `[x]`
- Summary: per-file detector runs execute in parallel;
  `tests/parallel_scan.rs:50-72` asserts byte-identical
  `Vec<Finding>` between serial and parallel runs.

T2-11. cargo cntrdct subcommand

- Status: `[x]`
- Summary: `cargo install cntrdct` ships a `cargo-cntrdct` shim so
  `cargo cntrdct scan` works alongside `cntrdct scan`.

## Tier 3 — polish (post-launch)

T3-12. LSP server

- Status: `[~]` (Phase 1 scaffolding landed 2026-05-08; Phase 1.b
  document events + Finding -> Diagnostic mapping landed 2026-05-08;
  Phase 1.c didChange debouncing landed 2026-05-09; Phase 1.c+
  per-URI generation counter landed 2026-05-09; Phases 2 / 3
  still pending)
- Goal: a `cntrdct-lsp` crate that exposes findings to IDEs
  (VS Code, Helix, Neovim) via the Language Server Protocol.
- Acceptance: a `vscode-cntrdct` extension or comparable
  client surfaces findings inline.
- Effort: 4-6 weeks (across all phases).
- Phase 1 — server scaffolding (done 2026-05-08): `cntrdct-lsp`
  binary added under the optional `lsp` Cargo feature
  (`tower-lsp` 0.20 + tokio multi-thread runtime). Implements the
  LSP lifecycle methods (`initialize` returning
  `text_document_sync = Full`, `initialized` logging via
  `window/logMessage`, `shutdown`). Spec: `docs/spec/lsp-v0.md`.
  CI gains a `clippy (lsp feature)` step so a future cntrdct API
  change that breaks `src/lsp.rs` fails CI rather than rotting
  silently. Default `cargo install cntrdct` is unchanged; LSP build
  is opt-in via `cargo install cntrdct --features lsp`.
- Phase 1.b — document events + Finding -> Diagnostic mapping
  (done 2026-05-08): `textDocument/{didOpen,didChange,didSave,didClose}`
  wired through to `textDocument/publishDiagnostics`. Buffer scan
  goes through a new `crate::scan_buffer` entry point that shares
  the Layer 1 detector battery with `scan_full_with_config` via
  the extracted `run_detectors_on` helper (so registration ordering
  lives in exactly one place). Severity, code, source, message,
  range (1-based → 0-based), `relatedInformation` (one entry per
  citation key, with the citation URL resolved through a static
  detector-citation registry when available, falling back to the
  buffer URI when the key is unknown), and `data` (verbatim
  `evidence.raw`) all follow the lsp-v0.md mapping table. Scans
  run on `tokio::task::spawn_blocking` so the event loop is not
  blocked while a multi-thousand-LOC buffer parses. CI exercises
  the new surface through `cargo test --features lsp --test
  lsp_smoke` (a subprocess JSON-RPC round-trip) and
  `cargo test --features lsp --lib lsp::tests` (seven unit tests
  pinning the Finding -> Diagnostic mapping); the `clippy (lsp
  feature)` step now passes `--all-targets` so the new test files
  are checked too.
- Phase 1.c — debouncing on didChange (done 2026-05-09): per-URI
  250 ms quiet window in `src/lsp.rs`. `did_change` now spawns a
  debounced task (`tokio::spawn` + `tokio::time::sleep`) instead of
  scanning inline; a successor `did_change` for the same URI calls
  `JoinHandle::abort()` on the prior handle and replaces it.
  `did_save` and `did_close` drain the per-URI pending map before
  acting so an explicit user action is not shadowed by a stale
  follow-up publish. `Cargo.toml` adds `time` + `sync` to the
  optional `tokio` features. New smoke test
  `did_change_debounces_rapid_bursts_to_one_publish` in
  `tests/lsp_smoke.rs` fires three notifications inside the window
  and asserts exactly one `publishDiagnostics` survives, carrying
  the most recent buffer state.
- Phase 1.c+ — per-URI generation counter (done 2026-05-09): the
  per-URI map evolves from `HashMap<Url, JoinHandle>` to
  `HashMap<Url, UriState>` where `UriState { handle: Option<JoinHandle>,
  latest_generation: u64 }`. Every event that produces a new scan
  (`did_open` / `did_change` / `did_save`) or invalidates pending
  work (`did_close`) bumps `latest_generation` atomically with the
  abort + spawn it performs; each scheduled scan captures the value
  at scheduling time and re-checks it after `spawn_blocking` returns,
  dropping its `publish_diagnostics` if a fresher event has overtaken
  it. This closes the documented race in which `JoinHandle::abort()`
  cannot interrupt a blocking-pool thread that is already inside the
  detector pass. Four new unit tests (`bump_generation_*`,
  `is_current_*`) in `src/lsp.rs::tests` pin the counter primitives;
  the existing `did_change_debounces_rapid_bursts_to_one_publish`
  smoke test continues to pass unchanged. The error-log path
  (`window/logMessage`) is intentionally left ungated — a stale scan
  that errored out describes a real failure the user wants to see.
- Phase 2 — `vscode-cntrdct` extension scaffolding (TypeScript /
  pnpm), bundling the LSP binary auto-downloaded from GitHub
  Releases. Separate repository under `ktrysmt/vscode-cntrdct`.
- Phase 3 — VS Code Marketplace listing + announcement.

T3-13. mdBook user guide

- Status: `[ ]`
- Goal: a `book/` directory hosting user-facing documentation
  (concepts, detector reference, configuration, FAQ) built with
  mdBook and published to GitHub Pages.
- Acceptance: `book/book.toml` builds without errors, all
  pages have at least placeholder content, and the published
  URL is linked from the README.
- Effort: 2-3 weeks.
- Note: the existing Jekyll essays under `docs/site/essays/` are
  scheduled to migrate to a separate external blog rather than be
  absorbed into mdBook. Plan: keep `docs/site/` in place during the
  user-guide build-out; once the external blog has the content,
  retire the Jekyll site (and its `pages.yml` deploy step) so the
  GitHub Pages URL serves the mdBook user guide alone.

T3-14. Distribution channels beyond crates.io

- Status: `[~]` (cargo-binstall and Homebrew tap landed 2026-05-08;
  AUR package deferred per maintainer decision)
- Goal: Homebrew tap, `cargo-binstall` metadata. AUR is no longer in
  scope.
- Effort: 1-2 days each.
- Depends on: T2-8.
- Summary: cargo-binstall metadata block in root `Cargo.toml` maps
  the existing release-archive layout
  (`cntrdct-v{version}-{target}/{cntrdct,cargo-cntrdct}{,.exe}`,
  tar.gz on Linux/macOS and zip on Windows); users can now run
  `cargo binstall cntrdct` to fetch the pre-built archive instead of
  compiling. The Homebrew tap lives at
  `ktrysmt/homebrew-cntrdct`, with `Formula/cntrdct.rb` covering
  macOS aarch64 and Linux x86_64/aarch64. The bump path is
  `.github/workflows/homebrew.yml` in this repo: it triggers on
  every `v*` tag push, polls for the release artifacts to be
  uploaded by `release.yml`, then rewrites and pushes
  `Formula/cntrdct.rb` in the tap repo. The `v0.2.0-rc.1` tag
  (2026-05-08) was the first end-to-end run; the workflow bumped
  the Formula from the seeded `0.2.0-beta.1` to `0.2.0-rc.1` with
  refreshed SHA256s on first try. README Quickstart documents both
  `brew tap ktrysmt/cntrdct` and `cargo binstall cntrdct`.
- Operational note: the bump workflow consumes the
  `HOMEBREW_TAP_TOKEN` repo secret on `ktrysmt/cntrdct` (a
  fine-grained PAT scoped to `ktrysmt/homebrew-cntrdct` with
  Contents: Read and write). The secret is registered as of
  2026-05-08; if it is ever rotated, the workflow exits with a
  pointer back to this entry on the next tag push.
- AUR (out of scope): originally listed as a distribution target,
  now dropped — the operational cost of maintaining an AUR account
  + submission flow is not justified by current Arch demand. If
  external demand materialises, reopen as a new T-series item.

T3-15. Auto-generated changelog

- Status: `[x]` 2026-05-08
- Summary: `cliff.toml` at repo root configures `git-cliff` against
  cntrdct's Conventional Commits prefixes (`feat` / `fix` / `perf` /
  `refactor` / `promote` / `docs` / `test` / `ci` / `chore`;
  `chore(release)` / `chore(changelog)` / `Merge` are skipped). The
  release workflow's `release` job now checks out with
  `fetch-depth: 0` and runs `orhun/git-cliff-action@v4` with
  `--latest --strip header`, then feeds the output into
  `softprops/action-gh-release` via `body_path: RELEASE_NOTES.md`,
  replacing the prior `generate_release_notes: true` path. CI side
  is fully self-contained: no local `git-cliff` install is required.
  `CONTRIBUTING.md` "Pull request review" updated so the squash-on-
  merge guidance points at the new pipeline; CLAUDE.md "Release
  procedure" non-negotiables documents the parser's drop list.
- Followup: a checked-in `CHANGELOG.md` and an auto-commit-back step
  on tag push were deferred until a future tag confirmed the
  release-body path is healthy in production. `v0.2.0-rc.1`
  (2026-05-08) is that confirmation — git-cliff produced the
  expected grouped output (Bug Fixes / CI / Chores / Documentation /
  Features) on first run with commit-link backrefs and a
  `compare/v0.2.0-beta.1..v0.2.0-rc.1` URL. The followup is now
  unblocked for whoever picks it up next; it is no longer a
  prerequisite for any other roadmap item, just an OSS-hygiene
  improvement.

T3-16. Telemetry-free assurance

- Status: `[x]` 2026-05-08
- Summary: a new `network-isolation` job in
  `.github/workflows/ci.yml` runs `cntrdct scan` inside a fresh
  Linux network namespace (`sudo unshare --net`) on every push and
  pull request. The namespace ships with no outbound routes; any
  unexpected network call from the scan path fails `ENETUNREACH` /
  `EAI_*` and the job goes red. The job exercises walker →
  parsers → Layer 1 detectors → Layer 2 ranker → Layer 4 SARIF
  emitter — i.e. everything that runs by default for end-users —
  and asserts a non-empty, well-formed SARIF document on stdout.
  The reqwest dependency stays constrained to `src/adjudicator.rs`
  (gated by the explicit `--adjudicate` flag) and `src/lib.rs`'s
  `wire_adjudicator` constructor. README.md carries a new
  "Network access" section documenting both the design property
  and the CI enforcement; the assurance has no opt-out path.
- Implementation note: the first attempt used the unprivileged
  `unshare -r --net` form, but Ubuntu 24.04's AppArmor
  `unprivileged_userns` profile blocks `/proc/self/uid_map` writes
  from non-root processes on GitHub-hosted runners
  (`unshare: write failed /proc/self/uid_map: Operation not
  permitted`). The fix was to drop the user-ns mapping entirely
  and run `sudo unshare --net` instead — passwordless sudo is
  available on GHA runners, and `--no-calibration` keeps the
  process from needing `$HOME` access since the priors are
  embedded into the binary via `include_str!`. Carried as a future
  signal: if GHA's runner image ever loosens the AppArmor profile,
  the unprivileged form is preferable for the smaller blast
  radius.

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
- Summary: `cntrdct::parsers` owns the `Language` enum and
  extension mapping. `ParsedFile.language` migrated from
  `String` to enum. The walker discovers all supported
  languages, not just `.rs`.

M-2. Pilot Python detector

- Status: `[x]`
- Summary: `unreachable-after-terminator` extended to Python
  (`raise`, `sys.exit()`, `os._exit()`, `assert False`,
  trailing-`return`). ≥ 5 positive + ≥ 3 negative Python fixtures
  shipped.
- Citation: `LanguageCitationStatus::Unconfirmed` per
  `docs/surveys/unreachable-after-terminator-python-2026-05.md`;
  P1 remains satisfied by two grandfathered Rust citations.

M-3. Cross-cutting detectors to Python

- Status: `[x]`
- Summary: `clone-drift` / `arg-swap` / `comment-code` extended
  to Python via internal `Language` dispatch (parameterised, not
  duplicated). Citation status:
  - `comment-code`: Unconfirmed
    (`docs/surveys/comment-code-python-2026-05.md`).
  - `arg-swap`: Confirmed via Allamanis, Jackson-Flux, Brockschmidt
    NeurIPS 2021 (PyBugLab / PyPIBugs).
  - `clone-drift`: Confirmed via Assi, Hassan, Zou TOSEM 2025
    (NiCad / SourcererCC on nine Python DL frameworks),
    DOI 10.1145/3721125; `MIN_FN_TOKENS = 22` size guard added.

M-4. Python β corpus

- Status: `[x]`
- Summary: `scripts/fetch_python_corpus.py` (stdlib-only) pins
  `(package, version, file_path)` triples from PyPI with SHA-256
  verification. v0 corpus = 11 files / 5 packages (six, attrs,
  click, idna, charset-normalizer) under
  `benchmarks/wild-corpus-python/`. `cntrdct eval` reports
  precision = 0.05, recall = 1.00, F1 = 0.10 — non-trivial
  precision is the M-4 acceptance signal. `ManifestEntry` extended
  with optional `source` / `license` / `sha256`
  (`#[serde(default)]`).

M-5. Surface multi-language across tooling

- Status: `[x]`
- Summary: `cntrdct.toml` `[languages.<canonical>]` section
  (`enabled`, `suppress`); GitHub Action `paths:` accepts
  `<path>:<lang_csv>` per-line entries via
  `prepare_config.py` / `merge_json.py` / `merge_sarif.py`. Mixed
  Rust+Python SARIF path verified by
  `sarif_emitter_handles_mixed_rust_and_python_unchanged`
  (`tests/multilang_config.rs`). Sample workflow:
  `examples/github-action-usage.yml`.

M-6. Citation policy for multi-language detectors

- Status: `[x]`
- Summary: `docs/spec/citations-policy.md` codifies P1 for the
  multi-language case (each supported language must carry at least
  one citation grounded in empirical work on that language).
  `tests/citations_consistency.rs` enforces the rule structurally;
  fails on a deliberately under-cited fixture detector.

## Tier 4 — community (opens contribution funnel)

T4-17. Issue templates

- Status: `[x]`
- Summary: `.github/ISSUE_TEMPLATE/{bug_report,feature_request,detector_proposal}.md`.
  `detector_proposal.md` requires citation key, citations-policy
  clause, IEEE 1044-2009 anomaly class, and the ≥ 8 positives-per-
  language commitment upfront.

T4-18. PR template

- Status: `[x]`
- Summary: `.github/PULL_REQUEST_TEMPLATE.md` covers Conventional
  Commit prefix, DCO sign-off, detector / corpus checklist, and
  gate boxes (`cargo test` / clippy / fmt).

T4-19. CONTRIBUTING.md

- Status: `[x]`
- Summary: `CONTRIBUTING.md` documents the two workspaces, the
  `promote(<area>)` rule, the detector authoring flow
  (proposal → spec → CITATIONS.md → implementation → corpus),
  Conventional Commits, DCO via `git commit -s` (no CLA), and PR
  review expectations. Carries an interim conduct paragraph until
  T4-20 lands.

T4-20. Code of Conduct

- Status: `[ ]`
- Goal: `CODE_OF_CONDUCT.md` based on Contributor Covenant 2.1.
- Effort: 15 minutes.
- Note: deferred until external contributor activity or GitHub
  Discussions warrants the operational overhead (running an
  enforcement contact and triage path). At adoption time the file
  will be a short pointer to the canonical Contributor Covenant URL
  rather than an inline copy. `CONTRIBUTING.md` carries an interim
  conduct paragraph until then.

T4-21. Roadmap discussion pinned

- Status: `[ ]`
- Goal: a GitHub Discussion thread that surfaces this roadmap
  and invites community input on prioritisation.
- Effort: 15 minutes.

## Quality-audit track (Q-series)

Beta-stage wiring fixes, governance hardenings, and methodology
lifts identified during the post-beta.1 quality audit. Q-1 through
Q-5 are RC1 blockers (release-tag prerequisites for v0.2.0-beta.2 /
v0.2.0-rc.1); Q-6 through Q-10 are RC1 governance / hygiene must-
haves; Q-11 through Q-16 target RC2 / v0.2.0 stable.

Q-1. SARIF detectors array missing pr-miner

- Status: `[x]` 2026-05-07
- Summary: `PrMinerDetector` re-added to the SARIF detectors vec
  in `src/main.rs` so `runs[0].tool.driver.rules[]` carries a
  `pr-miner` entry alongside the five Layer 1 peers. The
  `sarif_emitter_handles_mixed_rust_and_python_unchanged` test in
  `tests/multilang_config.rs` now asserts the full
  `cntrdct::ALL_DETECTOR_IDS` set is present in
  `tool.driver.rules` so the regression is caught at CI rather than
  after release.

Q-2. SARIF informationUri placeholder

- Status: `[x]` 2026-05-07
- Summary: `INFORMATION_URI` at `src/sarif.rs:15` is now
  `https://github.com/ktrysmt/cntrdct`, matching `Cargo.toml`'s
  `repository`. `docs/spec/sarif-v0.md` F3 updated to reflect the
  canonical URL. A `grep -RE 'TBD' src/` gate added to the
  `rustfmt` job in `.github/workflows/ci.yml` fails CI on a
  deliberately reintroduced placeholder.

Q-3. clone-drift doc-comment / value drift

- Status: `[x]` 2026-05-07
- Summary: doc comment on `NEAR_DUPLICATE_THRESHOLD`
  (`src/detectors/clone_drift.rs:38-50`) rewritten to describe
  the 0.7 threshold and its effective drift band, and to point at
  `docs/spec/clone-drift-v0.md` F5c-ii (which already documents
  the same value). The previous "0.85" mention left over from a
  draft before P-6's strict-majority + Jaccard gate landed is
  gone.

Q-4. Wiring consistency test

- Status: `[x]` 2026-05-07
- Summary: `cntrdct::ALL_DETECTOR_IDS` introduced as the single
  source of truth for the Layer 1 detector set.
  `tests/wiring_consistency.rs` asserts that (a) detector
  constructions matching `src/lib.rs::scan_full_with_config` and
  (b) the SARIF rules taxonomy emitted by the `cntrdct` binary
  (`src/main.rs`) both equal that constant.
  `tests/prereg_consistency.rs::registered_detectors` now
  includes `PrMinerDetector` and a new
  `registered_detectors_match_canonical_id_set` test pins it
  against `ALL_DETECTOR_IDS`. Removing any detector from any one
  of the three sites fails the suite.

Q-5. SARIF Severity::Info mapping rationale

- Status: `[x]` 2026-05-07
- Summary: `docs/spec/sarif-v0.md` F5 carries a decision-log
  entry that retains `Severity::Info → SARIF "none"`. Rationale:
  no shipped detector emits `Info` by construction (the variant
  enters only via user-authored `cntrdct.toml` severity
  overrides), so a user explicitly downgrading a finding to
  `Info` is signalling "less visible than `Note`" — which is
  exactly the GitHub Code Scanning behaviour for `none`-level
  findings. The original `raw_severity` is recoverable from
  `result.properties.raw` for SARIF consumers that need the full
  four-valued vocabulary.

Q-6. Citation retraction monitor

- Status: `[x]` 2026-05-07
- Summary: `scripts/check_retractions.py` extracts every DOI from
  `CITATIONS.md` and the `doi: Some("...")` slots of every
  `Citation` static array under `src/`, then cross-references them
  against (a) the cached Retraction Watch snapshot at
  `benchmarks/retraction-watch/cache.csv` (SHA-256-pinned by
  `cache.sha256`; mismatch fails CI) and (b) Crossref Works'
  `update-to` field with `type: "retraction"` (skipped under
  `--no-network`). `.github/workflows/citations.yml` runs the
  monitor on every push / PR and a Mondays-06:00-UTC cron refreshes
  the cache via the Crossref Labs Retraction Watch endpoint, opening
  a `chore(citations): refresh Retraction Watch cache` PR when the
  snapshot changes (gated on the `RETRACTION_WATCH_EMAIL` repo
  secret). The fixture under
  `tests/fixtures/retraction-watch/{citations.md,cache.csv,cache.sha256}`
  plants a synthetic-DOI retraction (`10.99999/cntrdct-q6-...`); the
  workflow's smoke step asserts the script exits 1 on it, so a future
  loosening of the matcher fails CI rather than silently re-opening
  the path. Evidence: Fong & Wilhite (2017) PLOS ONE 12(12),
  e0187394; COPE (2019) discussion document on citation
  manipulation.

Q-7. Venue tier whitelist

- Status: `[x]` 2026-05-07
- Summary: `docs/spec/citations-policy.md` carries a "Venue tier
  whitelist" section enumerating Tier-A (ICSE / FSE / OOPSLA /
  PLDI / POPL / ASE / ISSTA / EMSE / TOSEM / IEEE TSE plus
  adjacent SOSP / OSDI / EuroSys / NeurIPS / ICML / USENIX
  Security / S&P / CCS) and Tier-B (ICPC / ICSM / ICSME / MSR /
  SANER / WCRE / SCAM / ICST / ISSRE / JSS / IST). Tier-C is
  documented but starts empty; entries emit CI warnings rather
  than failures so grandfather clauses stay workable.
  `tests/citations_consistency.rs` adds
  `every_shipped_detector_citation_has_known_tier`,
  `fabricated_fixture_venue_is_rejected`, and
  `venue_tier_examples_classify_as_documented`. The fixture's
  fabricated venue (`"Fixture"`) is asserted to be unrecognised so
  the rejection path is pinned structurally; all six shipped
  detectors classify into Tier-A or Tier-B.

Q-8. Preregistration deviation log

- Status: `[x]` 2026-05-07
- Summary: `prereg/deviations/<date>-<topic>.md` is the new
  audit-trail surface for any preregistration revision carrying
  a `Supersedes:` header. Three back-filled entries land the
  retroactive 2026-05-03 → 2026-05-05 → 2026-05-06 → 2026-05-07
  supersession chain:
  `prereg/deviations/2026-05-05-multilang-rollup.md`,
  `prereg/deviations/2026-05-06-clone-drift-python.md`,
  `prereg/deviations/2026-05-07-wild-beta-fp-reduction.md`.
  `tests/prereg_consistency.rs` adds three new tests:
  `every_supersession_has_a_matching_deviation_log`,
  `deviation_logs_carry_required_headers`, and
  `deviation_log_supersedes_resolves_to_a_real_prereg_file`. A
  future revision with a `Supersedes:` line but no matching
  `prereg/deviations/<date>-*.md` fails the suite. Ungrounded
  per-deviation rationale is the documented Q-8 failure mode (van
  den Akker et al. 2024, doi:10.1037/met0000687); the three
  required headers (`Prereg:` / `Supersedes:` / `Author:` /
  `Date:`) keep the audit trail machine-checkable.

Q-9. Python attribute-style suppression

- Status: `[x]` 2026-05-07
- Summary: the wholesale Python skip in
  `collect_attribute_suppressions` (formerly an early
  `if file.language != Language::Rust { return vec![]; }`) is
  replaced by a per-language dispatch
  (`collect_rust_suppressions` / `collect_python_suppressions`).
  The Python path drives tree-sitter-python via
  `crate::parsers::parser_for(Language::Python)` (Q-10 seam) and
  recognises two forms of `# cntrdct: allow(<id>, ...)`:
  - Trailing comment on a code line — suppression range is the
    single comment line.
  - Standalone whole-line comment — suppression range covers the
    next non-comment named sibling (function / class / statement),
    mirroring the Rust attribute-precedes-item shape.
  `# cntrdct: allow()` is the catch-all (matches the Rust empty
  argument list). New unit tests in `src/config.rs` cover trailing
  / standalone / catch-all / wrong-id paths; integration tests in
  `tests/multilang_config.rs` (`python_attribute_allow_*`) drive the
  full scan + apply pipeline through both forms over the existing
  `PYTHON_ARG_SWAP` corpus and confirm that Rust findings on the
  same scan stay intact. The Q-10 parser seam was extended to
  cover `src/config.rs` at the same time, so adding M-7+ languages
  is still a single-module change in `src/parsers.rs`.

Q-10. ParserProvider seam tightening

- Status: `[x]` 2026-05-07
- Summary: every detector now reaches tree-sitter through
  `crate::parsers::parser_for(Language::*).ts_language()`. Eleven
  direct call sites across `arg_swap`, `clone_drift`,
  `comment_code`, `config_interaction`,
  `unreachable_after_terminator`, `pr_miner::extract_rust`, and
  `pr_miner::extract_python` were rewritten. A new
  `parser seam` step in `.github/workflows/ci.yml` greps
  `src/detectors/` for `tree_sitter_*::language()` and fails CI on
  any reintroduction, so a future M-7+ language addition is a
  single-module change in `src/parsers.rs`.

Q-11. Small-N statistical interval switching

- Status: `[x]` 2026-05-10
- Summary: `compute_priors` now switches between Wilson and a
  Beta(1, 1) Bayes-Laplace 95% lower bound based on cell size.
  The switch lives in `compute_lower_bound(tp, fp)` in
  `src/calibration.rs`: at `tp + fp >= SMALL_SAMPLE_THRESHOLD`
  (n = 30) it returns Wilson; below that, the Beta(1, 1) lower
  2.5% quantile, with the BCD 2001 §4 boundary modification at
  `tp = 0` (return 0 to align with Wilson at the same cell). The
  new `PriorMethod` enum is stored on `DetectorPrior` and propagated
  through `RankedFinding.prior_method` and SARIF
  `result.properties.priorMethod`. Field name `wilson_lower_95`
  is preserved on `DetectorPrior` for back-compat with pre-Q-11
  per-user cache files; `serde(default)` keeps old JSON loadable.
  `tests/ranker_small_sample.rs` (8 tests) gates the switching
  threshold, the boundary modification, distinguishability of the
  two methods at intermediate `(tp, fp)`, both methods staying
  near nominal one-sided lower coverage at `n >= 30`, and
  end-to-end `prior_method` propagation through the calibrated
  ranker, the uncalibrated fallback, and the SARIF emitter.
  Embedded `benchmarks/priors-default.json` regenerated: five of
  six shipped detectors now carry `prior_method: "jeffreys"`
  (only `pr-miner` at n=38 stays on Wilson). Spec
  `docs/spec/ranker-v1.md` adds the Q-11 section. CITATIONS.md
  adds `brown-cai-dasgupta-stat-sci-2001` and `thulin-ejs-2014`.
- Honesty note: the original "Jeffreys is closer to nominal than
  Wilson at n < 30" framing in the acceptance criterion does not
  hold robustly under one-sided lower coverage averaged over `p`
  (debug numbers: at small `n`, Wilson's mean coverage error sits
  marginally below Jeffreys' on a uniform-`p` grid, regardless of
  the boundary modification). The realised acceptance test gates
  the structurally provable properties Q-11 actually depends on
  rather than the brittle coverage-superiority claim. The
  calibrator still picks Jeffreys at `n < 30` for the
  methodological reason captured in `docs/spec/ranker-v1.md`
  ("Q-11 design notes"): `posterior_tp` is already a Beta(1, 1)
  Bayesian update, so a Beta(1, 1) credible-interval lower bound
  is the regime-coherent companion at small `n`.
- Evidence: Brown, Cai, DasGupta (2001) Statistical Science
  16(2), 101-133, doi:10.1214/ss/1009213286 (boundary modification
  + small-N regime); Thulin (2014) Electronic Journal of
  Statistics 8(1), 817-840, doi:10.1214/14-EJS909 (independent
  argument for Beta-prior credible bounds at small N).

Q-12. LLM calibration post-hoc Platt fit

- Status: `[x]` 2026-05-10
- Summary: the adjudicator prompt no longer requests a verbalised
  `calibration_tag`; the response parser still reads the field
  (`Option<String>`) so adjudication records collected before Q-12
  round-trip cleanly. Post-hoc Platt scaling fit per
  `(detector_id, anomaly_class)` cell replaces the verbalised tag.
  `cntrdct calibrate --fit-platt <CORPUS>` (extension of the
  existing `calibrate` subcommand) reads a JSONL of
  `LabelledLlmConfidence` rows and writes the per-cell `(a, b)`
  registry to
  `benchmarks/llm-calibration/platt-default.json` (or `--output
  <PATH>`); the file is `include_str!`-embedded into the binary so
  a fresh `cargo install cntrdct` ships with calibration ready.
  v0 ships an empty `{}` registry so `apply_llm_calibration` is a
  no-op until a real labelled adjudication corpus is fit. Wiring:
  `AdjudicationResult.calibrated_confidence: Option<f64>`,
  `cntrdct::apply_llm_calibration`, SARIF
  `result.properties.adjudication.calibrated_confidence` (omitted
  when `None`). `tests/calibration_ece.rs` runs end-to-end on a
  constructed-pathology fixture (over-confidence at 0.95/0.85/0.75
  raw with empirical accuracy ≈ 0.5) and asserts holdout ECE drops
  by ≥ 0.05 after Platt; on the shipped fixture raw ECE 0.256 →
  calibrated ECE near 0.001. Spec
  `docs/spec/llm-calibration-v0.md`. Citations: `platt-1999` and
  `spiess-koohestani-sergeyuk-2025` added under Layer 3
  (CITATIONS.md + `ADJUDICATOR_CITATIONS`).
- Evidence: Spiess, Koohestani, Sergeyuk (2025) arXiv:2510.22614;
  Spiess et al. (2025) ICSE 2025; J. Platt (1999), "Probabilistic
  Outputs for Support Vector Machines and Comparisons to
  Regularized Likelihood Methods", Advances in Large Margin
  Classifiers (MIT Press).

Q-13. Cross-model κ audit

- Status: `[x]` 2026-05-11 (CLI-shellout redesign 2026-05-11)
- Summary: on-demand 2-family κ audit between Claude Code's
  `claude --print` and the Gemini CLI's `gemini -p`. Three providers
  ship behind the new `PromptDispatch` trait:
  `AnthropicAdjudicator` (HTTP via `reqwest`, retained for
  `scan --adjudicate`), `ClaudeCliAdjudicator` (CLI shellout with
  `--system-prompt` / `--tools ""` / `--strict-mcp-config` /
  `--no-session-persistence` / `--output-format json` so Claude
  Code's agentic persona and tool surface are fully stripped), and
  `GeminiCliAdjudicator` (CLI shellout with `GEMINI_SYSTEM_MD` env
  override pointing at a temp file, `--output-format json`). Both
  CLI providers spawn the subprocess with `current_dir = <tempdir>`
  to suppress CLAUDE.md / GEMINI.md auto-discovery. Auth is
  delegated to each CLI's own login (no API keys read by cntrdct).
  Module `src/cross_model_kappa.rs` carries the pure `cohen_kappa`
  helper, per-`(detector_id, anomaly_class)` aggregation, the
  audit-report serde shapes, and stdlib-only date helpers. CLI:
  `cntrdct cross-model-kappa <CORPUS>` accepts JSONL or JSON-array
  ranked-finding corpora; default output is pretty JSON to stdout,
  `--output PATH` writes to disk. PR CI exercises κ aggregation via
  `tests/cross_model_kappa.rs` with `CannedDispatch` and pins the
  CLI flag set via stub-script tests in `src/adjudicator.rs::tests`.
- Design pivots (documented in
  `docs/spec/cross-model-kappa-v0.md` "Design rationale"):
  - Codex CLI dropped because `codex exec` cannot replace the
    system prompt (only `developer_instructions` additive), so
    Codex's residual persona would have confounded the κ signal.
  - OpenAI / Google API-key paths replaced by CLI shellout — users
    authenticate via subscription, not API keys.
  - Nightly CI workflow dropped. Continuous monitoring was unsupported
    by measurement stationarity: commercial LLMs version-bump
    silently, sampler stochasticity at temperature 0 still produces
    variance, and the time-series κ would have captured noise more
    than any cntrdct-side property. The audit ships as an
    on-demand snapshot only.
- Evidence: Wataoka, Takahashi, Ri (2024) arXiv:2410.21819;
  Zheng et al. (2023) NeurIPS 36, 46595-46623; Cohen (1960) and
  Landis & Koch (1977) for the κ statistic and substantial-agreement
  threshold.
- Spec: `docs/spec/cross-model-kappa-v0.md`.

Q-14. Recall-audit harness

- Status: `[~]` (Phase A scaffolding + Phase B first batch
  landed 2026-05-11; Phase B batch 2 (deepening
  `unreachable-after-terminator` measurement) landed 2026-05-12;
  Phase B batch 3 (broadening to a third detector,
  `comment-code`, via the Tan SOSP 2007 Pattern C bug from
  `sidan-lab/whisky-archive`) landed 2026-05-12;
  Phase C release-tag refresh discipline landed 2026-05-12;
  Phase B batch 4 (introducing the `paper-appendix` source kind
  via three PyPIBugs ArgSwap entries on permissive-licensed
  Python repositories — `c137digital/unv_app` MIT,
  `mwouts/nbrmd` MIT, `markokr/rarfile` ISC; all three are FN
  against cntrdct's narrow Rice-2017 arg-swap detector,
  documenting the gap between PyPIBugs labels and the detector's
  same-file + bare-identifier scope) landed 2026-05-12;
  pr-miner was the originally chosen batch 4 detector but
  punted on the structural blocker that the extractor walks
  only top-level `fn` / `def`, which collides with modern Rust
  RAII and Python `with` idioms — paired-API patterns survive
  almost exclusively in class methods the extractor drops;
  further Phase B batches broadening detector and source
  coverage (clone-drift, config-interaction, pr-miner, plus
  semgrep / codeql / clippy source kinds) still pending)
- Goal: counter the labeller-bias loop where cntrdct's priors are
  derived from corpora it labelled itself, biasing toward
  precision and silently sacrificing recall. Build
  `benchmarks/audit-corpus/` from the union of NVD / OSV.dev
  CVEs and findings from independent SAST tools (Semgrep,
  CodeQL community, Clippy), then add `cntrdct calibrate
  --audit-recall` to report per-detector recall upper bounds
  quarterly.
- Acceptance: the audit corpus README cites every CVE / external
  finding source with a stable URL; `cntrdct calibrate
  --audit-recall` produces non-trivial recall numbers for all
  six detectors; the README publishes the latest figures.
- Effort: 4-6 weeks.
- Depends on: P-1 (corpus tooling).
- Evidence: Heckman & Williams (2011) Information and Software
  Technology 53(4), 363-387 (selection-bias warning for
  actionable-alert pipelines).
- Phase A — harness scaffolding (done 2026-05-11): the `cntrdct
  calibrate --audit-recall <CORPUS_DIR> [--manifest <PATH>]
  [--output <PATH>]` flag set ships behind `clap`'s
  `conflicts_with` against `--fit-platt`, polymorphically
  reinterpreting the `corpus` positional as a directory in this
  mode. New module `src/recall_audit.rs` carries the audit-corpus
  manifest schema (`AuditExpectedFinding` requiring an
  `external_source: {kind, ref, url}` block per labelled
  finding), the `audit_recall(...) -> RecallAuditReport` pure
  function, and a `source_breakdown` per-detector tally so a
  detector whose recall is dominated by one source surfaces
  visibly. The orchestrator `cntrdct::run_recall_audit` mirrors
  `run_eval` (manifest validation → scan → audit). Twelve tests
  (4 unit + 8 integration) pin the loader edge cases, the
  matching arithmetic, byte-stable JSON, and the CLI conflict
  surface; `tests/fixtures/recall-audit/` carries a 2-detector /
  2-source synthetic fixture so PR CI exercises the path
  independently of Phase B data. `benchmarks/audit-corpus/`
  ships as a skeleton with the source list and per-detector
  seed targets documented; `manifest.jsonl` is empty pending
  Phase B. CITATIONS.md adds `heckman-williams-ist-2011` under
  Layer 2. Spec: `docs/spec/recall-audit-v0.md`.
- Phase B — audit-corpus data collection: per-detector seed
  lists land in `benchmarks/audit-corpus/files/` + the manifest,
  with each `expected[].external_source.url` resolving to a
  stable external page. Per-detector recall figures populate the
  README's "Latest audit run" section.
  - First batch (done 2026-05-11): six expected entries across
    two detectors and two sources. Three rust-lang/rust ui-test
    files for the rustc `unreachable_code` lint
    (`tests/ui/reachable/{unreachable-code-ret,expr_block,expr_if}.rs`
    at commit `4b0c9d76`, MIT OR Apache-2.0, stripped of the
    file-level `#![deny(unreachable_code)]` because cntrdct's
    SUPPRESSION_TOKEN scan would otherwise honour it) contribute
    five entries; one minimal extract of `totalsegmentator/statistics.py`
    at the pre-fix parent of PR #556 contributes one arg-swap
    entry. Audit numbers: `unreachable-after-terminator`
    tp=3 / fn=2 / recall_upper_bound=0.60; `arg-swap`
    tp=0 / fn=1 / recall_upper_bound=0.00; overall recall_upper_bound
    0.50. The 0.60 figure is the honest signal — cntrdct's
    statement-level terminator scan misses cases where the
    terminator lives inside an `if` / `if-else` expression. The
    arg-swap miss is also expected: the reverse-permutation
    heuristic does not catch swaps where the call-site
    identifiers do not share names with the parameter list.
    Source breakdown is preserved per detector so a future
    detector dominated by a single source surfaces visibly.
  - Second batch (done 2026-05-12): three additional rust-lang/rust
    ui-test files at the same pinned commit `4b0c9d76` —
    `tests/ui/reachable/{expr_return,expr_call,expr_loop}.rs`,
    each stripped of `#![deny(unreachable_code)]` on the same
    grounds as batch 1, contributing six additional
    `unreachable-after-terminator` entries. The batch intentionally
    deepens rather than broadens — every new entry pins a control-
    flow shape cntrdct's `rust_terminator_kind` classifier ignores
    by design: `expr_return.rs` puts `return` inside a tail
    expression (`let x: () = {return ...};`), `expr_call.rs`
    places `return` as a call argument (`foo(return, 22)`,
    `bar(return)`), `expr_loop.rs` uses divergent
    `loop { return; }` as the terminator (which the rustc lint
    recognises as never-typed but cntrdct treats as just another
    `loop_expression`). All six new entries are FN. Updated audit
    numbers: `unreachable-after-terminator` tp=3 / fn=8 /
    recall_upper_bound=0.27 (3/11); `arg-swap` unchanged at
    tp=0 / fn=1; overall recall_upper_bound 0.25 (3/12). The
    drop from 0.50 to 0.25 is the audit doing its job — the
    rustc lint's denominator captures shapes cntrdct's
    statement-level scan does not look through, and closing the
    gap is detector-improvement work (separate engineering,
    separate preregistration), not audit-harness work.
  - Third batch (done 2026-05-12): four expected entries adding
    a third detector, `comment-code`, to the corpus. Source:
    `sidan-lab/whisky-archive` `packages/whisky-common/src/data/
    primitives/constructors.rs` at commit `99243766`,
    Apache-2.0 licensed, verbatim copy. The `con_str` /
    `con_str0` / `con_str1` / `con_str2` family (upstream lines
    64 / 69 / 74 / 79; audit-corpus lines 68 / 73 / 78 / 83
    after the 3-line provenance header + 1 blank-line offset)
    each carries `/// Deprecated: Use ... instead.` prose but
    ships without the `#[deprecated]` runtime attribute — the
    textbook Pattern C bug Tan SOSP 2007 §3.2 ("bad comment")
    describes. All four are TP, so comment-code enters the
    corpus with single-source recall_upper_bound 1.00. Updated
    audit numbers: `comment-code` tp=4 / fn=0 / 1.00;
    `unreachable-after-terminator` tp=3 / fn=8 / 0.27 unchanged;
    `arg-swap` tp=0 / fn=1 / 0.00 unchanged; overall
    recall_upper_bound 0.4375 (7/16, up from 0.25). The lift is
    upward for the right reason: a new detector entered with
    high single-source recall, not because the existing
    detectors improved. Clone-drift was the original batch-3
    target but punted to a later batch: the published
    peer-reviewed clone-drift bug catalogues (Bettenburg MSR
    2009, Krinke ICSM 2007) target C/Java rather than
    Rust/Python, and the Assi TOSEM 2025 deep-learning-framework
    genealogies (mia1q/code-clone-DL-frameworks replication
    CSVs) expose size-2 clone pairs that fall under cntrdct's
    `MIN_GROUP_SIZE = 3` floor by construction.
  - Subsequent batches will broaden source coverage (semgrep,
    codeql, clippy, paper-appendix) and add the three remaining
    detectors (clone-drift, config-interaction, pr-miner).
    PyPIBugs (Allamanis NeurIPS 2021) is the natural next
    external source for arg-swap and clone-drift; the metadata
    file is hosted at Microsoft Download Center and requires
    offline preprocessing before commits can be cited with
    stable URLs.
- Phase C — release-tag refresh discipline (done 2026-05-12):
  `benchmarks/audit-corpus/README.md` carries a new "Refresh
  discipline (Phase C)" section enumerating the on-tag procedure
  (re-run the audit, refresh the "Latest audit run" table,
  stage the README change in the same `chore(release): bump
  version` commit, no follow-up). `CLAUDE.md` "Release
  procedure" steps now include `cargo run --release --bin
  cntrdct -- calibrate --audit-recall benchmarks/audit-corpus`
  between the lockfile sync and the commit, and the
  non-negotiables list pins the same-commit rule and the no-op
  refresh policy (figures unchanged = no-op is fine; the
  discipline is the re-run, not the delta). No CI enforcement
  — the rationale tracks Q-13's: audit-recall is a property of
  the embedded priors and the shipped detector logic, both of
  which CI already gates, so a README-update gate would catch
  the same drift twice. The release-procedure non-negotiable is
  the gap-closer.

Q-15. SOTA baseline comparators

- Status: `[ ]`
- Goal: publish `cntrdct eval` with side-by-side precision /
  recall / F1 against state-of-the-art comparators on the same
  corpus. Pilot baselines: SourcererCC (Sajnani et al. 2016) for
  clone-drift and PyBugLab (Allamanis et al. 2021) for arg-swap.
  Each baseline ships as a Docker image so the comparison is
  reproducible from a clean environment.
- Acceptance: `cntrdct eval --baseline sourcerercc,pybuglab`
  produces a comparison table with cntrdct's numbers and each
  baseline's numbers; the table is linked from the README so the
  detector-level recall gap is on the record rather than
  implicit.
- Effort: 3-4 weeks.
- Depends on: Q-14 (so the corpus contains TPs the baselines can
  catch).

Q-16. cargo-mutants nightly mutation testing

- Status: `[x]` 2026-05-11
- Summary: `.github/workflows/mutants.yml` runs cargo-mutants on
  every UTC night at 06:00 (also on `workflow_dispatch`).
  `.cargo/mutants.toml` scopes the run to `src/detectors/**/*.rs`
  via `examine_globs`; the rest of the codebase is intentionally
  out of scope for this gate. The workflow installs cargo-mutants
  via `taiki-e/install-action`, runs `cargo mutants --no-shuffle
  -j 2`, treats exit code 2 (some missed) as expected, and tallies
  `mutants.out/{caught,missed,unviable,timeout}.txt` to compute a
  catch rate. The step writes a markdown table to
  `$GITHUB_STEP_SUMMARY` plus the verbatim missed-mutant list, then
  fails the job when `caught / (caught + missed) < 0.80`.
  `mutants.out/` is uploaded as an artifact (30-day retention) for
  off-runner inspection. `.gitignore` adds `/mutants.out/` and
  `/mutants.out.old/` so accidental local runs do not leak the
  per-mutant log dirs into commits.
- Caveat: cargo-mutants is too slow to validate locally (multi-hour
  runs even on six detectors), so the first nightly run on master is
  the real signal for whether the codebase already satisfies the
  80% gate. If the first run fails, follow-up work is to either
  strengthen the test suite at the unguarded judgement boundaries
  the missed-mutants list calls out, or temporarily relax the gate
  while the detector tests catch up — both are roadmap-scope
  decisions, not config tweaks.
- Evidence: Just, Jalali, Inozemtseva, Ernst, Holmes, Fraser (2014)
  "Are mutants a valid substitute for real faults in software
  testing?" FSE 2014 (mutation-detection ↔ real-bug detection
  agreement); cargo-mutants project documentation
  (<https://mutants.rs/>) for the per-mutant test-rerun semantics.

Future Q-series candidates (not yet scheduled):

- Apriori v1 → FP-growth in pr-miner. Already noted in
  `docs/spec/pr-miner-v0.md` future work; revisit once Q-15 is
  in place so before/after F1 numbers are publishable on a
  consistent baseline.
- Layer 3 ML-detector ensemble. Run PyBugLab / GraphCodeBERT
  alongside the LLM judge; preserves Layer 1-2 / Layer 4
  determinism while lifting the recall ceiling. This crosses
  the P3 boundary as currently written and would require a new
  OSF preregistration, so it stays out of the numbered Q-series
  until that prereg lands.

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

24. T4-17 / T4-18 / T4-19 community scaffolding (landed)
24a. T4-20 / T4-21 deferred per maintainer decision
25. T3-15 git-cliff release-notes pipeline (landed 2026-05-08)
26. T3-16 telemetry-free assurance (landed 2026-05-08)
27. T3-14 distribution channels — cargo-binstall + Homebrew tap
    landed 2026-05-08; AUR dropped from scope
28. T3-12 LSP server — Phase 1 scaffolding + Phase 1.b document
    events + Finding -> Diagnostic mapping landed 2026-05-08;
    Phase 1.c per-URI didChange debouncing landed 2026-05-09;
    Phase 1.c+ per-URI generation counter landed 2026-05-09; Phase
    2 (vscode-cntrdct extension) next
29. T3-13 mdBook user guide (essay migration to external blog
    precedes Jekyll retirement, see T3-13 note)

Phase G (post-beta.1 quality-audit RC1 blockers; 1-2 days total,
required before the next release tag):

28. Q-1 SARIF detectors array missing pr-miner
29. Q-2 SARIF informationUri placeholder
30. Q-3 clone-drift doc-comment / value drift
31. Q-4 wiring consistency test
32. Q-5 SARIF Severity::Info mapping rationale

Phase H (RC1 governance and hygiene; 2-3 weeks, in parallel with
the Phase F community items):

33. Q-6 citation retraction monitor
34. Q-7 venue tier whitelist
35. Q-8 preregistration deviation log
36. Q-9 Python attribute-style suppression
37. Q-10 ParserProvider seam tightening

Phase I (RC2 / v0.2.0 methodology lift; 2-3 months):

38. Q-11 small-N statistical interval switching (landed 2026-05-10)
39. Q-12 LLM calibration post-hoc Platt fit (landed 2026-05-10)
40. Q-13 cross-model κ audit (landed 2026-05-11)
41. Q-14 recall-audit harness — Phase A scaffolding + Phase B
    first batch landed 2026-05-11; Phase B batch 2 (deepening
    `unreachable-after-terminator` recall measurement against
    additional rustc UI testset files), Phase B batch 3
    (broadening to a third detector, `comment-code`, via the
    Tan SOSP 2007 Pattern C bug in `sidan-lab/whisky-archive`),
    Phase C release-tag refresh discipline (README
    "Refresh discipline" section + CLAUDE.md release-procedure
    steps), and Phase B batch 4 (introducing the
    `paper-appendix` source kind via three PyPIBugs ArgSwap
    entries on permissive-licensed Python repositories —
    `c137digital/unv_app` MIT, `mwouts/nbrmd` MIT,
    `markokr/rarfile` ISC; all three FN against cntrdct's
    narrow Rice-2017 arg-swap detector, surfacing the gap
    rather than inflating it) all landed 2026-05-12; pr-miner
    was the originally chosen batch 4 detector but punted on
    the structural blocker that the extractor walks only
    top-level `fn` / `def`; further Phase B batches broadening
    detector and source coverage (semgrep / codeql / clippy
    cases for clone-drift / config-interaction / pr-miner)
    still pending
42. Q-15 SOTA baseline comparators
43. Q-16 cargo-mutants nightly mutation testing (landed 2026-05-11)

The split between Phase A (Tier 1, blocking) and later phases is the
single most important boundary in this roadmap. Everything in Phase A
should be done before any external announcement; everything after
Phase A can be sequenced based on signal from early users. Phase G
plays the same role for the next release tag (RC1) as Phase A did
for the first announcement.
