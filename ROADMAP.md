# cntrdct implementation roadmap

Last updated: 2026-05-21. Current shipped version: v0.5.1. Audit
overall `recall_upper_bound = 0.92`, eval F1 = 0.94 (`comment-code`,
`config-interaction`, `pr-miner` saturated at 1.00; `unreachable-
after-terminator` 0.94; `clone-drift` 0.50; `arg-swap` 0.25 at the
SwapD-style name-correlation ceiling, see Q-17). Per-release notes
in `CHANGELOG.md`.

## Status legend

- `[x]` completed
- `[~]` in progress
- `[ ]` pending
- (retired) — preregistered then dropped without shipping; one line
  in place so the index stays stable

## In flight

The four items below are the active work surface. Everything else
on this page has either landed (one-line entries under
"Completed") or is a deliberately deferred / retired marker.

T3-12. LSP server

- Status: `[~]` Phase 1 (scaffolding + document events + debouncing
  + per-URI generation counter) landed 2026-05-08 – 09. Phase 2
  prereq (`cntrdct-lsp` binary shipping in GitHub Release archives
  alongside `cntrdct` / `cargo-cntrdct`) landed 2026-05-20.
- Phase 2 (pending): `vscode-cntrdct` extension scaffolding
  (TypeScript / pnpm) under a separate `ktrysmt/vscode-cntrdct`
  repo, bundling the LSP binary auto-downloaded from GitHub
  Releases.
- Phase 3 (pending): VS Code Marketplace listing + announcement.
- Spec: `docs/spec/lsp-v0.md`.

T3-13. mdBook user guide

- Status: `[~]` Phase 1 scaffolding landed 2026-05-20 — `book/`
  with Introduction / Getting-started / Concepts / Detectors /
  Configuration / Workflows / Integrations / FAQ chapters
  populated with substantive content; `book.toml` carries
  `git-repository-url` and `edit-url-template` pointing at master.
- Phase 2 (pending): GitHub Pages re-enable and README link
  add. Gated on the `docs/site/essays/` external-blog migration
  finishing first (T1-7 retired the original Pages workflow until
  the essay source is moved out).

Q-15. SOTA baseline comparators

- Status: `[~]` SourcererCC adapter (v0.4.0, 2026-05-19) and
  PyBugLab adapter (2026-05-20) scaffolding landed. Live Docker
  comparison numbers pending.
- Goal: `cntrdct eval --baseline <name>` publishes side-by-side
  P / R / F1 against external SOTA on the same corpora. Baselines
  ship as pinned Docker images for reproducibility.
- Shipped (Phases A + B): registry, `NormalisedFinding`,
  `load_baseline_jsonl`, `run_baseline_docker`
  (`--network=none --read-only`, digest-pinned), `compare_one`,
  `assemble_report` in `src/baselines.rs`. Wrapper Dockerfiles +
  `entrypoint.sh` + `UPSTREAM.md` under
  `baselines/{sourcerercc,pybuglab}/` with placeholder image
  digests. `cntrdct eval --baseline <name>[,…]` /
  `--baselines-out PATH` / `--baselines-skip-run` flags compose
  with the existing eval path. README's "Baseline comparison"
  section documents the entry point.
- Pending (Phase D — maintainer workstation): pin live image
  digests + upstream commit SHAs + (PyBugLab) pre-trained weights
  URL / SHA-256 / inference seed; run against audit-corpus +
  wild corpora; commit per-baseline JSONL under
  `benchmarks/baselines/v<release>/`; populate README's
  comparison table with real numbers.
- Spec: `docs/spec/sota-baselines-v0.md`.

Q-17. Layer 0 LLM candidate generator for semantic arg-swap (preregistered, blocked on P3 revisit)

- Status: `[ ]`
- Motivation: the audit-corpus arg-swap recall ceiling sits at 0.25
  (1 of 4 expected entries). The three FN cases —
  `unv_app_settings.py:41`, `nbrmd_test_ipynb_to_R.py:26`,
  `totalsegmentator_statistics.py:10` — were re-examined against
  the published state-of-the-art syntactic name-correlation
  algorithm (Scott et al. ASE 2020 SwapD, arXiv 2009.09117, §3.4
  cover-based checker). All three are unreachable by SwapD's
  morpheme-tokenisation + first-character similarity metric:
  intra-position common-morpheme elimination collapses the
  argument sets to ∅, or the surviving morphemes share no first
  character so coverage drops to 0 and the dual threshold
  `(<α₁=0.5, >α₂=0.75)` cannot be satisfied. The bugs are genuine
  semantic swaps (CT vs. segmentation, base vs. override,
  expected vs. actual) that require reasoning beyond identifier
  morphology — they sit in the Allamanis et al. NeurIPS 2021
  PyBugLab / LLM-adjudication band rather than the syntactic-Rice
  / SwapD band cntrdct's Layer 1 occupies. Documented in
  `docs/spec/arg-swap-v0.md` under "Name-correlation upper bound".
