# cntrdct

[![CI](https://github.com/ktrysmt/cntrdct/actions/workflows/ci.yml/badge.svg)](https://github.com/ktrysmt/cntrdct/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/cntrdct-cli.svg)](https://crates.io/crates/cntrdct-cli)
[![docs.rs](https://img.shields.io/docsrs/cntrdct-core)](https://docs.rs/cntrdct-core)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Evidence-based linter for logical contradictions and technical
inconsistencies in Rust code. Every finding cites the peer-reviewed
paper or established benchmark that justifies the detection — detectors
without citations are rejected at startup. The pipeline runs entirely
offline by default; an optional Layer 3 LLM adjudicator can be enabled
explicitly per-scan.

Status: alpha. CLI and five Layer 1 detectors are working; the full
Layer 1 → 4 pipeline (detect → rank → adjudicate → SARIF) runs end-to-end.

## Quickstart

```sh
# Pre-built binary (Linux x86_64/aarch64, macOS x86_64/aarch64).
curl -fsSL https://raw.githubusercontent.com/ktrysmt/cntrdct/main/scripts/install.sh | bash

# Or, from source.
git clone https://github.com/ktrysmt/cntrdct.git
cd cntrdct
cargo install --path crates/cli

# Scan any Rust path. Default output is JSON to stdout.
cntrdct scan ./src

# Or invoke through cargo.
cargo cntrdct scan ./src

# SARIF 2.1.0 output, ready for GitHub code scanning or any SARIF viewer.
cntrdct scan ./src --format sarif > findings.sarif
```

Three runnable end-to-end examples (scan, calibrate, adjudicate-with-mock)
live under [`examples/`](examples/).

## Design constraints

The project ships under five hard constraints. Every change is reviewed
against them; violations are rejected at startup, in tests, or in code
review.

- P1 — every detector cites peer-reviewed prior art (`register_detector`
  rejects detectors with empty `citations()`; CI checks that every key
  resolves to an entry in `CITATIONS.md`).
- P2 — empirical results carry a preregistration id
  (`DetectorConfig::preregistration_id`).
- P3 — only the Layer 3 adjudicator may invoke an LLM. Layers 1, 2, 4
  are deterministic.
- P4 — statistical priors come from labelled corpora, not from prompts
  or hardcoded constants. They live in the ranker, not the adjudicator.
- P5 — severities map to IEEE 1044-2009 anomaly classes at SARIF
  emission time.

## Architecture (4 layers)

1. Deterministic detectors (Layer 1) — tree-sitter based, no LLM.
2. Statistical false-positive filter (Layer 2) — Wilson lower bound,
   Z-Ranking on labelled corpora; uncalibrated sibling-count fallback
   when no corpus is loaded.
3. LLM adjudicator (Layer 3) — sole layer permitted to invoke an LLM
   (Anthropic Messages API). Opt-in via `--adjudicate`.
4. SARIF 2.1.0 output (Layer 4) — IEEE 1044-2009 compatible severity
   and anomaly classification.

## Detectors shipped

| id | what it flags | citations |
|---|---|---|
| `clone-drift` | a near-duplicate function whose AST diverged from a majority of its siblings | Cordy & Roy (ICPC 2008), Bettenburg et al. (MSR 2009), Krinke (ICSM 2007) |
| `arg-swap` | a 2-arg call whose argument names are swapped relative to the same-file definition | Li & Zhou (ESEC/FSE 2005), Rice et al. (ICSE 2017) |
| `comment-code` | doc comment claims a behaviour the implementation does not exhibit (Result/panic/deprecated patterns) | Tan et al. (SOSP 2007, PLDI 2011) |
| `unreachable-after-terminator` | a statement following `return` / `panic!()` / `unreachable!()` / `todo!()` / `break` / `continue` within the same block | Hovemeyer & Pugh (OOPSLA 2004), Engler et al. (SOSP 2001) |
| `config-interaction` | a top-level item bears two `#[cfg(...)]` attributes whose predicates are structurally negations of each other (item dead under any configuration) | Tartler et al. (EuroSys 2011), Nadi et al. (ICSE 2014) |

See `CITATIONS.md` for the full bibliography and `docs/spec/` for
per-detector specifications.

## Usage

Scan a path:

```sh
cntrdct scan ./src                    # JSON output to stdout
cntrdct scan ./src --format sarif     # SARIF 2.1.0
cntrdct scan ./src --adjudicate       # adds Layer 3 verdicts on top-N findings
                                      # requires ANTHROPIC_API_KEY
```

Build calibration priors from a labelled JSONL corpus:

```sh
cntrdct calibrate corpus.jsonl                       # writes default cache path
cntrdct calibrate corpus.jsonl --output priors.json
```

Evaluate detectors against a labelled corpus and print a
precision/recall/F1 report:

```sh
cntrdct eval benchmarks/corpus
```

The seed corpus under `benchmarks/corpus/` covers the five shipped
detectors with a handful of positive and negative cases. See
`benchmarks/README.md` for the manifest format and how to add cases.

When a priors file is present (default cache or `--priors`), the
calibrated ranker uses Wilson lower bound × log-scaled sibling count.
Without one, `cntrdct scan` falls back to sibling-count ordering.

## Claude Code skill

`.claude/skills/cntrdct/SKILL.md` ships a thin entry-point wrapper.
With the binary on PATH, Claude Code users can invoke `/cntrdct` to run
a scan and have the top findings summarised in chat. The skill performs
no detection itself (P3); it only orchestrates the binary and renders
results.

## Workspace layout

| crate | role |
|---|---|
| `core` | shared traits (`Detector`, `Ranker`, `Adjudicator`), `Finding`/`RankedFinding` types, P1 enforcement |
| `detector-clone-drift` | Layer 1 — Type-3 near-miss clone drift |
| `detector-arg-swap` | Layer 1 — argument-order defects |
| `detector-comment-code` | Layer 1 — doc/code mismatch |
| `detector-unreachable-after-terminator` | Layer 1 — unreachable code after divergent terminator |
| `detector-config-interaction` | Layer 1 — contradictory cfg attribute pair on a single item |
| `ranker` | Layer 2 — `UncalibratedRanker`, `CalibratedRanker` |
| `calibration` | Layer 2 data layer — corpus loader, Wilson bound, Laplace posterior |
| `adjudicator-llm` | Layer 3 — Anthropic Messages adjudicator with `HttpClient` seam |
| `sarif` | Layer 4 — SARIF 2.1.0 emitter |
| `eval` | precision/recall/F1 evaluation harness against a labelled corpus |
| `cli` | binary + library glue |

## Design notes

`docs/spec/` contains the active specs that drove the TDD
implementation. `ROADMAP.md` tracks engineering deliverables; the
academic research tracks live under `projects/`.

## License

MIT. See [LICENSE](LICENSE).
