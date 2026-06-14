# Cross-model κ audit (Q-13)

Q-13 deliverable. Spec: [`docs/spec/cross-model-kappa-v0.md`](../../docs/spec/cross-model-kappa-v0.md).

## Layout

- `sample-corpus.jsonl` — example JSONL ranked-finding corpus you can
  feed to the audit subcommand to see the output shape. One row per
  finding; identical to what `cntrdct scan --format json` produces
  (the loader also accepts the JSON-array form directly).

The audit is on-demand only — there is no nightly cadence, so this
directory does not auto-fill with dated reports. Drop your own
`<name>.json` under here if you want to keep audits checked in for a
given branch / release; otherwise the default invocation prints the
JSON to stdout.

## Running the audit locally

Both CLIs must be installed and authenticated:

```sh
claude          # OAuth login for Claude Code (interactive)
agy             # OAuth login for Google Antigravity (interactive)
```

(`agy` is Antigravity's multi-model CLI; it replaced the retired
standalone `gemini` binary. A free / not-fully-logged-in Antigravity
account is aggressively rate-limited, so bursts of calls may hang /
throttle — log in (and/or use a paid tier) and space the calls for
clean κ runs. `AGY_CLI_MODEL_OVERRIDE` selects the model.)

Then:

```sh
# stdout (default)
cntrdct cross-model-kappa benchmarks/cross-model-kappa/sample-corpus.jsonl

# write to disk
cntrdct cross-model-kappa benchmarks/cross-model-kappa/sample-corpus.jsonl \
  --output benchmarks/cross-model-kappa/2026-05-11.json
```

cntrdct itself reads no API keys — auth is delegated to each CLI's
own login. A missing CLI (or one whose `--version` probe fails)
surfaces as a `skipped` provider record in the audit JSON; at least
two live providers are required to compute pairwise Cohen's κ.

The provider program names can be overridden for testing or for
running a wrapper:

```sh
CLAUDE_CLI_PROGRAM_OVERRIDE=/usr/local/bin/claude \
AGY_CLI_PROGRAM_OVERRIDE=/usr/local/bin/agy \
cntrdct cross-model-kappa <corpus.jsonl>
```

The `agy` model is overridable too (default `"Gemini 3.5 Flash (Low)"`):

```sh
AGY_CLI_MODEL_OVERRIDE="Gemini 3.5 Flash (Medium)" \
cntrdct cross-model-kappa <corpus.jsonl>
```

## Methodology recipe

The audit invokes each CLI with a flag set chosen to strip agentic
features so the model receives essentially the user prompt only:

- **Claude**: `--print --model claude-sonnet-4-6 --system-prompt "<minimal>"
  --tools "" --strict-mcp-config --disable-slash-commands
  --no-session-persistence --output-format json`. CLAUDE.md
  auto-discovery is suppressed by spawning the subprocess in a
  fresh tempdir.
- **Antigravity (`agy`)**: `agy --model "Gemini 3.5 Flash (Low)"
  --print "<forceful closed-book system prompt> <prompt>"`. Arg order is
  load-bearing: `--print` / `-p` takes the prompt as its VALUE, so
  `--model` must precede it and the prompt is the token right after
  `--print` (the wrong order makes `--print` swallow `--model`). `agy`
  has no `--output-format json` or `--system-prompt` flag, so the system
  prompt is folded into the (single-line, compact) prompt body and the
  provider parses the raw text response directly. The model is forced to
  a Gemini variant so the pairing with `claude-cli` stays cross-family.
  Project-context auto-discovery is suppressed the same way (subprocess
  `cwd = <tempdir>`).

Temperature is not directly exposed by either CLI's flag surface; Claude
Code uses an internal default and `agy` exposes no temperature knob in
print mode. v0 accepts this asymmetry since the audit is a snapshot,
not a precision instrument.

## Audit JSON schema

Top-level fields:

- `date` — UTC date of the audit run (`YYYY-MM-DD`).
- `generated_at` — ISO 8601 UTC timestamp of generation.
- `providers` — one record per declared provider:
  - `provider_id` — `"claude-cli"` or `"agy-cli"`.
  - `model` — model id passed to the CLI (`claude-sonnet-4-6`,
    `"Gemini 3.5 Flash (Low)"`).
  - `status.kind` — `"live"` / `"mocked"` / `"skipped"`.
  - `status.detail` — present only on `skipped`; explains why
    (`"<binary> CLI not available on PATH"` etc.).
- `cells` — per-`(detector_id, anomaly_class)` cell, sorted by key:
  - `n` — finding count in the cell.
  - `pairwise_kappa` — `{ "claude-cli-agy-cli": { "kappa": …, "degenerate": … } }`.
    With two providers there is exactly one pair per cell. Pair
    labels are alphabetised so the JSON diff stays line-oriented.
  - `min_kappa` — smallest non-degenerate κ across pairs.
  - `low_reliability` — `n >= 5` and `min_kappa < 0.6`
    (Landis & Koch substantial-agreement floor).
  - `low_n` — `n < 5`. Excluded from `low_reliability` flagging and
    from `worst_cell` selection.
- `worst_cell` — single worst non-degenerate κ across `n >= 5` cells,
  or absent.

## Citations

- `wataoka-2024` — K. Wataoka, T. Takahashi, R. Ri, "Self-Preference
  Bias in LLM-as-a-Judge", arXiv:2410.21819, 2024.
- `zheng-neurips-2023` — L. Zheng et al., "Judging LLM-as-a-Judge with
  MT-Bench and Chatbot Arena", NeurIPS 36, 46595–46623, 2023.

Both keys are surfaced through `ADJUDICATOR_CITATIONS` in
`src/adjudicator.rs`; the
`adjudicator_citations_match_layer3_section_exactly` test in
`tests/citations_consistency.rs` enforces parity with
[`CITATIONS.md`](../../CITATIONS.md).
