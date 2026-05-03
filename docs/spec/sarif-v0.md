# cntrdct SARIF emitter v0 spec

Status: active draft, approved for TDD implementation 2026-05-03.

## Scope

Convert `Vec<Finding>` to SARIF 2.1.0 JSON. Library only; no binary, no I/O.

## Functional requirements

### F1 — API

- `cntrdct_sarif::to_sarif(findings: &[Finding]) -> serde_json::Value`
- `cntrdct_sarif::to_sarif_pretty(findings: &[Finding]) -> String`

### F2 — Top-level structure

```
{
  "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
  "version": "2.1.0",
  "runs": [{ "tool": {...}, "results": [...] }]
}
```

### F3 — Tool driver

```
"tool": {
  "driver": {
    "name": "cntrdct",
    "version": <CARGO_PKG_VERSION of the sarif crate>,
    "informationUri": "https://github.com/TBD/cntrdct"
  }
}
```

For v0 the `rules` array is omitted; results carry `ruleId` directly.

### F4 — Result mapping

Each `Finding` produces one SARIF `result`:

| Finding field | SARIF field |
|---|---|
| `detector_id` | `result.ruleId` |
| `raw_severity` | `result.level` (per F5 mapping) |
| `message` | `result.message.text` |
| `primary` | `result.locations[0].physicalLocation` |
| `related` | `result.relatedLocations[*].physicalLocation` |
| `evidence.citation_keys` | `result.properties.citationKeys` (array of strings) |
| `anomaly_class` | `result.properties.anomalyClass` (IEEE 1044-2009 class as string) |
| `evidence.raw` | `result.properties.raw` (passthrough) |

### F5 — Severity mapping (IEEE 1044-2009 informed)

| `Severity` | SARIF `level` |
|---|---|
| `Info` | `"none"` |
| `Note` | `"note"` |
| `Warning` | `"warning"` |
| `Error` | `"error"` |

### F6 — Location mapping

Each `Location` becomes:
```
{
  "physicalLocation": {
    "artifactLocation": { "uri": <file path as UTF-8 string> },
    "region": {
      "startLine": <u32>,
      "startColumn": <u32>,
      "endLine": <u32>,
      "endColumn": <u32>
    }
  }
}
```

Path is stringified via `to_string_lossy`. Non-UTF-8 paths are mangled but never panic.

### F7 — Determinism

`to_sarif` is a pure function of its input; identical input yields identical output.
Field order in JSON objects follows insertion order from `serde_json::json!`.

## Non-functional requirements

- N1. No I/O, no panics on valid Finding input
- N2. No network, no LLM
- N3. Output is valid JSON parseable by `serde_json::from_str`

## Test plan

| ID | Description | Expected |
|---|---|---|
| T1 | empty findings | minimal SARIF, `runs[0].results = []`, `version = "2.1.0"` |
| T2 | one Warning Finding | `result.ruleId`, `result.level = "warning"`, `result.message.text` set |
| T3 | severity round-trip | each `Severity` maps to expected SARIF level |
| T4 | $schema field present | `sarif["$schema"]` is a string |
| T5 | location fields | `primary` and `related` correctly populate `locations` and `relatedLocations` |
| T6 | pretty form is valid JSON | `to_sarif_pretty` output round-trips through `serde_json::from_str` |
| T7 | citationKeys propagate | every finding's `citation_keys` appear under `result.properties.citationKeys` |

## Non-goals (v0)

- Rules array under `tool.driver.rules`
- Help URIs per rule
- `result.kind`, `result.rank`, `result.fingerprints`
- Suppressions, taxonomies, conversion records
