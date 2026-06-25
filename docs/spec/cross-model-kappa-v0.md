# cntrdct cross-model κ audit v0 spec

Status: active draft, approved for TDD implementation 2026-05-11.

Q-13 deliverable (from the since-retired rebuild plan). Surfaces self-preference bias in
the Layer 3 LLM adjudicator by routing the same `RankedFinding` set
through two installed CLI judges (Claude Code's `claude --print` and
Google Antigravity's `agy -p`, running a non-Anthropic Gemini model —
this replaces the retired standalone `gemini` CLI, which was folded
into Antigravity upstream) and computing pairwise Cohen's κ on the
resulting verdicts per `(detector_id, anomaly_class)` cell. Cells with
κ below the Landis & Koch (1977) substantial-agreement floor (κ < 0.6)
are flagged as low-reliability adjudication regions.

## Design rationale (why CLI shellout, why on-demand)

Two earlier designs were considered and dropped:

1. **Three-provider HTTP audit (Anthropic + OpenAI + Gemini APIs)**.
   Methodologically cleanest — all three providers receive the same
   raw user prompt at temperature 0.0, no system-prompt residue, no
   tool use. Dropped because the user-facing setup (three separate
   API keys, separate billing) was disproportionate to the value of
   continuous monitoring (see point 3 below).

2. **Three CLI judges (Claude + Codex + Gemini)**. Symmetric on
   subscription billing, but `codex exec` does not expose a
   system-prompt override flag (only `developer_instructions` which
   is additive, not replacing). Codex's residual system prompt is a
   confounding variable in self-preference bias measurement, so
   keeping Codex specifically would produce a κ figure that mixes
   "model agreement" with "agent-CLI persona difference".

3. **Nightly continuous monitoring**. The original plan wording
   called for a CI cron job emitting a dated audit log every 24h.
   Two practical realities make this low-value: (a) commercial LLMs
   are tuned and version-bumped continuously, so a κ time-series
   captures noise from upstream model drift more than it captures
   any property of the cntrdct adjudicator; (b) even at temperature
   0.0, sampler stochasticity and provider-side rounding produce
   verdict variance that swamps the bias signal at small N. The
   continuous tracking premise was not supported by the
   measurement's stationarity, so v0 ships an on-demand snapshot
   instead.

The shipped design — Claude + Antigravity (`agy`) CLI shellout,
on-demand subcommand — is the smallest implementation that lets cntrdct surface
cross-model agreement at audit time without taking on three-API-key
setup or false-precision time-series infrastructure.

## Scope

In scope:

- A new `PromptDispatch` trait shared with the existing
  `AnthropicAdjudicator` (HTTP, used by `scan --adjudicate`).
- Two CLI-shellout providers under `src/adjudicator.rs`:
  - `ClaudeCliAdjudicator` — invokes `claude --print` with the
    methodology-clean flag set documented below.
  - `AgyCliAdjudicator` — invokes `agy -p` (Antigravity). `agy` has no
    `--output-format json` or `--system-prompt` flag, so it parses the
    raw text response and folds the system prompt into the prompt body
    (see F3).
- Module `src/cross_model_kappa.rs` with the pure `cohen_kappa`
  helper and per-cell aggregation. Two-family operation by design;
  one κ per cell.
- A new `cntrdct cross-model-kappa <CORPUS>` subcommand. Default
  output is pretty JSON to stdout; `--output <PATH>` writes to disk.
- Deterministic mock fixtures in `tests/cross_model_kappa.rs` so PR
  CI can exercise the κ aggregation without invoking real CLIs.
- Citations `wataoka-2024` and `zheng-neurips-2023` in Layer 3.

Out of scope (v0):

- Anthropic / OpenAI / Google API-key paths (replaced by CLI
  shellout). The existing `AnthropicAdjudicator` HTTP path stays
  for `scan --adjudicate`; it is not used by Q-13.
- Codex CLI integration (see rationale section).
- Nightly CI workflow (see rationale section).
- Three-or-more provider audits. The aggregation generalises to N
  providers but v0 ships exactly two.
- README badge linking to a "latest" audit log. Dropped along with
  the nightly workflow.
- Recall calibration via cross-model agreement. Q-14 territory.

## Functional requirements

### F1 — `PromptDispatch` trait

```rust
pub trait PromptDispatch: Send + Sync {
    fn provider_id(&self) -> &'static str;
    fn model(&self) -> &str;
    fn dispatch(&self, prompt: &str) -> Result<AdjudicationResult, DetectorError>;
}
```

Object-safe so cross-model audits hold heterogeneous providers in
`Vec<Box<dyn PromptDispatch>>`. The existing `AnthropicAdjudicator`
implements both this trait and the legacy `Adjudicator` trait; the
new CLI providers implement only `PromptDispatch` (Q-13's audit is
the only consumer).

