<!--
Thanks for contributing to cntrdct.

Pick a Conventional Commit prefix in the PR title:
  feat / fix / docs / chore / test / ci
Append `!` for a breaking change. See CONTRIBUTING.md for the full list.
-->

## Summary

<!-- 1-3 sentences: what changes and why. Link the issue or the
design doc this closes. -->

## Checklist

- [ ] Conventional Commit prefix in title and commits
- [ ] Commits are signed off (`git commit -s`; DCO line `Signed-off-by:` present)
- [ ] Did not stage unrelated files; staging used explicit paths, not `git add -A`

## Detector / corpus changes

Skip this section if your PR does not add a detector, extend an existing
detector to a new language, or modify the corpus.

- [ ] Citation added or updated in `CITATIONS.md` with a `Languages:` line
- [ ] Per-language coverage justified per `docs/spec/citations-policy.md` clause (a), (b), or (c) — or a `docs/surveys/<detector>-<lang>-<YYYY-MM>.md` records the gap and the finding emits `LanguageCitationStatus::Unconfirmed`
- [ ] At least 8 positive corpus cases per supported language under `benchmarks/corpus/files/`
- [ ] `tests/citations_consistency.rs` and `tests/corpus_shape.rs` updated as needed

## Gates

Run from the repo root:

- [ ] `cargo test --all-targets`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo fmt --all -- --check`

## Notes for the reviewer

<!-- Anything non-obvious: design tradeoffs considered, alternatives
rejected, follow-ups deliberately deferred. Optional. -->
