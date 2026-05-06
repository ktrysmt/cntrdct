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
  `core`, `parsers`, `config`, `sarif`, `calibration`, `ranker`, `eval`,
  `adjudicator`, and `detectors::{arg_swap,clone_drift,comment_code,
  config_interaction,pr_miner,unreachable_after_terminator}`.
- Tests: `tests/*.rs` (one file per integration scope).
- Fixtures: `fixtures/*` (referenced by `tests/calibration_lib.rs`).
- Binaries: `cntrdct` (main) and `cargo-cntrdct` (shim that lets
  `cargo cntrdct ...` work; same code path as `cntrdct ...`).
- Subcommands: `scan`, `calibrate`, `eval`.
- Scope: shippable detector / linter product, preregistered evaluation,
  citation policy, multi-language detector ports.
- Owns at repo root: `prereg/`, `docs/surveys/`, `CITATIONS.md`,
  `ROADMAP.md`, `benchmarks/`, `examples/`, `scripts/`.
- History: was a 15-crate workspace (`crates/{core,parsers,config,sarif,
  calibration,ranker,eval,adjudicator-llm,detector-*,cli}`) until
  v0.2.0-beta.0 prep collapsed everything into one package. If you find
  a reference to `crates/<X>/src/lib.rs`, the equivalent is
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
