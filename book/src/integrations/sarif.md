# SARIF output

`cntrdct scan --format sarif` emits SARIF 2.1.0 compatible with
GitHub Code Scanning and any other OASIS-conformant consumer. Every
CI run validates the emission against the official OASIS schema via
`Sarif.Multitool validate` (`.github/workflows/ci.yml`).

## Mapping table

| cntrdct concept | SARIF location |
|---|---|
| `detector_id` | `result.ruleId` and `tool.driver.rules[].id` |
| `severity` | `result.level` (Error / Warning / Note → error / warning / note; Info → none — see [Severity](../concepts/severity.md)) |
| IEEE 1044-2009 anomaly class | `result.properties.anomalyClass` |
| `prior_method` | `result.properties.priorMethod` (`wilson` or `jeffreys`) |
| Citations | `result.relatedInformation[]` (one entry per citation key) |
| Adjudicator verdict | `result.properties.adjudication.*` |

The rules taxonomy under `tool.driver.rules[]` is pinned by
`tests/wiring_consistency.rs` against
`cntrdct::ALL_DETECTOR_IDS` — adding a detector without registering it
in both the construction path and the SARIF emitter fails CI.

Spec:
[`docs/spec/sarif-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/sarif-v0.md).
