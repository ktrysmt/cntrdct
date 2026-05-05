# Repository guide for Claude Code

This repository hosts TWO independent cargo workspaces. The boundary between
them is load-bearing — confusing the two produces broken builds and silent
contract drift. Read this file before editing or running gates.

## Workspaces

### Technical workspace (root)

- Manifest: `Cargo.toml` (members under `crates/*`)
- Lockfile: `Cargo.lock`
- Build artefacts: `target/`
- Binaries: `cntrdct` (main) and `cargo-cntrdct` (shim that lets
  `cargo cntrdct ...` work; same code path as `cntrdct ...`)
- Subcommands: `scan`, `calibrate`, `eval`
- Scope: shippable detector / linter product, preregistered evaluation,
  citation policy, multi-language detector ports.
- Owns at repo root: `prereg/`, `docs/surveys/`, `CITATIONS.md`,
  `ROADMAP.md`, `benchmarks/`, `examples/`, `scripts/`.

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

1. **No cross-workspace path dependencies.** `crates/*` MUST NOT use
   `path = "../research/..."` and `research/*` MUST NOT use
   `path = "../crates/..."`. Intra-workspace path deps are fine
   (e.g. `research/cli-research` depending on `path = "../corpus-fetch"`
   inside the same `research/` workspace). CI does not enforce the
   cross-workspace ban structurally; the discipline is on us.
2. **No shared `Cargo.lock`.** Each workspace resolves independently.
3. **Promotion is explicit and manual.** Moving a research artefact into
   the technical product is NOT `git mv`. Re-implement it under
   `crates/*` (or extend an existing technical crate) and prefix the
   commit `promote(<area>): ...`. The two workspaces are NOT a staging
   pipeline; do not assume research code will eventually flow into
   technical.
4. **CLI surface is split.** `cntrdct` exposes only `scan`, `calibrate`,
   `eval`. Anything else — `fetch`, `aggregate`, `overlap`, `clippy`,
   `sample`, `rank` — lives on `cntrdct-research`. Update scripts and
   docs accordingly when touching them; never reintroduce the old form
   on `cntrdct`.

## Working in the right context

When you edit a file, run gates for the workspace that owns it. When a
change spans both workspaces, run gates for both.

```sh
# Technical (run from repo root)
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check

# Research (run from research/)
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

- Path under `crates/*` -> technical workspace, gate from repo root.
- Path under `research/*` -> research workspace, gate from `research/`.
- A research-track change should not modify `crates/*` and vice versa,
  unless the change is an explicit `promote(<area>): ...` commit.
- Stale paths from before the workspace split no longer exist:
  `crates/corpus-fetch/*` moved to `research/corpus-fetch/*`, and the
  research halves of `crates/cli/src/{lib,main}.rs` plus
  `crates/cli/tests/{fetch,aggregate_sample,clippy_harness,overlap}.rs`
  moved into `research/cli-research/`. If a parallel session shows you
  edits against the old paths, re-target them before committing.

When proposing or implementing a promotion:

- Do not `git mv`. Re-implement under `crates/*` so the technical
  product's history reflects deliberate intake, not an accidental
  workspace shuffle.
- Use commit prefix `promote(<area>): <summary>`.
- Verify both workspaces still pass their gates after the promotion.

## Commit conventions

- Conventional Commits prefixes are in use: `feat(scope)`, `fix(scope)`,
  `chore(scope)`, `docs(scope)`, `ci`, `test(scope)`.
- Use `promote(<area>)` for research-to-technical promotions (new since
  the workspace split).
- Append `!` after the scope for breaking changes
  (e.g. `chore(workspace)!: split research crates`).

## Preregistration discipline (technical workspace)

- `prereg/` files at repo root are FROZEN once their associated phase
  begins (per the parent OSF preregistration). Subsequent rule changes
  go to a new dated file (`YYYY-MM-DD-osf-prereg.md`) that names the
  file it supersedes via a `Supersedes:` line in its front matter;
  never edit a frozen file in place. The consistency test picks the
  alphabetically last `*.md` in `prereg/`, so ISO date prefixes sort
  to the latest revision automatically.
- The consistency test at `crates/cli/tests/prereg_consistency.rs`
  picks the alphabetically last `*.md` in `prereg/` as the canonical
  preregistration. Sibling artefacts (rubrics, addenda) must be
  filtered there if they do not follow the OSF schema.
- If the research track wants its own preregistration cadence, host it
  under `research/prereg/` rather than mixing into the root-level set.
