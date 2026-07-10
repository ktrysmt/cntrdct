# Repository guide for Claude Code

This repository hosts a single cargo project: the `cntrdct` technical
package at the repo root. Read this file before editing or running gates.

## Layout

### Technical package (root)

- Manifest: `Cargo.toml` (single `[package]`, no `[workspace]`)
- Lockfile: `Cargo.lock`
- Build artefacts: `target/`
- Source: `src/{lib.rs,main.rs,cargo_subcommand.rs}` plus modules
  `core`, `parsers`, `config`, `sarif`, `calibration`,
  `llm_calibration`, `cross_model_kappa`, `ranker`, `eval`,
  `adjudicator`, `baseline` (B-1 ratchet:
  `docs/spec/baseline-v0.md` — NOT the Q-15 release-fixture
  "baselines" in `tests/baselines.rs`), and
  `detectors::{arg_swap,clone_drift,comment_code,config_interaction,
  pr_miner,unreachable_after_terminator}`.
- Tests: `tests/*.rs` (one file per integration scope).
- Fixtures: `fixtures/*` (referenced by `tests/calibration_lib.rs`)
  and `tests/fixtures/*` (per-test-file inputs, e.g. the Q-12
  calibration corpus).
- Binaries: `cntrdct` (main) and `cargo-cntrdct` (shim that lets
  `cargo cntrdct ...` work; same code path as `cntrdct ...`).
- Subcommands: `scan` (B-1 flags `--baseline` / `--write-baseline` /
  `--fail-on` with exit code 3 on threshold, S-1 stderr summary),
  `calibrate` (`--fit-platt` switches it to
  Q-12 LLM-confidence calibration mode; default mode produces P-4
  detector priors), `eval`, `cross-model-kappa` (Q-13: shells out to
  `claude --print` and `agy -p` — Google Antigravity, a non-Anthropic
  Gemini model, replacing the retired `gemini` CLI — reports pairwise
  Cohen's κ; auth via each CLI's own login, no API keys read by cntrdct).
- Scope: shippable detector / linter product, citation policy,
  multi-language detector ports.
- Owns at repo root: `docs/surveys/`, `CITATIONS.md`,
  `benchmarks/`, `examples/`, `scripts/`.
- History: collapsed from a 15-crate workspace
  (`crates/{core,parsers,config,sarif,calibration,ranker,eval,
  adjudicator-llm,detector-*,cli}`) into one package during
  v0.2.0-beta.0 prep. The "Editing checklist" below has the full
  rename table; the short version is `crates/<X>/src/lib.rs` ->
  `src/<X>.rs` (or `src/detectors/<id>.rs` for detectors).

## Design constraints (P1, P3 - P5)

The technical package ships under four hard constraints. They are
enforced at startup, in tests, or in code review; violating one is
treated as a regression, not a stylistic choice. The README is
end-user-only and does NOT document them, so reproduce here:

- P1 — every detector cites peer-reviewed prior art.
  `core::register_detector` rejects any `Detector` whose
  `citations()` returns empty; `tests/citations_consistency.rs`
  asserts that every key resolves to an entry in `CITATIONS.md`.
