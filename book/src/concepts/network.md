# Network access policy (P3)

`scan`, `calibrate`, and `eval` never open a socket. Two subcommands
talk to the network and both are opt-in:

- `scan --adjudicate` — Layer 3 LLM adjudicator, gated behind
  `ANTHROPIC_API_KEY`, hits the Anthropic Messages API directly.
- `cross-model-kappa` — Q-13 cross-model audit. cntrdct itself does
  not open sockets here; it shells out to `claude --print` and
  `gemini -p`, and those CLIs handle their own auth. No API keys are
  read by cntrdct.

The design property is enforced by the `network-isolation` job in
`.github/workflows/ci.yml`, which runs `cntrdct scan` inside a fresh
Linux network namespace (`sudo unshare --net`) on every push and
pull request. Any unintended socket open fails the job with
`ENETUNREACH` / `EAI_*`. There is no opt-out — adding a non-adjudicator
network path on `scan`, `calibrate`, or `eval` breaks both the P3
constraint and the netns gate.

The `reqwest` dependency is reachable only from `src/adjudicator.rs`
and the `build_default_adjudicator` constructor in `src/lib.rs`.
