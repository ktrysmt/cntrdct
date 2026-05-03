# cntrdct

Evidence-based linter for logical contradictions and technical inconsistencies.

Status: alpha. CLI and four Layer 1 detectors are working; the full
Layer 1 → 4 pipeline (detect → rank → adjudicate → SARIF) runs end-to-end.

## Why another linter

Existing AI-driven review tools detect issues without grounding them in
peer-reviewed prior art. cntrdct enforces the inverse: every detector must
reference at least one published paper or established benchmark
(constraint P1). Findings without citations cannot ship — `register_detector`
rejects them at startup, and a workspace-wide test enforces that every
declared citation key appears in `CITATIONS.md`.

## Architecture (4 layers)

1. Deterministic detectors (Layer 1) — tree-sitter based, no LLM.
2. Statistical false-positive filter (Layer 2) — Wilson lower bound, Z-Ranking
   on labelled corpora; uncalibrated sibling-count fallback when no corpus
   is loaded.
3. LLM adjudicator (Layer 3) — sole layer permitted to invoke an LLM
   (Anthropic Messages API). Opt-in via `--adjudicate`.
4. SARIF 2.1.0 output (Layer 4) — IEEE 1044-2009 compatible severity and
   anomaly classification.

## Detectors shipped

| id | what it flags | citations |
|---|---|---|
| `clone-drift` | a near-duplicate function whose AST diverged from a majority of its siblings | Cordy & Roy (ICPC 2008), Bettenburg et al. (MSR 2009), Krinke (ICSM 2007) |
| `arg-swap` | a 2-arg call whose argument names are swapped relative to the same-file definition | Li & Zhou (ESEC/FSE 2005), Rice et al. (ICSE 2017) |
| `comment-code` | doc comment claims a behaviour the implementation does not exhibit (Result/panic/deprecated patterns) | Tan et al. (SOSP 2007, PLDI 2011) |
| `unreachable-after-terminator` | a statement following `return` / `panic!()` / `unreachable!()` / `todo!()` / `break` / `continue` within the same block | Hovemeyer & Pugh (OOPSLA 2004), Engler et al. (SOSP 2001) |
| `config-interaction` | a top-level item bears two `#[cfg(...)]` attributes whose predicates are structurally negations of each other (item dead under any configuration) | Tartler et al. (EuroSys 2011), Nadi et al. (ICSE 2014) |

See `CITATIONS.md` for the full bibliography and `docs/spec/` for per-detector
specifications.

## Usage

Install from source (no published release yet):

```
cargo install --path crates/cli
```

Scan a path:

```
cntrdct scan ./src                    # JSON output to stdout
cntrdct scan ./src --format sarif     # SARIF 2.1.0
cntrdct scan ./src --adjudicate       # adds Layer 3 verdicts on top-N findings
                                      # requires ANTHROPIC_API_KEY
```

Build calibration priors from a labelled JSONL corpus:

```
cntrdct calibrate corpus.jsonl                       # writes default cache path
cntrdct calibrate corpus.jsonl --output priors.json
```

Evaluate detectors against a labelled corpus and print a precision/recall/F1
report:

```
cntrdct eval benchmarks/corpus
```

The seed corpus under `benchmarks/corpus/` covers the four shipped detectors
with a handful of positive and negative cases. See `benchmarks/README.md` for
the manifest format and how to add cases.

When a priors file is present (default cache or `--priors`), the calibrated
ranker uses Wilson lower bound × log-scaled sibling count. Without one,
`cntrdct scan` falls back to sibling-count ordering.

## Claude Code skill

`.claude/skills/cntrdct/SKILL.md` ships a thin entry-point wrapper. With the
binary on PATH, Claude Code users can invoke `/cntrdct` to run a scan and have
the top findings summarised in chat. The skill performs no detection itself
(P3); it only orchestrates the binary and renders results.

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

`docs/spec/` contains the active specs that drove the TDD implementation.
The full design log lives at
`../knowledge/infra/results/00023_エビデンスベース不整合検出リンタ設計/`.

## License

MIT.
