# cntrdct.toml reference

A `cntrdct.toml` placed at the scan root tunes severity, thresholds,
per-path allow/deny rules, and per-language enablement.

## Severity remapping

```toml
[severity]
"clone-drift" = "Note"   # downgrade
"arg-swap"    = "Error"  # upgrade
```

## Per-path rules

```toml
[[paths]]
glob = "vendor/**"
action = "deny"          # never scan these files

[[paths]]
glob = "tests/**"
action = "allow"
detectors = ["unreachable-after-terminator"]  # only this detector
```

## Per-language enablement

```toml
[languages.rust]
enabled = true

[languages.python]
enabled = true
suppress = ["pr-miner"]  # disable this detector for Python only
```

The full schema lives in `src/config.rs`. The Python suppression
scanner recognises the same `cntrdct.toml` schema as the Rust path.

See also: [In-source suppressions](./suppressions.md).
