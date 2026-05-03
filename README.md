# cntrdct

Evidence-based linter for logical contradictions and technical inconsistencies.

Status: pre-alpha. Trait scaffolding only; no working binary yet.

## Why another linter

Existing AI-driven review tools detect issues without grounding them in peer-reviewed
prior art. cntrdct enforces the inverse: every detector must reference at least one
published paper or established benchmark. Findings without citations cannot ship.

## Architecture (4 layers)

1. Deterministic detectors (Layer 1) — tree-sitter based, no LLM.
2. Statistical false-positive filter (Layer 2) — Z-Ranking, Wilson lower bound.
3. LLM adjudicator (Layer 3) — sole layer permitted to invoke an LLM.
4. SARIF 2.1.0 output (Layer 4) — IEEE 1044-2009 compatible severity.

## Design notes

See `../results/00023_エビデンスベース不整合検出リンタ設計/` for the full design log.

## License

MIT.
