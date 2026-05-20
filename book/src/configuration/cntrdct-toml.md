# cntrdct.toml reference

A `cntrdct.toml` placed at the scan root tunes per-detector behaviour,
filters by path, and controls per-language enablement. The file is
optional — a missing `cntrdct.toml` is silently treated as the empty
config (every detector on, every language on, no suppression). The
file location can be overridden with `cntrdct scan --config <PATH>`.

The full schema lives in `src/config.rs` and is enforced via
`serde(deny_unknown_fields)`, so a typo in a section name is a hard
parse error rather than a silent skip.

## Top-level shape

```toml
[detectors.<id>]      # per-detector overrides (enabled, severity)
[paths]               # include / exclude globs
[languages.<canonical>]  # per-language overrides (enabled, suppress)
```

## `[detectors.<id>]`

```toml
[detectors.clone-drift]
enabled = false                # drop every clone-drift finding

[detectors.arg-swap]
severity = "error"             # remap raw severity

[detectors.unreachable-after-terminator]
enabled = true
severity = "note"
```

`enabled` accepts `true` or `false` (default `true`). `severity`
accepts the lowercase strings `"info"`, `"note"`, `"warning"`,
`"error"`; the value replaces the detector's `raw_severity` on every
finding before SARIF emission applies the IEEE 1044-2009 mapping
([Severity and anomaly classes (P5)](../concepts/severity.md)).

`<id>` values are exactly the strings in
`cntrdct::ALL_DETECTOR_IDS` — i.e. `arg-swap`, `clone-drift`,
`comment-code`, `config-interaction`, `pr-miner`,
`unreachable-after-terminator`. Unknown IDs are accepted but
ineffective; consider running `cntrdct scan` and checking findings
to confirm the section is wired.

## `[paths]`

```toml
[paths]
exclude = ["vendor/**", "**/generated/*.rs"]
include = ["src/**", "lib/**"]
```

Both fields are lists of `globset`-compatible patterns evaluated
against the finding's *primary* file (relative to the scan root).
The semantics are:

- `exclude` always wins. Any finding whose primary file matches an
  exclude glob is dropped.
- `include` is "allowlist mode" when non-empty: a finding survives
  only if its primary file matches at least one include glob.
- Both empty (or absent) is the default: every file is in scope.

Path filtering happens at the apply stage, not at file discovery, so
exclusion does not save scan time — it filters the output. To skip a
directory at discovery time, use a per-language disable (`[languages.<x>]
enabled = false`) or invoke cntrdct against a narrower path.

## `[languages.<canonical>]`

```toml
[languages.rust]
enabled = true

[languages.python]
enabled = true
suppress = ["pr-miner", "comment-code"]
```

Two effects:

- `enabled = false` causes the file walker to skip files of that
  language at discovery time. This is the right knob to turn off an
  entire language family.
- `suppress = ["<id>", ...]` drops findings whose primary file is in
  that language and whose `detector_id` is in the list. Equivalent in
  spirit to `[detectors.<id>] enabled = false`, but scoped to one
  language so a detector can stay on for Rust while being silenced on
  Python (or vice versa).

Canonical language names are `rust` and `python` at v0.4.3. Unknown
keys (`[languages.ruby]` etc.) are accepted but ineffective.

## Precedence

When multiple knobs apply to the same finding, the apply order is:

1. Language enablement (`[languages.<x>] enabled = false` removes the
   file from the scan).
2. Path filter (`[paths]` `exclude` then `include`).
3. Per-detector enablement (`[detectors.<id>] enabled = false`).
4. Per-language suppression (`[languages.<x>] suppress`).
5. In-source attributes ([In-source suppressions](./suppressions.md)).
6. Per-detector severity remap (`[detectors.<id>] severity = "..."`).

Any single drop in steps 1–5 short-circuits the finding before SARIF
emission. Severity remap (step 6) only fires on findings that survive
the drops.

## See also

- [In-source suppressions](./suppressions.md) — `#[cntrdct::allow(...)]`
  attribute and `# cntrdct: allow(...)` comment forms.
- [Severity and anomaly classes (P5)](../concepts/severity.md) — what
  remapping `severity` actually changes downstream.
