<!--
Thanks for contributing to cntrdct.

Pick a Conventional Commit prefix in the PR title:
  feat / fix / docs / chore / test / ci / promote(<area>)
Append `!` for a breaking change. See CONTRIBUTING.md for the full list.
-->

## Summary

<!-- 1-3 sentences: what changes and why. Link the ROADMAP entry, issue,
or design doc this closes. -->

## Workspace touched

- [ ] Technical (root `Cargo.toml`, `crates/*`, repo-root docs)
- [ ] Research (`research/Cargo.toml`, `research/*`)
- [ ] Both (must be a `promote(<area>): ...` commit; re-implementation, not `git mv`)

## Checklist

- [ ] Conventional Commit prefix in title and commits
- [ ] Commits are signed off (`git commit -s`; DCO line `Signed-off-by:` present)
- [ ] Did not stage unrelated files; staging used explicit paths, not `git add -A`
- [ ] Did not introduce a path dependency across the technical / research boundary
- [ ] ROADMAP entry updated to `[x]` (or a new entry added) when this PR closes a roadmap item

## Detector / corpus changes

Skip this section if your PR does not add a detector, extend an existing
detector to a new language, or modify the corpus.

- [ ] Citation added or updated in `CITATIONS.md` with a `Languages:` line
- [ ] Per-language coverage justified per `docs/spec/citations-policy.md` clause (a), (b), or (c) — or a `docs/surveys/<detector>-<lang>-<YYYY-MM>.md` records the gap and the finding emits `LanguageCitationStatus::Unconfirmed`
- [ ] At least 8 positive corpus cases per supported language under `benchmarks/corpus/files/`
- [ ] `crates/cli/tests/citations_consistency.rs` and `crates/cli/tests/corpus_shape.rs` updated as needed

## Gates

Run the gates for the workspace(s) you touched. Both must pass for a
PR that spans both workspaces.

Technical (from repo root):

- [ ] `cargo test --workspace`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo fmt --all -- --check`

Research (from `research/`):

- [ ] `cargo test --workspace`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo fmt --all -- --check`

## Notes for the reviewer

<!-- Anything non-obvious: design tradeoffs considered, alternatives
rejected, follow-ups deliberately deferred. Optional. -->