### F2 — `ClaudeCliAdjudicator`

Invokes `claude --print` with these flags, in order:

```
--print
--model <model>                            # default claude-sonnet-4-6
--system-prompt "<minimal system prompt>"  # full override of Claude Code's persona
--tools ""                                  # disable every built-in tool
--strict-mcp-config                        # ignore all MCP configurations
--disable-slash-commands                   # skip skills
--no-session-persistence                   # no rollout file written
--output-format json                       # parseable wrapper envelope
[<prompt>]                                  # adjudication prompt as positional arg
```

The process is spawned with `current_dir = <tempdir>` so Claude
Code's CLAUDE.md auto-discovery picks up no project context.

The minimal system prompt is hard-coded:
`"You are evaluating a static analysis finding from cntrdct. Respond
only with the requested JSON object."`

#### Output parsing

`claude --output-format json` emits a single JSON object whose
`result` field is the model's text response (which is itself the
verdict JSON we asked for). The provider extracts `result`, then
runs the existing `parse_inner_text` helper to strip optional
markdown fences and parse the inner verdict envelope.

#### Auth

Claude Code's CLI uses OAuth (subscription) by default. No env vars
required. If the user is not logged in, `claude --print` prints an
auth prompt to stderr and exits non-zero; the provider surfaces this
as a `DetectorError::Config` and the orchestrator records the
provider as `Skipped` rather than failing the whole audit.

### F3 — `AgyCliAdjudicator`

Invokes Google Antigravity's `agy` (the multi-model CLI that replaced
the retired standalone `gemini` binary):

```
agy
  --model <model>           # default "Gemini 3.5 Flash (Low)" (forced Gemini); MUST precede --print
  --print <prompt>          # `--print`/`-p` takes the prompt as its VALUE, not positionally
```

LOAD-BEARING arg order: `agy`'s `--print` / `-p` is NOT a boolean flag —
it takes the prompt as its value. `--model` must therefore come BEFORE
`--print`, and the prompt is the token immediately after `--print`.
The wrong order (`agy --print --model <m> <prompt>`) makes `--print`
swallow the literal `"--model"` as the prompt and drops the real prompt
as a stray positional; agy then replies chattily or emptily. Pinned by a
regression assertion in the `adjudicator.rs` agy dispatch test.

The process is spawned with `current_dir = <tempdir>` so Antigravity's
project-context auto-discovery (AGENTS.md / memory) picks up nothing.

`agy` exposes a smaller flag surface than `claude` / the retired
`gemini`: there is no `--output-format json` and no `--system-prompt`,
AND it runs a stickier agentic persona. Consequences the provider
absorbs:

- No system-prompt flag → a FORCEFUL closed-book directive
  (`AGY_SYSTEM_PROMPT`, not the weaker `CLI_SYSTEM_PROMPT`) is prepended
  to the prompt body. agy otherwise treats a "review this finding"
  prompt as an agentic task and hangs / returns empty in `-p` mode.
- Prompt shape → the `Adjudicator` path sends a COMPACT, single-line,
  plain-text-evidence prompt (`build_compact_prompt`), NOT the verbose
  `build_prompt` template. The labelled multi-field layout + nested
  `EVIDENCE_RAW` JSON block trips the same agentic path. Evidence is
  rendered as flat `k=v` pairs with no `{...}` braces, dropping the
  proposer's own `llm_rationale` / `llm_confidence` (which would bloat
  the prompt and bias the judge toward the proposal).
- Multi-model → the model is FORCED to a Gemini variant via `--model`
  (default `AGY_CLI_MODEL = "Gemini 3.5 Flash (Low)"`, overridable via
  `AGY_CLI_MODEL_OVERRIDE`). This keeps the provider non-Anthropic so
  the `claude-cli` + `agy-cli` pairing is genuinely cross-family, and
  so `candidate_llm::model_family` (which keys on the model string)
  classifies it as `google`. Overriding to a Claude model would
  collapse the pairing back to same-family.

Temperature is not pinned at the flag level (no exposed knob); the
flag-level surface is asymmetric across the two CLIs and v0 accepts
that.

#### Output parsing

Unlike `claude` / `gemini`, `agy --print` prints the model's text
response directly with no outer JSON envelope. That text IS the verdict
envelope (after markdown-fence stripping), so `parse_agy_cli_envelope`
hands the raw stdout straight to `parse_inner_text`.

