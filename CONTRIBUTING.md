# Contributing to cntrdct

Thanks for taking the time to look. cntrdct is a small project with a
sharp design constraint (every detector must cite peer-reviewed prior
art); this guide tells you how to satisfy it without surprises.

If anything below contradicts `CLAUDE.md` at the repo root, treat
`CLAUDE.md` as authoritative — it is the file the maintainers and
their tooling read first.

## Workspace layout

The repository is a single cargo project rooted at the repo root.

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

## Local dev loop

Run the gates from the repo root before committing:

```sh
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

The required CI status checks are `clippy-test`, `fmt`, `licenses`, and
`sarif`.

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

cntrdct routes all cross-cutting detectors through a language-agnostic
IR layer (`src/ir.rs`; spec `docs/spec/ir-v0.md`). Adding a new
language to every cross-cutting detector requires one parser converter
plus corpus and survey work, not detector edits.

1. Run a literature survey for the target language for each
   cross-cutting detector you intend to cover (arg-swap, clone-drift,
   comment-code, pr-miner, unreachable-after-terminator). Record
   each outcome at `docs/surveys/<detector>-<lang>-<YYYY-MM>.md`
   whether or not the survey produces a citation.
2. If a qualifying citation exists per `docs/spec/citations-policy.md`
   clause (a), (b), or (c): add it to `CITATIONS.md` with the
   `Languages:` line and update the relevant detector's `Citation`
   array's `languages` field. Findings emit
   `LanguageCitationStatus::Confirmed` for that (detector, language)
   pair.
3. If no qualifying citation exists: point `CITATIONS.md` at the
   survey via the `unconfirmed:` form. Findings emit
   `LanguageCitationStatus::Unconfirmed`.
4. Add the IR converter at `src/parsers/<lang>.rs` (tree-sitter AST
   → `IrFile`). Add the `tree-sitter-<lang>` Cargo dependency.
   Register the language in `src/core.rs::Language` and
   `src/parsers/mod.rs::parser_for`.
5. Extend each migrated detector's `supported_languages()` to
   include the new variant.
6. Add corpus cases (≥ 8 positives, ≥ 3 negatives) per opted-in
   detector under `benchmarks/wild-corpus-<lang>/`.

If you also want to add a language-specific detector (something
whose concept does not transfer across languages — e.g. a Go
build-tag interaction analogue of `config-interaction`), place it
under `src/detectors/lang/<lang>_<id>.rs`. Language-specific
detectors read tree-sitter ASTs directly and follow the same P1
citation rules as cross-cutting ones.

The transitional v0.5.x flow (per-detector `match Language::*` arms,
~1500-1900 LOC per language) is retired by R-1 along with the IR
migration. Until R-1 ships, follow the spec at
`docs/spec/multilang-v0.md` §F6 (Pattern A).

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

Append `!` after the scope for breaking changes
(e.g. `chore(release)!: collapse 15 crates into single package`).

## Sign-off (DCO)

Every commit must carry a `Signed-off-by:` trailer. The simplest way
is `git commit -s`. By signing off you agree to the Developer
Certificate of Origin (<https://developercertificate.org>): your
contribution is yours to give and you are licensing it under the
project's terms.

We do not maintain a separate CLA. The DCO is sufficient.

## Pull request review

- One maintainer approval is required before merge.
- All required CI checks must be green: `clippy-test`, `fmt`,
  `licenses`, `sarif`.
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
