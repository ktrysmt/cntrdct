# Claude Code skill

With the `cntrdct` binary on `PATH`, Claude Code users can invoke
`/cntrdct` to run a scan and have the top findings summarised in
chat. The skill definition lives under
[`.claude/skills/cntrdct/`](https://github.com/ktrysmt/cntrdct/tree/master/.claude/skills/cntrdct)
in the repo.

The skill is purely a thin wrapper around the same `cntrdct scan`
pipeline a CLI user would invoke. It does not enable adjudication by
default; pass `--adjudicate` explicitly if an `ANTHROPIC_API_KEY` is
present in the shell environment Claude Code inherits.