Operational note (free-tier reliability): once the arg order / prompt
shape above are correct, `agy` returns the verdict envelope (verified:
`Gemini 3.5 Flash (Low)` confirmed a Bound-B swap, conf 0.95). The
residual constraint is the account: a free / not-fully-logged-in
Antigravity account (`agy` cli.log: `not logged into Antigravity`) is
aggressively rate-limited, so bursts of calls hang / throttle. Space the
calls or use a logged-in / paid tier; a non-JSON / empty response under
throttle still parses as an error and is treated as a dropped verdict
(the provider's Skipped/degrade contract) — not a cntrdct bug.

#### Auth

`agy` uses the Antigravity login (OAuth / subscription) by default.
Same Skipped-on-auth-failure contract as Claude.

### F4 — Cohen's κ helper

Unchanged from the prior design. Pure function over two slices of
three-class verdicts, returns `Option<f64>`, `None` on degenerate
single-class collapse. Already implemented and tested.

### F5 — Per-cell aggregation

Unchanged surface (`AuditCell`, `KappaEntry`, `WorstCell`,
`AuditCellSummary`). With two providers, every cell has exactly one
pairwise κ entry (`"claude-cli-agy-cli"`). `min_kappa` equals
that single value; `low_reliability` triggers when it is below 0.6
and the cell is not `low_n`.

### F6 — `AuditReport`

Unchanged shape. `providers` carries two records when both CLIs are
available, one when one is missing, zero (after which the audit
errors with `InsufficientProviders`) when neither is.

The `date` and `generated_at` fields are kept for human readability
but are no longer load-bearing — there is no nightly cadence
behind them.

### F7 — CLI: `cntrdct cross-model-kappa`

```sh
cntrdct cross-model-kappa <CORPUS> [--output PATH]
```

Default output is pretty JSON to stdout. `--output PATH` writes the
audit JSON to disk and prints a one-line stderr summary
(`worst cell: clone-drift:Logic pair=claude-cli-agy-cli κ=0.42`).

The earlier `benchmarks/cross-model-kappa/<UTC date>.json` default
path is dropped with the nightly workflow; ad-hoc users default to
stdout (which composes cleanly with `> file.json` if they want the
old behaviour).

### F8 — Citations

Layer 3 of `CITATIONS.md` carries:

- `wataoka-2024` — empirical evidence of self-preference bias.
- `zheng-neurips-2023` — methodological grounding for LLM-as-judge.

Both keys land in `ADJUDICATOR_CITATIONS` in `src/adjudicator.rs`
alongside the existing Q-12 keys; the
`adjudicator_citations_match_layer3_section_exactly` test enforces
parity.

### F9 — PR-CI mock test

`tests/cross_model_kappa.rs` exercises the κ aggregation through
the existing `CannedDispatch` (in-process `PromptDispatch` impl).
Adding stub-script integration tests for the actual CLI shellout
parsing surface is desirable but not v0-blocking; the wire-format
parser (`parse_inner_text`) is unit-tested in
`src/adjudicator.rs::tests`.

## Non-functional requirements

- N1. P3 unchanged. The CLI providers use `std::process::Command`,
  not `reqwest`. Adding them does not introduce a new HTTP path on
  any non-`AnthropicAdjudicator` route. The `network-isolation` CI
  job's invariant ("`scan` / `calibrate` / `eval` are network-free")
  stays true; the cross-model-kappa subcommand is excluded by
  design (it spawns subprocesses that themselves talk to the
  network, same shape as `scan --adjudicate`'s opt-in HTTP).
- N2. Determinism. The κ math (`cohen_kappa`, the per-cell
  aggregator) is deterministic. The CLI providers themselves are
  not — the underlying models are stochastic and the provider
  software is version-bumped silently. This non-determinism is the
  fundamental reason continuous monitoring was dropped (see design
  rationale point 3).
- N3. No API key leakage. CLIs handle their own auth. The provider
  implementations never hold or forward API keys.
- N4. Audit-log byte stability across runs. NOT promised v0. The
  inputs (CLI provider outputs) are non-deterministic; the
  aggregation is byte-stable for fixed inputs but the upstream
  inputs aren't. Tests that need byte-stability use
  `CannedDispatch` directly.

## References

- `wataoka-2024` — K. Wataoka, T. Takahashi, R. Ri, "Self-Preference
  Bias in LLM-as-a-Judge", arXiv:2410.21819, 2024.
- `zheng-neurips-2023` — L. Zheng et al., "Judging LLM-as-a-Judge
  with MT-Bench and Chatbot Arena", NeurIPS 36, 46595–46623, 2023.
- Cohen, J. (1960), "A coefficient of agreement for nominal
  scales", Educational and Psychological Measurement 20(1), 37–46.
- Landis, J.R. & Koch, G.G. (1977), "The measurement of observer
  agreement for categorical data", Biometrics 33(1), 159–174.
- Spiess et al. (2025), "Calibration and Correctness of Language
  Models for Code", ICSE 2025 — already cited under Q-12; the
  agent-CLI residual system-prompt observation in §6 is the
  mechanism by which Codex was excluded from the v0 design.
