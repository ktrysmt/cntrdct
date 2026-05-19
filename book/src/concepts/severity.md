# Severity and anomaly classes (P5)

Each finding carries an IEEE 1044-2009 anomaly classification and a
SARIF-compatible severity level. The mapping is fixed at SARIF
emission time and pinned by `tests/sarif_lib.rs`.

SARIF level mapping:

| Severity | SARIF level | Rationale |
|---|---|---|
| `Error` | `error` | Hard contradictions — false positives are rare by construction. |
| `Warning` | `warning` | Default for shipped detectors. |
| `Note` | `note` | Lower-confidence findings or downgrades. |
| `Info` | `none` | Reserved for user-authored severity overrides; signals "less visible than `Note`" to GitHub Code Scanning. The original severity is recoverable from `result.properties.raw`. |

The decision log for `Info → none` is captured in
[`docs/spec/sarif-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/sarif-v0.md)
F5.

Severity may be remapped per-detector or per-path via
`cntrdct.toml` — see [Configuration](../configuration/cntrdct-toml.md).