- P3 — only the Layer 3 adjudicator may invoke an LLM. Layers 1, 2,
  and 4 are deterministic, including the Q-12 post-processing helper
  `apply_llm_calibration`. `reqwest` is reachable only from
  `src/adjudicator.rs::ReqwestClient` and the
  `build_default_adjudicator` constructor in `src/lib.rs` (used by
  `scan --adjudicate --adjudicate-via=anthropic`, an explicit opt-in
  needing `ANTHROPIC_API_KEY`). The DEFAULT `scan --adjudicate` backend
  is `--adjudicate-via=claude-cli` (`claude --print` on Haiku, with an
  `agy` usage-cap fallback); `claude-cli` / `agy-cli` run Layer 3 over a
  CLI shellout (subscription auth, no API key, no reqwest path).
  The Q-13 cross-model audit (`run_cross_model_audit` +
  `build_audit_claude_cli_provider` + `build_audit_agy_cli_provider`)
  does NOT open a socket from cntrdct itself — it shells out to
  `claude --print` and `agy -p`, which handle auth and HTTP
  themselves. The
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
  `AnthropicAdjudicator` (HTTP via `reqwest`, explicit opt-in via
  `scan --adjudicate-via=anthropic`, needs `ANTHROPIC_API_KEY`),
  `ClaudeCliAdjudicator` (CLI shellout to `claude --print` with
  `--system-prompt` / `--tools ""` / `--strict-mcp-config` /
  `--no-session-persistence` / `--output-format json`; uses the Sonnet
  model as the Layer 0 PROPOSER and the Haiku model
  `CLAUDE_CLI_ADJUDICATE_MODEL` as the Layer 3 ADJUDICATOR), and
  `AgyCliAdjudicator` (CLI shellout to `agy`, Google Antigravity's
  multi-model CLI that replaced the retired `gemini` binary — no
  `--output-format json` / `--system-prompt`, so it parses the raw
  text response, folds a forceful closed-book system prompt into a
  COMPACT prompt body, and CRITICALLY orders args `--model <m> --print
  <prompt>` because `--print` takes the prompt as its value; the model
  is forced to a Gemini variant so the provider stays non-Anthropic).
  All three implement `PromptDispatch`; the two CLI providers also
  implement `Adjudicator`. The DEFAULT `scan --adjudicate` backend is
  `claude-cli` (Haiku) wrapped in a `FallbackAdjudicator` that switches
  to `agy` (Gemini) when the Claude subscription hits its usage cap
  (`is_usage_limit_error`) — both run on subscription auth with no
  `ANTHROPIC_API_KEY`. The Q-13 cross-model audit
  (`src/cross_model_kappa.rs`) consumes the `claude-cli` + `agy-cli`
  providers (genuinely cross-family) and reports pairwise Cohen's κ per
  `(detector_id, anomaly_class)` cell on demand — there is no
  nightly cadence (see `docs/spec/cross-model-kappa-v0.md`
  "Design rationale" for why continuous monitoring was dropped).
  Verdict confidence is post-hoc Platt-calibrated
  (`src/llm_calibration.rs`, Q-12) when a fitted registry is
  present; v0 ships empty so
  `AdjudicationResult.calibrated_confidence` stays `None`.
- Layer 4 — SARIF 2.1.0 emitter (`src/sarif.rs`). IEEE 1044-2009
  compatible severity / anomaly class mapping.

## Working in the right context

Run the gates from the repo root before committing:

```sh
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

The required CI status checks are `clippy-test (technical, ...)`, `fmt`,
`licenses`, and `sarif`.

## Editing checklist

Before editing, locate the file:

- Path under `src/`, `tests/`, `examples/`, `benchmarks/`, `docs/`, or
  any repo-root file (`Cargo.toml`, `README.md`, etc.).
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

## Commit conventions

- Conventional Commits prefixes are in use: `feat(scope)`, `fix(scope)`,
  `chore(scope)`, `docs(scope)`, `ci`, `test(scope)`.
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
(`CHANGELOG.md` history, docs, workflow examples) are NOT
version-tracking and must not be bumped as part of the release.

Steps (run from repo root):

```sh
$EDITOR Cargo.toml                                 # bump version to X.Y.Z
cargo update -p cntrdct                            # sync Cargo.lock
git mv tests/fixtures/baselines/baselines/v<old>/ \
       tests/fixtures/baselines/baselines/vX.Y.Z/  # Q-15 fixture pin
git add Cargo.toml Cargo.lock
git commit -m "chore(release): bump version to X.Y.Z"
git tag -a vX.Y.Z -m "release vX.Y.Z"              # MUST be annotated
git push --follow-tags
```

The fixture rename is required because `tests/baselines.rs` derives
the cached-JSONL release tag from `CARGO_PKG_VERSION` (Q-15 spec),
and the CI gate at tag time is build-only — a missed rename does not
fail the tagged release but does break `cargo test` on master until
the next person rebumps. Run `cargo test --all-targets` before tagging
to catch this and any other pre-existing breakage.

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
- The GitHub Release body is generated by `git-cliff` via `cliff.toml`
  on every tag push (`--latest --strip header`), grouping commits since
  the previous tag by Conventional Commits prefix. Commits that do not
  parse as Conventional Commits (or that match `chore(release)` /
  `chore(changelog)` / `Merge`) are dropped from the notes by design;
  do not bypass the parser by adding ad-hoc release-only commits.
- After the GitHub Release is published, the `update-changelog` job in
  `release.yml` regenerates the full `CHANGELOG.md` (no `--latest`) and
  pushes a `chore(changelog): update for vX.Y.Z` commit back to
  `master` via `GITHUB_TOKEN`. The parser also skips `chore(changelog)`
  so the bot commit is invisible to the next release's notes. Do not
  hand-edit `CHANGELOG.md`; if it drifts, re-run `git cliff --output
  CHANGELOG.md` locally and commit the result.
Optional but recommended:

- Re-running `cargo run --release --bin cntrdct -- calibrate
  --audit-recall benchmarks/audit-corpus` at release time and
  refreshing the README's "Latest audit run" table is good hygiene
  when detector logic has changed, but is no longer required.
