# Severity and anomaly classes (P5)

Every finding carries two orthogonal labels:

- An IEEE 1044-2009 anomaly classification (`AnomalyClass`),
  describing the kind of defect.
- A SARIF-compatible severity level (`Severity`), describing the
  attention the finding warrants.

Both are pinned at SARIF emission time and asserted by
`tests/sarif_lib.rs`. The mapping is the P5 design constraint.

## IEEE 1044-2009 anomaly classes

`Finding.anomaly_class` is one of seven values defined in
`src/core.rs`:

| Class | Meaning | Used by |
|---|---|---|
| `Logic` | Behavioural contradiction in control or data flow. | `clone-drift`, `config-interaction`, `pr-miner`, `unreachable-after-terminator` |
| `Interface` | Caller-callee mismatch in arguments or signatures. | `arg-swap` |
| `Data` | Inconsistency in stored or transferred data. | — |
| `Documentation` | Contradiction between comment and code. | `comment-code` |
| `Performance` | Resource-use issue. | — |
| `Standards` | Coding-standard violation. | — |
| `Other` | Defects not covered above. | — |

The class is fixed per detector — clone-drift is always `Logic`,
arg-swap is always `Interface`, and so on. Future detectors may
populate the unused rows.

## SARIF level mapping

Each finding's `Severity` maps to a SARIF 2.1.0 `level` value at
emission time:

| `Severity` | SARIF `level` | Rationale |
|---|---|---|
| `Error` | `error` | Hard contradictions; false positives are rare by construction. |
| `Warning` | `warning` | Default for shipped detectors. |
| `Note` | `note` | Lower-confidence findings or downgrades. |
| `Info` | `none` | User-authored downgrade; signals "less visible than `Note`" to GitHub Code Scanning. |

The mapping is one-way at the SARIF surface, but the original
`Finding.raw_severity` is preserved in `result.properties.raw` so
downstream consumers that branch on the four-valued vocabulary can
recover it.

## Info -> none decision log

`Severity::Info` maps to SARIF `"none"`, not `"note"`. The rationale,
recorded in [`docs/spec/sarif-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/sarif-v0.md)
F5:

- No shipped detector emits `Severity::Info` by construction. The
  variant enters the pipeline only through user-authored
  `cntrdct.toml` severity overrides. A user who explicitly
  downgrades a finding to `Info` is signalling "I want this less
  visible than `Note`."
- SARIF 2.1.0 §3.27.10 defines `"none"` as "the level is not
  applicable to the result," and GitHub Code Scanning suppresses
  `none`-level results from the inline PR review surface. That is
  exactly what an `Info`-tagged finding should do.
- The alternative remap (`Info -> "note"`) would conflate
  `Severity::Info` and `Severity::Note` in SARIF output, defeating
  the user's intent.

The trade-off is explicit: GitHub Code Scanning hides `Info`
findings from the inline PR review surface. This is intentional and
documented so a future SARIF consumer change does not silently
reopen the question.

## Severity remapping via cntrdct.toml

Severity can be overridden per-detector or per-path in
`cntrdct.toml`:

```toml
[severity]
"clone-drift" = "note"      # global downgrade

[[severity.path]]
glob = "tests/**"
detector = "clone-drift"
level = "info"              # tests-only downgrade
```

The override is applied before SARIF emission, so the SARIF `level`
reflects the remapped value. The original detector-side severity is
still available in `result.properties.raw`.

See [Configuration: cntrdct.toml reference](../configuration/cntrdct-toml.md)
for the full schema.

## What is pinned where

| File | Pins |
|---|---|
| `src/core.rs` | `AnomalyClass` enum and serde projection. |
| `src/sarif.rs` | The `Severity` -> SARIF `level` mapping. |
| `tests/sarif_lib.rs` | Assertions that the mapping holds end-to-end. |
| `tests/multilang_config.rs` | Assertions that `cntrdct::ALL_DETECTOR_IDS` is fully present in `tool.driver.rules`. |
| `.github/workflows/ci.yml` | `Sarif.Multitool validate` against the OASIS 2.1.0 schema on every CI run. |

See also:

- [`docs/spec/sarif-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/sarif-v0.md)
  — SARIF emitter contract and F5 decision log.
- [Integrations: SARIF output](../integrations/sarif.md)
  — consumer-side notes on the emitted document.