- Architectural problem: cntrdct's current P3 design constraint
  permits LLM use only on the Layer 3 adjudicator, which is a
  post-hoc filter over Layer 1 candidates. When Layer 1 produces
  zero candidates for a call site (because name correlation
  cannot fire), the adjudicator has nothing to adjudicate; the
  semantic swap stays invisible. Closing the gap requires either
  (a) a new "Layer 0 candidate generator" (any 2-arg same-file
  call where param/arg are simple identifiers becomes a candidate,
  and the adjudicator decides), or (b) extending Layer 3 with a
  candidate-generation mode (`scan --adjudicate-call-sites` with
  the LLM reading the AST directly). Either path crosses the P3
  boundary as currently written; before scheduling implementation
  work, the boundary itself needs an explicit revisit (cf. the
  existing "Layer 3 ML-detector ensemble" note in the Future
  candidates list).
- Scope when scheduled:
  - Candidate enumeration: same predicate as Layer 1 arg-swap's
    F3 / F4 (2-arg same-file call with bare-identifier params /
    args, after F3b's `self.X` / `cls.X` carve-outs) but WITHOUT
    the F5 name-match filter. Every binary call is a candidate.
  - Adjudicator contract: the existing `PromptDispatch` providers
    (`AnthropicAdjudicator`, `ClaudeCliAdjudicator`,
    `GeminiCliAdjudicator`) carry the prompt; the Layer 0
    generator enqueues one call per candidate. Output is a
    verdict per call.
  - Cost envelope: ≈ 1 LLM round-trip per call site. For a medium
    repo (~1k binary call sites) at Claude Opus 4.7 the input is
    ~1k × ~2k tokens ≈ 2M input tokens — non-trivial. Cost
    calibration is part of the work.
  - Calibration: the Q-12 Platt registry accepts
    `(detector_id, anomaly_class)` cells; Layer 0 findings would
    enter as a new cell tagged `arg-swap-semantic` and be
    Platt-fitted against a labelled extension of the audit
    corpus.
- Evidence (motivating literature):
  - Allamanis, Jackson-Flux, Brockschmidt (2021) "Self-Supervised
    Bug Detection and Repair", NeurIPS 2021. PyBugLab + PyPIBugs;
    arg-swap is one of four bug classes the GNN co-training
    targets. Already cited as `allamanis-neurips-2021` in
    `docs/spec/arg-swap-v0.md`.
  - Scott, Ranieri, Kot, Kashyap (2020) "Out of Sight, Out of
    Place: Detecting and Assessing Swapped Arguments", ASE 2020
    (arXiv 2009.09117). Consulted alongside Q-17 motivation; not
    a cntrdct citation because v0 does not adopt the SwapD
    algorithm, but useful for the upper-bound rationale.
  - Audit-corpus FN walkthrough lives at
    `docs/spec/arg-swap-v0.md` "Name-correlation upper bound";
    Q-17 inherits its three case studies as the work's pinning
    fixtures.
- Caveats:
  - P3 revisit MUST land first. Without it, Q-17 violates the
    architectural contract every detector test (and the
    `network-isolation` CI gate) is built around. The revisit is
    not Q-17's job; Q-17 only starts once a "Layer 0 / Layer 3+
    candidate origination" amendment exists.
  - Determinism is lost for Layer 0 findings. Q-13 cross-model κ
    audit becomes the only way to characterise variance, so Q-13
    coverage MUST be expanded to include the Q-17 cell before
    Layer 0 ships in any release.
  - Cost. The envelope above is the lower bound (one LLM call per
    candidate). Real deployments may need a cheap-prefilter (e.g.
    require param-identifier overlap < N morphemes) to drop the
    call-site count by an order of magnitude before paying for
    adjudication.
  - Generalisation. Q-17 frames this for arg-swap because that
    detector's upper bound is the immediate motivation, but the
    same architectural shape lifts ceilings on `clone-drift`
    (semantic clone divergence the syntactic detector misses)
    and on the F4f exception-handler Q-18 below. Q-17 should
    ship as the generic primitive, not an arg-swap-specific one.

Q-18. Python `except` handler reachability via exception-type analysis (preregistered, F4f)

- Status: `[ ]`
- Motivation: the audit-corpus `unreachable-after-terminator`
  recall sits at 0.941 (16 of 17 expected entries) after F4d-v +
  F4e landed in the v0.5.1 follow-up. The remaining FN is
  `codeql_python_unreachable_test.py:88` — CodeQL's
  `UnreachableCode` query (ODASA-5387 in the upstream fixture)
  flags the `except NameError:` handler inside `def odasa5387()`
  because `str` is a Python 3 builtin that can never raise
  `NameError`, making the handler unreachable. F4d / F4e classify
  reachability syntactically; this case requires reasoning about
  *which exception types the body inside `try:` can raise*, which
  is class-hierarchy + raise-set analysis, not AST-local.
  Documented as an explicit non-goal under
  `docs/spec/unreachable-after-terminator-v0.md` "F4d / F4e
  non-goals (preregistered)".
- Scope when scheduled (call this rule F4f):
  - Raise-set extraction. For each `try:` block, compute the set
    of exception classes the body may raise. v0 conservative
    sources:
    - bare `raise <Type>` / `raise <Type>(...)` statements
      (direct).
    - calls into Python builtins with documented exception
      contracts (`open` → `OSError`, `int` / `float` →
      `ValueError`, `dict[k]` → `KeyError`, `list[i]` →
      `IndexError`, `getattr` → `AttributeError`, `__import__` →
      `ImportError`, name lookups → `NameError` only when the
      target name is bound somewhere in scope or imported). The
      builtin contract table ships with the detector; lifting it
      to a config file is post-v0.
    - sub-expressions that themselves call known-raising
      builtins (e.g. `x = open(p)` contributes `OSError`).
  - Class-hierarchy check. For each `except <T>:` handler, flag
    the handler when no member of the raise-set is a subclass of
    `<T>`. Subclass relationships are read from a hardcoded
    Python 3 builtin exception hierarchy; user-defined exception
    classes default to "could match anything" (conservative, no
    flag) to avoid cross-file resolution work.
  - F4f Finding shape: `terminator_kind = "unreachable-except"`,
    `primary` = the `except` clause, `related` = the `try:` block
    whose raise-set is empty for this exception type.
  - Carve-outs:
    - `except Exception:` and `except BaseException:` never flag
      (they catch anything).
    - bare `except:` never flags (same).
    - `try:` bodies that contain a call to a function not on the
      builtin contract table never flag — the raise-set is
      "anything" by default.
- Evidence (motivating literature):
  - Hovemeyer & Pugh (2004) "Finding Bugs is Easy", OOPSLA 2004
    — already cited as `hovemeyer-pugh-oopsla-2004`. FindBugs's
    `REC_CATCH_EXCEPTION` family covers exception-handler
    reachability against the Java class hierarchy; F4f is the
    Python port of the same idea against CPython's documented
    exception tree.
  - Engler, Chen, Hallem, Chou, Chelf (2001) "Bugs as Deviant
    Behavior", SOSP 2001 — already cited as `engler-sosp-2001`.
    §4 covers "should-never-trigger" patterns including
    impossible exception handlers, framed as deviant code under
    the same statistical-anomaly umbrella the rest of the UAT
    detector lives under.
  - The CodeQL `UnreachableCode` test fixture (upstream line 84 /
    audit-corpus line 88, ODASA-5387) is the motivating positive;
    cntrdct should reproduce CodeQL's finding here.
- Caveats:
  - User-defined exception classes. Without cross-module
    resolution, F4f cannot know whether `MyError(ValueError)`
    extends `ValueError`. The v0 stance ("user-defined classes
    match anything") gives up some recall to keep precision
    high; lifting it later is a v1 widening, not a v0 spec gap.
  - Cross-file `raise`. A function `foo()` called inside `try:`
    might raise types not visible to the local file. F4f cannot
    follow function bodies (Layer 1 is AST-local). The carve-out
    above ("non-builtin call → raise-set is anything") covers
    this conservatively but means many real-world unreachable
    handlers stay invisible — the same conservatism CodeQL
    applies on its first pass before deeper data-flow analysis.
  - Builtin contract table provenance. The mapping
    `open → OSError`, `int → ValueError` etc. must cite the
    CPython documentation rather than be hand-authored. The
    table should ship as a JSON file under
    `data/python-builtin-exceptions.json` with each entry citing
    the relevant `docs.python.org/3/library/...` URL, so future
    Python version drift is auditable.
  - Wild-corpus FP measurement. Before F4f ships, run it against
    `benchmarks/wild-corpus-python/` with the table populated
    only for `open` / `int` / `float` (the highest-volume
    builtins) and inspect the FP set. The carve-out for
    user-defined exception classes should make wild-corpus FP
    rate near zero; if not, narrow the builtin contract table
    further before widening.

T4-20. Code of Conduct

- Status: `[ ]` deferred until external contributor activity or
  GitHub Discussions warrants the operational overhead (running an
  enforcement contact and triage path). At adoption time the file
  will be a short pointer to the canonical Contributor Covenant
  URL rather than an inline copy. `CONTRIBUTING.md` carries an
  interim conduct paragraph in the meantime.

T4-21. Roadmap discussion pinned

- Status: `[ ]` 15-minute task; deferred alongside T4-20 until
  Discussions are enabled on the repository.

## Completed (compact log)

Detailed implementation notes for each completed item live in
`CHANGELOG.md`, the per-detector specs under `docs/spec/`, the
commit history, and the surveys under `docs/surveys/`. The
one-liners below are an index, not a substitute.

### Practical track

- P-1 `[x]` Rust β corpus: `scripts/fetch_rust_corpus.py` → 36
  crates / 270 files / 13 MB under permissive licenses;
  hand-labelled findings (all v0 FPs) feed P-4.
- P-2 `[x]` pr-miner detector (multi-language). Apriori pair-rule
  mining over per-function call-site transactions. v0.5.0
  closed FM-A + FM-B via R6/R7/R8 (F4b item-cardinality post-
  filter; F4c per-language stop-list; F4d `pr_miner_eligible`
  manifest field). v0.5.1 followup added F4e Python carve-outs
  (`with X(...) as Y:` cleanup synthesis; non-identifier-chain
  attribute receivers dropped). Result: 16 TP / 0 FP / posterior_tp
  0.944 on the labelled corpus; precision = 1.000 on the audit
  corpus. Spec `docs/spec/pr-miner-v0.md`.
- P-3 `[x]` SARIF validation in CI via `Sarif.Multitool validate`
  against OASIS 2.1.0.
- P-4 `[x]` Layer 2 ranker recalibration on the β corpus.
  `cntrdct calibrate` writes `benchmarks/priors-default.json`,
  embedded into the binary via `include_str!`. Calibrated ranker
  reorders when Wilson disagrees with sibling-count baseline.
- P-5 `[x]` Release tagging + crates.io publish workflow.
  `cntrdct --version` wired; `cargo-cntrdct` shim inherits.
- P-6 `[x]` Wild-β FP reduction pass cut Rust 124 → 24 (80.6 %)
  and Python 19 → 4 (78.9 %) via structural fixes (UAT F4b/F4c,
  comment-code F5b/F5c, clone-drift F5b/F5c).
- P-7 `[x]` clone-drift within-scope residual cleanup. F5d
  sibling-family discriminator (3 sub-gates) takes wild β
  clone-drift FP to 0 in both languages; Wilson lower
  0.355 → 0.676 (8 TP / 0 FP).

### Tier 1 — usable OSS

- T1-1 `[x]` GitHub Actions CI (test + clippy + fmt, Linux + macOS
  matrix).
- T1-2 `[x]` crates.io metadata; `cargo publish --dry-run` green.
- T1-3 `[x]` README polish (badges, quickstart, P1-P5 one-liners).
- T1-4 `[x]` `examples/` ships scan / calibrate /
  adjudicate-with-mock-API scripts; CI smoke-tests each.
- T1-5 `[x]` rustdoc on every public item;
  `#![deny(missing_docs)]` enforced.
- T1-6 `[x]` MIT-only workspace; `cargo deny check licenses`
  passes.
- T1-7 (retired 2026-05-12) GitHub Pages essay site never
  served — Pages was never enabled on the repository, the
  `pages.yml` workflow failed on its first invocation, and the
  workflow was retired the same day. `docs/site/` source survives
  pending external-blog migration that gates T3-13 Phase 2.

### Tier 2 — adoption-grade

- T2-7 `[x]` Suppression: `#[cntrdct::allow(<id>)]` on items +
  project-wide `cntrdct.toml` for severity / threshold / per-path
  rules. Q-9 added Python `# cntrdct: allow(<id>)` line comments
  (trailing or whole-line) via a tree-sitter-python scanner.
- T2-8 `[x]` Pre-built release binaries for Linux x86_64/aarch64,
  macOS aarch64, Windows x86_64; `curl | sh` install path works
  end-to-end.
- T2-9 `[x]` GitHub Action wrapper consuming the pre-built binary,
  emitting findings as PR annotations.
- T2-10 `[x]` Parallel detection via rayon; per-file detector runs
  byte-identical to serial.
- T2-11 `[x]` `cargo-cntrdct` shim so `cargo cntrdct scan` works
  alongside `cntrdct scan`.

### Tier 3 — polish

- T3-12, T3-13: see In flight.
- T3-14 `[x]` Distribution channels: cargo-binstall metadata in
  `Cargo.toml`; Homebrew tap at `ktrysmt/homebrew-cntrdct`
  auto-bumped via `.github/workflows/homebrew.yml`
  (`HOMEBREW_TAP_TOKEN` secret). AUR dropped from scope per
  maintainer decision; reopen as a new T-series item if external
  demand materialises.
- T3-15 `[x]` Auto-generated release notes via `git-cliff`
  against Conventional Commits prefixes. v0.4.3 followup ships
  `CHANGELOG.md` as a checked-in artefact, regenerated by the
  `update-changelog` job on every tag push (`fetch-depth: 0`,
  pushes back as `chore(changelog): update for vX.Y.Z`).
- T3-16 `[x]` `network-isolation` CI job runs `cntrdct scan`
  inside `sudo unshare --net` on every push / PR; any unintended
  socket open fails the job with `ENETUNREACH` / `EAI_*`.
  `reqwest` reach stays constrained to `src/adjudicator.rs` and
  `src/lib.rs`'s `wire_adjudicator` constructor.

### Multi-language track (M-series)

- M-1 `[x]` Language abstraction: `cntrdct::parsers::Language`
  enum + extension mapping; `ParsedFile.language` is an enum, not
  a string.
- M-2 `[x]` Python pilot: `unreachable-after-terminator` extended
  to Python (`raise`, `sys.exit()`, `os._exit()`, `assert False`,
  trailing-`return`).
- M-3 `[x]` Cross-cutting detectors to Python:
  `clone-drift` / `arg-swap` / `comment-code` via internal
  `Language` dispatch. Citation status per
  `docs/surveys/*-python-2026-05.md`.
- M-4 `[x]` Python β corpus: 11 files / 5 packages from PyPI
  under `benchmarks/wild-corpus-python/`.
- M-5 `[x]` Multi-language surface: `[languages.<id>]` config
  section; GitHub Action `paths:` accepts `<path>:<lang_csv>`;
  mixed Rust+Python SARIF path test-pinned.
- M-6 `[x]` `docs/spec/citations-policy.md` codifies per-language
  citation requirements; `tests/citations_consistency.rs` enforces
  structurally.

### Tier 4 — community

- T4-17 `[x]` Issue templates (bug / feature / detector_proposal).
- T4-18 `[x]` PR template covering Conventional Commit prefix,
  DCO sign-off, detector / corpus checklist, gate boxes.
- T4-19 `[x]` `CONTRIBUTING.md` (two workspaces, `promote(<area>)`
  rule, detector authoring flow, Conventional Commits, DCO via
  `git commit -s` (no CLA)).
- T4-20, T4-21: see In flight.

### Quality-audit track (Q-series)

- Q-1 `[x]` SARIF `tool.driver.rules[]` includes pr-miner;
  `tests/multilang_config.rs` asserts the full
  `cntrdct::ALL_DETECTOR_IDS` set is present.
- Q-2 `[x]` `INFORMATION_URI` set to repo URL;
  `grep -RE 'TBD' src/` gate prevents reintroduction.
- Q-3 `[x]` clone-drift `NEAR_DUPLICATE_THRESHOLD` doc-comment
  updated to the 0.7 value the spec actually uses.
- Q-4 `[x]` `cntrdct::ALL_DETECTOR_IDS` + structural wiring tests
  in `tests/wiring_consistency.rs`.
- Q-5 `[x]` `Severity::Info` → SARIF `"none"` decision logged in
  `docs/spec/sarif-v0.md` F5.
- Q-6 `[x]` Citation retraction monitor:
  `scripts/check_retractions.py` cross-references every DOI in
  `CITATIONS.md` and `Citation` arrays against a SHA-256-pinned
  Retraction Watch cache + Crossref `update-to: retraction`.
- Q-7 (retired 2026-05-19) Venue tier whitelist was specific to
  the OSF preregistration audit trail; dropped with `prereg/` in
  v0.4.0.
- Q-8 (retired 2026-05-19) Preregistration deviation log; same
  reason as Q-7.
- Q-9 `[x]` Python tree-sitter suppression scanner;
  see T2-7.
- Q-10 `[x]` ParserProvider seam: every detector reaches
  tree-sitter through `crate::parsers::parser_for(Language::*)
  .ts_language()`. CI greps `src/detectors/` for
  `tree_sitter_*::language()` and fails on reintroduction.
- Q-11 `[x]` Small-N Wilson↔Jeffreys switching at n=30 boundary,
  with the BCD 2001 §4 boundary modification at `tp = 0`.
  `PriorMethod` propagated through SARIF `result.properties
  .priorMethod`. Five of six shipped detectors carry
  `prior_method: "jeffreys"`. Spec `docs/spec/ranker-v1.md`.
- Q-12 `[x]` Post-hoc Platt scaling per
  `(detector_id, anomaly_class)` cell.
  `cntrdct calibrate --fit-platt` writes
  `benchmarks/llm-calibration/platt-default.json` (v0 ships
  empty, applies no-op fallback). Spec
  `docs/spec/llm-calibration-v0.md`.
- Q-13 `[x]` On-demand cross-model κ audit between Claude Code's
  `claude --print` and Gemini CLI's `gemini -p` via the
  `PromptDispatch` trait + CLI shellout (no API keys read by
  cntrdct). Spec `docs/spec/cross-model-kappa-v0.md`.
- Q-14 `[x]` Audit-corpus recall harness:
  `cntrdct calibrate --audit-recall benchmarks/audit-corpus`
  against externally-sourced bug catalogues (NVD / OSV / Semgrep
  / CodeQL / Clippy / rustc lint testset / paper-appendix /
  upstream bug-fix commits). v0.5.1 figures: 49 corpus files,
  61 expected entries, overall `recall_upper_bound = 0.92`.
  Spec `docs/spec/recall-audit-v0.md`. Phase C release-tag
  refresh discipline retired with the OSF preregistration drop
  (v0.4.0); CI gates the embedded priors instead.
- Q-15: see In flight.
- Q-16 `[x]` `cargo-mutants` nightly mutation testing scoped to
  `src/detectors/**/*.rs`, fails when catch rate < 0.80.

## Future Q-series candidates (not scheduled)

- Apriori v1 → FP-growth in pr-miner. Noted in
  `docs/spec/pr-miner-v0.md` future work; revisit once Q-15
  Phase D lands so before/after F1 numbers are publishable on a
  consistent baseline.
- Layer 3 ML-detector ensemble. Run PyBugLab / GraphCodeBERT
  alongside the LLM judge; preserves Layer 1-2 / Layer 4
  determinism while lifting the recall ceiling. Crosses the P3
  boundary like Q-17; the broader ensemble lift would land on
  the same boundary amendment.

## Remaining execution sequence

Everything in "Completed" above has shipped. The active sequence
across "In flight" items is:

1. Q-15 Phase D — pin live image digests, run baselines against
   audit + wild corpora, populate README's comparison table.
2. T3-12 Phase 2 — `vscode-cntrdct` extension scaffolding.
3. T3-13 Phase 2 — gated on `docs/site/essays/` external-blog
   migration.
4. T4-20 / T4-21 — deferred until Discussions are enabled.
5. Q-17 / Q-18 — both preregistered; both require an explicit
   architecture amendment (P3 revisit for Q-17; spec extension
   with builtin-exception contract table for Q-18) before
   implementation work starts.
