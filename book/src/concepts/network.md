# Network access policy (P3)

The default `cntrdct scan` pipeline never opens a socket. This is
the P3 design constraint, and it is enforced structurally in CI on
every push and pull request.

## What does not touch the network

| Subcommand | Network? |
|---|---|
| `cntrdct scan` (default) | No |
| `cntrdct scan --adjudicate` | Yes — Layer 3 LLM adjudicator (opt-in) |
| `cntrdct calibrate` | No |
| `cntrdct calibrate --fit-platt` | No |
| `cntrdct eval` | No |
| `cntrdct cross-model-kappa` | Indirect — shells out to CLIs that themselves talk to the network |

Layers 1 (detectors), 2 (ranker), and 4 (SARIF emitter) are
deterministic and offline. The Q-12 `apply_llm_calibration` helper
is Layer 2 / Layer 4 post-processing, also offline.

## The two opt-in network paths

`scan --adjudicate` invokes the Layer 3 adjudicator. It is gated
behind `ANTHROPIC_API_KEY` and hits the Anthropic Messages API via
`reqwest`. The `reqwest` dependency is reachable only from
`src/adjudicator.rs::ReqwestClient` and the
`build_default_adjudicator` constructor in `src/lib.rs` — adding a
non-adjudicator reach into `reqwest` is a P3 violation and breaks
the netns gate below.

`cntrdct cross-model-kappa` (Q-13) is the cross-model audit. It
shells out to `claude --print` and `gemini -p`, both of which handle
their own auth and HTTP. No API keys are read by cntrdct itself, and
no socket is opened from the cntrdct process. The subprocess that
talks to the network is the user's installed CLI.

## CI enforcement: the netns gate

`.github/workflows/ci.yml` runs a `network-isolation` job on every
push and PR. The job:

- Creates a fresh Linux network namespace via
  `sudo unshare --net`.
- Runs the full `cntrdct scan` path (walker -> parsers -> Layer 1
  detectors -> Layer 2 ranker -> Layer 4 SARIF emitter) inside that
  namespace.
- Asserts the emitted SARIF document is non-empty and well-formed.

The namespace has no outbound routes, so any unintended socket open
fails with `ENETUNREACH` or `EAI_*` and the job goes red. There is
no opt-out. Adding a non-adjudicator network path on `scan`,
`calibrate`, or `eval` breaks both the P3 constraint and the netns
gate.

The `cross-model-kappa` subcommand is excluded from the netns gate
by design — its whole purpose is to spawn subprocesses that talk to
the network. The same exclusion applies to `scan --adjudicate`.

## Implementation note (AppArmor)

The first netns implementation used the unprivileged `unshare -r
--net` form, but Ubuntu 24.04's AppArmor `unprivileged_userns`
profile blocks `/proc/self/uid_map` writes from non-root processes
on GitHub-hosted runners. The current job uses `sudo unshare --net`
instead — passwordless sudo is available on GHA runners, and
`--no-calibration` keeps the scan from needing `$HOME` access since
priors are embedded into the binary via `include_str!`. If GHA's
runner image ever loosens the AppArmor profile, the unprivileged
form is preferable for the smaller blast radius.

## What this guarantees, what it does not

The netns gate proves the default `scan` path opens no sockets. It
does not prove:

- That `scan --adjudicate` only opens sockets to the Anthropic API.
  Use a network proxy or audit `src/adjudicator.rs` directly if you
  need this property.
- That CLI subprocesses spawned by `cross-model-kappa` only talk to
  the LLM vendor's API. The audit explicitly delegates trust to the
  vendor CLI (`claude` / `gemini`).
- That tree-sitter parsers, the SARIF crate, or the standard
  library never attempt DNS or socket calls under exotic conditions.
  These would fail under netns even if they tried, so the gate
  catches them retroactively.

See also:

- [Four-layer architecture](./architecture.md) — where the Layer 3
  boundary sits.
- [`docs/spec/cross-model-kappa-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/cross-model-kappa-v0.md)
  — design rationale for the CLI-shellout shape.
- [`docs/spec/adjudicator-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/adjudicator-v0.md)
  — Layer 3 contract.
