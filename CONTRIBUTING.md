# Contributing to cntrdct

Thanks for taking the time to look. cntrdct is a small project with a
sharp design constraint (every detector must cite peer-reviewed prior
art); this guide tells you how to satisfy it without surprises.

If anything below contradicts `CLAUDE.md` at the repo root, treat
`CLAUDE.md` as authoritative — it is the file the maintainers and
their tooling read first.

## Workspace layout

The repository hosts two independent cargo workspaces. The boundary
between them is load-bearing.

Technical package (root):

- Manifest: `Cargo.toml` (single `[package]`)
- Lockfile: `Cargo.lock`
- Build artefacts: `target/`
- Source: `src/{lib.rs,main.rs,cargo_subcommand.rs}` plus per-layer
  modules (`core`, `parsers`, `config`, `sarif`, `calibration`,
  `ranker`, `eval`, `adjudicator`, `detectors::*`).
- Tests: `tests/*.rs`. Fixtures: `fixtures/*`.
- Binaries: `cntrdct` and `cargo-cntrdct`
- Subcommands: `scan`, `calibrate`, `eval`
- Scope: shippable detector / linter product, preregistered
  evaluation, citation policy, multi-language detector ports.

Research workspace (`research/`):

- Manifest: `research/Cargo.toml`, members under `research/*`
- Lockfile: `research/Cargo.lock`
- Build artefacts: `research/target/`
- Binary: `cntrdct-research`
- Subcommands: `fetch`, `aggregate`, `overlap`, `clippy`, `sample`, `rank`
- Scope: corpus mining, replication / position projects, exploratory
  tooling that has not been promoted into the product.

Boundary contract:

- No `path = "research/..."` in the root `Cargo.toml`; no
  `path = "../src/..."` (or any other technical-side path) in
  `research/*`.
- The two projects resolve independently and have separate
  `Cargo.lock` files.
- Promotion from research to technical is not `git mv`. Re-implement
  under `src/` and prefix the commit `promote(<area>): ...`.

## Local dev loop

Run the gates for the project that owns the file you edited. If a
PR spans both, run both.

Technical (from repo root):

```sh
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

Research (from `research/`):

```sh
cd research
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

CI runs both via parallel jobs (`clippy-test` for technical and
`research-clippy-test` for research). Branch protection requires the
technical jobs only; research-side failures do not block a technical
merge.

## Authoring a new detector

P1 (every detector must cite peer-reviewed prior art) is the gate. The
flow is:

1. Open a Detector proposal issue using
   `.github/ISSUE_TEMPLATE/detector_proposal.md`. The required citation
   field exists so we agree the detector can ship before code is
   written.
2. After the proposal is approved, draft a spec at
   `docs/spec/<detector>-v0.md`. Existing specs (e.g.
   `docs/spec/pr-miner-v0.md`) are the canonical templates: detection
   rule, AST shape, test plan T1..Tn, citation rationale, migration
   sequence if multi-language.
3. Add the citation entry to `CITATIONS.md` with a `Languages:` line
   per `docs/spec/citations-policy.md`. If the cited paper does not
   ground a target language under clause (a), (b), or (c), add a
   `docs/surveys/<detector>-<lang>-<YYYY-MM>.md` recording the survey
   effort and have the detector emit
   `LanguageCitationStatus::Unconfirmed` for that language.
4. Implement the detector at `src/detectors/<id>.rs` (or
   `src/detectors/<id>/mod.rs` if the implementation needs multiple
   files; pr_miner is the precedent). Register it in `src/lib.rs` (the
   `pub mod detectors;` declaration auto-discovers the file) and add
   the constructor call in `scan_full_with_config`. Wire its citations
   into the static `Citation` array; `register_detector` rejects
   detectors with empty `citations()`.
5. Add at least 8 positive cases and 3 negative cases per supported
   language under `benchmarks/corpus/files/`. Update
   `tests/corpus_shape.rs` if you introduce a new detector-id slot.
   Update `tests/citations_consistency.rs` so the new detector's
   citation keys resolve.
