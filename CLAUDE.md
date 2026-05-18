# Repository guide for Claude Code

This repository hosts TWO independent cargo projects. The boundary between
them is load-bearing — confusing the two produces broken builds and silent
contract drift. Read this file before editing or running gates.

## Layout

### Technical package (root)

- Manifest: `Cargo.toml` (single `[package]`, no `[workspace]`)
- Lockfile: `Cargo.lock`
- Build artefacts: `target/`
- Source: `src/{lib.rs,main.rs,cargo_subcommand.rs}` plus modules
  `core`, `parsers`, `config`, `sarif`, `calibration`,
  `llm_calibration`, `cross_model_kappa`, `ranker`, `eval`,
  `adjudicator`, and
  `detectors::{arg_swap,clone_drift,comment_code,config_interaction,
  pr_miner,unreachable_after_terminator}`.
- Tests: `tests/*.rs` (one file per integration scope).
- Fixtures: `fixtures/*` (referenced by `tests/calibration_lib.rs`)
  and `tests/fixtures/*` (per-test-file inputs, e.g. the Q-12
  calibration corpus).
- Binaries: `cntrdct` (main) and `cargo-cntrdct` (shim that lets
  `cargo cntrdct ...` work; same code path as `cntrdct ...`).
- Subcommands: `scan`, `calibrate` (`--fit-platt` switches it to
  Q-12 LLM-confidence calibration mode; default mode produces P-4
  detector priors), `eval`, `cross-model-kappa` (Q-13: shells out to
  `claude --print` and `gemini -p`, reports pairwise Cohen's κ; auth
  via each CLI's own login, no API keys read by cntrdct).
- Scope: shippable detector / linter product, preregistered evaluation,
  citation policy, multi-language detector ports.
- Owns at repo root: `prereg/`, `docs/surveys/`, `CITATIONS.md`,
  `ROADMAP.md`, `benchmarks/`, `examples/`, `scripts/`.
- History: collapsed from a 15-crate workspace
  (`crates/{core,parsers,config,sarif,calibration,ranker,eval,
  adjudicator-llm,detector-*,cli}`) into one package during
  v0.2.0-beta.0 prep. The "Editing checklist" below has the full
  rename table; the short version is `crates/<X>/src/lib.rs` ->
  `src/<X>.rs` (or `src/detectors/<id>.rs` for detectors).

### Research workspace (`research/`)

- Manifest: `research/Cargo.toml` (members under `research/*`)
- Lockfile: `research/Cargo.lock`
- Build artefacts: `research/target/`
- Binary: `cntrdct-research`
- Subcommands: `fetch`, `aggregate`, `overlap`, `clippy`, `sample`, `rank`
- Scope: corpus mining, replication / position projects, exploratory
  tooling that has not been promoted into the product.
- May stand up its own preregistration discipline under
  `research/prereg/`, `research/surveys/`, `research/CITATIONS.md`.
  Do not share these with the root-level technical files.

## Design constraints (P1 - P5)

The technical package ships under five hard constraints. They are
enforced at startup, in tests, or in code review; violating one is
treated as a regression, not a stylistic choice. The README is
end-user-only and does NOT document them, so reproduce here:

- P1 — every detector cites peer-reviewed prior art.
  `core::register_detector` rejects any `Detector` whose
  `citations()` returns empty; `tests/citations_consistency.rs`
  asserts that every key resolves to an entry in `CITATIONS.md`.
- P2 — empirical results carry a preregistration id
  (`DetectorConfig::preregistration_id`); see also
  `## Preregistration discipline` below.
- P3 — only the Layer 3 adjudicator may invoke an LLM. Layers 1, 2,
  and 4 are deterministic, including the Q-12 post-processing helper
  `apply_llm_calibration`. `reqwest` is reachable only from
  `src/adjudicator.rs::ReqwestClient` and the
  `build_default_adjudicator` constructor in `src/lib.rs` (used by
  `scan --adjudicate`). The Q-13 cross-model audit
  (`run_cross_model_audit` + `build_audit_claude_cli_provider` +
  `build_audit_gemini_cli_provider`) does NOT open a socket from
  cntrdct itself — it shells out to `claude --print` and
  `gemini -p`, which handle auth and HTTP themselves. The
  `network-isolation` CI job (`.github/workflows/ci.yml`) runs
  `cntrdct scan` inside a fresh Linux network namespace
  (`sudo unshare --net`) on every push / PR; any unintended socket
  open fails the job with `ENETUNREACH` / `EAI_*`. No opt-out for
  `scan` / `calibrate` / `eval`; adding a non-adjudicator network
  path on any of those breaks both P3 and the netns gate. The Q-13
  `cross-model-kappa` subcommand is excluded from the netns gate by
  design — it spawns subprocesses that themselves talk to the
  network, same shape as `scan --adjudicate`.
- P4 — statistical priors come from labelled corpora, not from
  prompts or hardcoded constants. The Layer 2 priors pipeline lives
  under `src/calibration.rs` + `src/ranker.rs`; the embedded
  defaults at `benchmarks/priors-default.json` come from
  `cntrdct calibrate` against `benchmarks/labelled-findings.jsonl`,
  not from hand-authored numbers. The Q-12 Layer 3 extension
  (`src/llm_calibration.rs`) follows the same rule: the embedded
  `benchmarks/llm-calibration/platt-default.json` Platt registry is
  produced by `cntrdct calibrate --fit-platt` against a labelled
  LLM-confidence corpus. v0 ships an empty registry and applies a
  no-op fallback rather than authoring numbers in code.
- P5 — severities map to IEEE 1044-2009 anomaly classes at SARIF
  emission time. The mapping lives in `src/sarif.rs`;
  `tests/sarif_lib.rs` pins it.

## Layer mapping

The four-layer architecture is implicit in the module list above
but worth pinning:

- Layer 1 — deterministic detectors
  (`src/detectors/{arg_swap, clone_drift, comment_code,
  config_interaction, pr_miner, unreachable_after_terminator}`).
  Tree-sitter based; no LLM, no network.
- Layer 2 — statistical ranker (`src/ranker.rs` +
  `src/calibration.rs`). Wilson / Jeffreys lower bound × log-scaled
  sibling count. Auto-picks calibrated vs. uncalibrated per
  `pick_ranker` in `src/lib.rs`.
- Layer 3 — LLM adjudicator (`src/adjudicator.rs`). The sole layer
  permitted to invoke an LLM. Three providers ship:
  `AnthropicAdjudicator` (HTTP via `reqwest`, used by
  `scan --adjudicate`), `ClaudeCliAdjudicator` (Q-13 CLI shellout to
  `claude --print` with `--system-prompt` / `--tools ""` /
  `--strict-mcp-config` / `--no-session-persistence` /
  `--output-format json`), and `GeminiCliAdjudicator` (Q-13 CLI
  shellout to `gemini -p` with `GEMINI_SYSTEM_MD` env override and
  `--output-format json`). All three implement `PromptDispatch`. The
  Q-13 cross-model audit (`src/cross_model_kappa.rs`) consumes the
  two CLI providers and reports pairwise Cohen's κ per
  `(detector_id, anomaly_class)` cell on demand — there is no
  nightly cadence (see `docs/spec/cross-model-kappa-v0.md`
  "Design rationale" for why continuous monitoring was dropped).
  Verdict confidence is post-hoc Platt-calibrated
  (`src/llm_calibration.rs`, Q-12) when a fitted registry is
  present; v0 ships empty so
  `AdjudicationResult.calibrated_confidence` stays `None`.
- Layer 4 — SARIF 2.1.0 emitter (`src/sarif.rs`). IEEE 1044-2009
  compatible severity / anomaly class mapping.

## Boundary contract (do not violate)

1. **No cross-project path dependencies.** Root `Cargo.toml` MUST NOT
   reference `path = "research/..."` and `research/*` MUST NOT reference
   `path = "../src/..."` or any other technical-side path. Intra-workspace
   path deps are fine inside `research/` (e.g. `research/cli-research`
   depending on `path = "../corpus-fetch"`). CI does not enforce the
   cross-project ban structurally; the discipline is on us.
2. **No shared `Cargo.lock`.** Each project resolves independently.
3. **Promotion is explicit and manual.** Moving a research artefact into
   the technical product is NOT `git mv`. Re-implement it under
   `src/` (or extend an existing technical module) and prefix the
   commit `promote(<area>): ...`. The two projects are NOT a staging
   pipeline; do not assume research code will eventually flow into
   technical.
4. **CLI surface is split.** `cntrdct` exposes only `scan`, `calibrate`,
   `eval`. Anything else — `fetch`, `aggregate`, `overlap`, `clippy`,
   `sample`, `rank` — lives on `cntrdct-research`. Update scripts and
   docs accordingly when touching them; never reintroduce the old form
   on `cntrdct`.

## Working in the right context

When you edit a file, run gates for the project that owns it. When a
change spans both projects, run gates for both.

```sh
# Technical package (run from repo root)
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check

# Research workspace (run from research/)
cd research
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

CI runs both: a `research-clippy-test` job mirrors the technical job in
`working-directory: research`. Independent cadences are intended, but
the YAML alone does NOT enforce that — both jobs are top-level and run
in parallel. To make a research-side failure not block a technical
merge, GitHub branch protection's "required status checks" must be
narrowed to only the technical jobs (`clippy-test (technical, ...)`,
`fmt`, `licenses`, `sarif`); leaving `research-clippy-test` out of the
required set is what realises the independence in practice.

## Editing checklist

Before editing, locate the file:

- Path under `src/`, `tests/`, `examples/`, `benchmarks/`, `docs/`,
  `prereg/`, or any repo-root file (`Cargo.toml`, `README.md`, etc.) ->
  technical package, gate from repo root.
- Path under `research/*` -> research workspace, gate from `research/`.
- A research-track change should not touch the technical surface and
  vice versa, unless the change is an explicit `promote(<area>): ...`
  commit.
- Stale paths from prior layouts:
  - `crates/<X>/src/lib.rs` -> `src/<X>.rs`
  - `crates/detector-<id>/src/lib.rs` -> `src/detectors/<id>.rs` (or
    `src/detectors/<id>/mod.rs` for multi-file detectors like pr_miner)
  - `crates/cli/{src,tests}/...` -> `src/...` and `tests/...` at root
  - `crates/<X>/tests/integration.rs` -> `tests/<X>_lib.rs` (lib-scope)
    or `tests/detector_<id>.rs` (detector-scope)
  - `crates/calibration/fixtures/example_corpus.jsonl` -> `fixtures/example_corpus.jsonl`
  If a parallel session shows you edits against the old paths,
  re-target them before committing.

When proposing or implementing a promotion:

- Do not `git mv`. Re-implement under `src/` so the technical product's
  history reflects deliberate intake, not an accidental shuffle.
- Use commit prefix `promote(<area>): <summary>`.
- Verify both projects still pass their gates after the promotion.

## Commit conventions

- Conventional Commits prefixes are in use: `feat(scope)`, `fix(scope)`,
  `chore(scope)`, `docs(scope)`, `ci`, `test(scope)`.
- Use `promote(<area>)` for research-to-technical promotions.
- Append `!` after the scope for breaking changes
  (e.g. `chore(release)!: collapse 15 crates into single package`).

## Release procedure (technical package)

Releasing the root `cntrdct` package to crates.io and GitHub Releases is
driven entirely by pushing an annotated `vX.Y.Z` tag.
`.github/workflows/release.yml` runs `build` (cross-target binaries) ->
`release` (GitHub Release) and `publish-crate` (crates.io) on tag push.
The `publish-crate` job verifies that the tag (with the leading `v`
stripped) equals `Cargo.toml`'s `version`, so a mismatch fails the job
before anything reaches crates.io.

Only `Cargo.toml` is the source of truth for the version; `Cargo.lock`
is kept in sync via cargo. Other `0.2.0`-shaped strings in the repo
(`ROADMAP.md` history, `research/Cargo.toml`, docs, workflow examples)
are NOT version-tracking and must not be bumped as part of the release.

Steps (run from repo root):

```sh
$EDITOR Cargo.toml                                 # bump version to X.Y.Z
cargo update -p cntrdct                            # sync Cargo.lock
cargo run --release --bin cntrdct -- \
  calibrate --audit-recall benchmarks/audit-corpus  # Q-14 Phase C
$EDITOR benchmarks/audit-corpus/README.md          # refresh "Latest audit run"
git add Cargo.toml Cargo.lock benchmarks/audit-corpus/README.md
git commit -m "chore(release): bump version to X.Y.Z"
git tag -a vX.Y.Z -m "release vX.Y.Z"              # MUST be annotated
git push --follow-tags
```

Non-negotiable details:

- Tag MUST be annotated (`git tag -a` or `-s`). `git push --follow-tags`
  silently skips lightweight tags, so a plain `git tag vX.Y.Z` will not
  trigger CI.
- Tag name MUST be `v` + the exact `Cargo.toml` version, including any
  pre-release suffix (e.g. `v0.3.0`, or `v0.4.0-rc.1` if a future cycle
  re-introduces pre-releases). The CI verify step strips the leading
  `v` and demands an exact match.
- crates.io publishes are irreversible. A given version can be published
  exactly once; subsequent attempts get `409 already exists`. To recover
  from a bad release, bump the version again and tag anew.
- `cargo install cntrdct` ignores pre-release versions by default. If a
  future release cuts a pre-release suffix (`-alpha.N` / `-beta.N` /
  `-rc.N`), document the explicit `--version X.Y.Z-suffix` invocation
  for users.
- The CI gate is build-only across four targets; it does not re-run
  `cargo test` / `clippy` / `fmt`. Run the standard root-package gates
  (see "Working in the right context") before tagging.
- The research workspace has its own version cycle and is NOT released
  through this procedure. Do not bump `research/Cargo.toml` here.
- The GitHub Release body is generated by `git-cliff` via `cliff.toml`
  on every tag push (`--latest --strip header`), grouping commits since
  the previous tag by Conventional Commits prefix. Commits that do not
  parse as Conventional Commits (or that match `chore(release)` /
  `chore(changelog)` / `Merge`) are dropped from the notes by design;
  do not bypass the parser by adding ad-hoc release-only commits.
- The Q-14 recall-audit refresh lands in the SAME
  `chore(release): bump version to X.Y.Z` commit, not a follow-up.
  Re-run `cargo run --release --bin cntrdct -- calibrate
  --audit-recall benchmarks/audit-corpus` and update the README's
  "Latest audit run" table per `benchmarks/audit-corpus/README.md`
  "Refresh discipline (Phase C)". A no-op refresh (figures
  unchanged) is fine — the discipline is the re-run, not the
  delta. CI does not enforce this; it is part of the
  release-procedure non-negotiables for the same reason the tag
  itself is.

## Preregistration discipline (technical workspace)

- `prereg/` files at repo root are FROZEN once their associated phase
  begins (per the parent OSF preregistration). Subsequent rule changes
  go to a new dated file (`YYYY-MM-DD-osf-prereg.md`) that names the
  file it supersedes via a `Supersedes:` line in its front matter;
  never edit a frozen file in place. The consistency test picks the
  alphabetically last `*.md` in `prereg/`, so ISO date prefixes sort
  to the latest revision automatically.
- The consistency test at `tests/prereg_consistency.rs`
  picks the alphabetically last `*.md` in `prereg/` as the canonical
  preregistration. Sibling artefacts (rubrics, addenda) must be
  filtered there if they do not follow the OSF schema.
- If the research track wants its own preregistration cadence, host it
  under `research/prereg/` rather than mixing into the root-level set.
