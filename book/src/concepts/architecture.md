# Four-layer architecture

cntrdct is organised into four layers with a deliberate separation
between deterministic and stochastic surfaces. Each finding flows
through the layers in the same fixed order:

```text
source files
    -> Layer 1 (tree-sitter detectors)
    -> Layer 2 (statistical ranker)
    -> Layer 3 (optional LLM adjudicator; only when --adjudicate)
    -> Layer 4 (SARIF 2.1.0 emitter)
```

| Layer | Role | LLM? | Network? | Source |
|---|---|---|---|---|
| Layer 1 | Tree-sitter-based detectors | No | No | `src/detectors/` |
| Layer 2 | Statistical ranker (Wilson / Jeffreys × log-sibling-count) | No | No | `src/ranker.rs`, `src/calibration.rs` |
| Layer 3 | Optional LLM adjudicator | Yes | Yes (opt-in) | `src/adjudicator.rs` |
| Layer 4 | SARIF 2.1.0 emitter | No | No | `src/sarif.rs` |

## Layer 1 — deterministic detectors

Each detector under `src/detectors/` is a pure function from a
parsed tree-sitter syntax tree to a `Vec<Finding>`. Six detectors
ship in v0: `arg-swap`, `clone-drift`, `comment-code`,
`config-interaction`, `pr-miner`, `unreachable-after-terminator`.
The detector set is registered through `cntrdct::ALL_DETECTOR_IDS`,
which is the single source of truth that `tests/wiring_consistency.rs`
asserts against both `scan_full_with_config` and the SARIF
`tool.driver.rules` array. Adding or removing a detector at one site
without the other fails CI.

All detectors reach tree-sitter through the
`crate::parsers::parser_for(Language::*).ts_language()` seam (Q-10),
so adding a new language is a single-module change in
`src/parsers.rs` rather than a cross-detector edit.

## Layer 2 — statistical ranker

The ranker scores findings by a Wilson or Jeffreys lower bound on
detector precision multiplied by a log-scaled sibling-finding count.
Priors are fit by `cntrdct calibrate` against a labelled corpus
(`benchmarks/labelled-findings.jsonl`) and embedded into the binary
via `include_str!`. The runtime fallback chain is: explicit
`--priors` -> per-user cache -> embedded default -> uncalibrated
(see [Statistical priors (P4)](./priors.md)).

## Layer 3 — LLM adjudicator (opt-in)

Layer 3 is the only layer permitted to invoke an LLM. Three
providers ship behind a common `PromptDispatch` trait:

- `AnthropicAdjudicator` (HTTP via `reqwest`) — used by
  `scan --adjudicate`.
- `ClaudeCliAdjudicator` (CLI shellout to `claude --print`) — used
  by `cntrdct cross-model-kappa` (Q-13).
- `GeminiCliAdjudicator` (CLI shellout to `gemini -p`) — used by
  `cntrdct cross-model-kappa`.

The two CLI providers do not open sockets from cntrdct itself; the
underlying CLIs handle their own auth and HTTP. Verdict confidence
is post-hoc Platt-calibrated by the Q-12 helper
`apply_llm_calibration` when a fitted registry is present; v0 ships
the registry empty.

## Layer 4 — SARIF emitter

`src/sarif.rs` converts `Vec<Finding>` to SARIF 2.1.0 JSON validated
against the OASIS schema on every CI run. Severity maps to
IEEE 1044-2009 anomaly classes at emission time
(see [Severity and anomaly classes (P5)](./severity.md)).

## Why this split

The Layer 3 boundary is load-bearing: it is the only layer permitted
to open a socket, and only when `--adjudicate` is passed. The
default `scan` pipeline (Layers 1 -> 2 -> 4) is fully offline,
deterministic, and embeddable. The CI `network-isolation` job runs
`cntrdct scan` inside a Linux network namespace
(`sudo unshare --net`) on every push and PR; any unintended socket
open fails the job with `ENETUNREACH` / `EAI_*`. See
[Network access policy (P3)](./network.md) for the full enforcement
story.

The deterministic-by-default split is what lets cntrdct be a
reproducible artefact: a given source tree plus a given binary
always produces the same SARIF output, modulo the explicit
`--adjudicate` opt-in.

See also:

- [Citation policy (P1)](./citations.md)
- [Network access policy (P3)](./network.md)
- [Statistical priors (P4)](./priors.md)
- [Severity and anomaly classes (P5)](./severity.md)
- [`docs/spec/`](https://github.com/ktrysmt/cntrdct/tree/master/docs/spec)
  for per-layer contracts.