6. Open the PR. Use the PR template; tick the citation, corpus, and
   gate checkboxes.

## Adding a new language to an existing detector

Same flow as M-3 (see `ROADMAP.md`):

1. Run a literature survey for the target language. Record the
   outcome at `docs/surveys/<detector>-<lang>-<YYYY-MM>.md` whether or
   not it produces a citation.
2. If a qualifying citation exists, add it to `CITATIONS.md` with the
   `Languages:` line and update the detector's `Citation` array's
   `languages` field. The detector's findings emit
   `LanguageCitationStatus::Confirmed` for that language.
3. If no qualifying citation exists, point `CITATIONS.md` at the
   survey via the `unconfirmed:` form. Findings emit
   `LanguageCitationStatus::Unconfirmed`.
4. Extend `supported_languages()` and add corpus cases (≥ 8 positives,
   ≥ 3 negatives) in the new language.

## Citation format

`CITATIONS.md` is the bibliography. Each entry has the form:

```
- <citation-key> — <Authors>, "<Title>", <Venue> <Year>. <DOI or URL>.
  Languages: <comma-separated list, or `Rust (grandfathered)` for v0 entries>
```

Citation keys are kebab-case `<last-author>-<venue>-<year>`,
e.g. `cordy-roy-icpc-2008`, `assi-tosem-2025`. Same key referenced
from multiple detectors is fine; duplicates within one detector are
not (the consistency test catches this).

For the policy that decides whether a citation is grounded in a
specific language, see `docs/spec/citations-policy.md`. The short
form: clause (a) experimental subjects, (b) language-agnostic
algorithm with independent peer-reviewed secondary application, or
(c) the cited paper introduces a benchmark in that language.

## Commit conventions

Conventional Commits prefixes are in use:

- `feat(scope)` — new user-visible capability
- `fix(scope)` — bug fix
- `docs(scope)` — documentation only
- `chore(scope)` — tooling, refactors, or other internal changes
- `test(scope)` — test-only changes
- `ci` — workflow changes
- `promote(<area>)` — research-to-technical promotion

Append `!` after the scope for breaking changes
(e.g. `chore(workspace)!: split research crates`).

## Sign-off (DCO)

Every commit must carry a `Signed-off-by:` trailer. The simplest way
is `git commit -s`. By signing off you agree to the Developer
Certificate of Origin (<https://developercertificate.org>): your
contribution is yours to give and you are licensing it under the
project's terms.

We do not maintain a separate CLA. The DCO is sufficient.

## Pull request review

- One maintainer approval is required before merge.
- All required CI checks must be green: technical `clippy-test`,
  `fmt`, `licenses`, `sarif`. Research-side failures do not block.
- Squash on merge is the default. Keep the squashed commit message in
  Conventional Commits form: the release workflow runs `git-cliff`
  (config at `cliff.toml`) on every tag push and uses the grouped
  output as the GitHub Release body. The same workflow also
  regenerates the full `CHANGELOG.md` and commits it back to `master`
  as `chore(changelog): update for vX.Y.Z` (the parser skips that
  prefix so the bot commit does not pollute the next release notes).
  Off-shape commits silently fall out of both surfaces.
- Avoid force-pushes to a PR branch once review has started; prefer
  fixup commits so review threads stay anchored.

## Reporting bugs and proposing features

- Bugs: `.github/ISSUE_TEMPLATE/bug_report.md`. Include the smallest
  source that reproduces the false positive or the missed finding.
- Features: `.github/ISSUE_TEMPLATE/feature_request.md`. For new
  detectors, use the Detector proposal template instead.
- Detectors: `.github/ISSUE_TEMPLATE/detector_proposal.md`. Citation
  field is required upfront; this is where P1 starts.

## Conduct

The project does not yet ship a formal Code of Conduct. As maintainer
volume grows we expect to adopt the Contributor Covenant; for now,
default to professional, evidence-driven discussion and assume good
faith. Report any concern to the maintainers via the contact in
`README.md`.
