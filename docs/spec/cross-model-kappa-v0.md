# cntrdct cross-model κ audit v0 spec

Status: active draft, approved for TDD implementation 2026-05-11.

Q-13 deliverable from `ROADMAP.md`. Surfaces self-preference bias in
the Layer 3 LLM adjudicator by routing the same `RankedFinding` set
through two installed CLI judges (Claude Code's `claude --print` and
the Gemini CLI's `gemini -p`) and computing pairwise Cohen's κ on the
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

3. **Nightly continuous monitoring**. The original ROADMAP wording
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

The shipped design — Claude + Gemini CLI shellout, on-demand
subcommand — is the smallest implementation that lets cntrdct surface
cross-model agreement at audit time without taking on three-API-key
setup or false-precision time-series infrastructure.

## Scope

In scope:

- A new `PromptDispatch` trait shared with the existing
  `AnthropicAdjudicator` (HTTP, used by `scan --adjudicate`).
- Two CLI-shellout providers under `src/adjudicator.rs`:
  - `ClaudeCliAdjudicator` — invokes `claude --print` with the
    methodology-clean flag set documented below.
  - `GeminiCliAdjudicator` — invokes `gemini -p` with
    `GEMINI_SYSTEM_MD` env var pointing at a temp file.
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

### F3 — `GeminiCliAdjudicator`

Invokes `gemini -p` with the system prompt pre-written to a temp
file, then exposed via `GEMINI_SYSTEM_MD`:

```
[env: GEMINI_SYSTEM_MD=<temp file path>]
gemini
  -p <prompt>
  -m <model>                # default gemini-2.5-flash
  --output-format json
```

The process is spawned with `current_dir = <tempdir>` so GEMINI.md
hierarchical context auto-discovery picks up no project context.

Temperature pinning (Gemini's `modelConfigs.customAliases.audit-flash`
in settings.json with `generateContentConfig.temperature = 0.0`) is
documented in the audit README but not enforced by the provider; the
flag-level surface is asymmetric across the two CLIs and v0 accepts
that.

#### Output parsing

`gemini --output-format json` emits `{"response": "<text>", "stats": {...}}`.
The provider extracts `response`, then runs `parse_inner_text` on it.

#### Auth

Gemini CLI uses OAuth by default. Same Skipped-on-auth-failure
contract as Claude.

### F4 — Cohen's κ helper

Unchanged from the prior design. Pure function over two slices of
three-class verdicts, returns `Option<f64>`, `None` on degenerate
single-class collapse. Already implemented and tested.

### F5 — Per-cell aggregation

Unchanged surface (`AuditCell`, `KappaEntry`, `WorstCell`,
`AuditCellSummary`). With two providers, every cell has exactly one
pairwise κ entry (`"claude-cli-gemini-cli"`). `min_kappa` equals
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
(`worst cell: clone-drift:Logic pair=claude-cli-gemini-cli κ=0.42`).

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
