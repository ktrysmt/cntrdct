# Release: push the ONE new tag explicitly, never `git push --follow-tags`

`git push --follow-tags` pushes every annotated tag reachable from the
pushed commits that origin lacks — not just the tag you just created.
Because tag pushes trigger `.github/workflows/release.yml`, any stale
local annotated tag that was never pushed will start its OWN release run
(cross-target build → GitHub Release → crates.io publish → changelog
push to master).

- VERIFIED (2026-07-10, v0.14.0 release): the local repo carried unpushed
  annotated tags `v0.12.0` and `v0.13.0` (versions that were bumped then
  superseded by 0.12.1 / 0.13.1 and NEVER published to crates.io — the
  crates.io version list skips 0.12.0 and 0.13.0). `git push --follow-tags
  origin master` pushed all three tags and kicked off three release runs.
  The v0.12.0 / v0.13.0 runs would have published brand-new 0.12.0 / 0.13.0
  crates (their publish-crate verify `tag == Cargo.toml@tag` passes and
  crates.io had no such version to 409 against). Caught and `gh run cancel`
  ed both while still in the build stage — all their jobs show `cancelled`,
  so nothing published — then deleted the two remote tags to restore origin.

## Rule

Push master and the single intended tag EXPLICITLY:

```sh
git push origin master
git push origin v X.Y.Z        # exactly the tag you just created
```

Do NOT use `git push --follow-tags` for releases. Before pushing, audit
for stray local tags that origin lacks:

```sh
comm -23 <(git tag | sort) <(git ls-remote --tags origin | sed 's#.*refs/tags/##;s/\^{}//' | sort -u)
```

If that list contains anything other than the tag you mean to release,
either delete the stray local tags first or push only the intended tag by name.

Cross-checks used to confirm no damage: `gh run view <id> --json jobs`
(every job `cancelled`), and the crates.io version list
(`curl -s https://crates.io/api/v1/crates/cntrdct | jq '.versions[].num'`).
Remote tag deletion (`git push origin :refs/tags/vX.Y.Z`) does NOT trigger
release.yml, so it is a safe cleanup.
