# config-interaction

Flags a top-level Rust item that carries two `#[cfg(...)]` attributes
whose predicates are structurally negations of each other — so the
item is unsatisfiable under every feature configuration.

| Property | Value |
|---|---|
| Detector ID | `config-interaction` |
| Languages | Rust |
| IEEE 1044-2009 class | Logic |
| Default severity | Warning |
| Spec | [`docs/spec/config-interaction-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/config-interaction-v0.md) |

## What it flags

```rust
#[cfg(feature = "x")]
#[cfg(not(feature = "x"))]
fn never_compiled() {            // <- flagged: dead under every cfg
    // ...
}
```

The detector canonicalises each predicate's text, checks for the
`not(X)` / `X` pair structurally, and emits a finding pointing at the
item with both attribute locations in `related`. The
`evidence.raw.inner_predicate` field carries the canonical predicate
string so reviewers can audit the match without re-reading the source.

Detection is deliberately literal: only structural negation pairs
fire. Anything that requires SAT-style reasoning over `all(...)` /
`any(...)` combinations is out of scope for v0.

## Examples that fire

- `#[cfg(unix)]` + `#[cfg(not(unix))]` on the same struct.
- `#[cfg(all(unix, x))]` + `#[cfg(not(all(unix, x)))]` — the inner
  predicates are byte-equal after canonicalisation.
- Three attributes `P`, `not(P)`, `not(P)` — one finding;
  `evidence.raw.additional_pairs` records the extras.

## Examples that do not fire

- `#[cfg(feature = "a")]` + `#[cfg(feature = "b")]` — no `not(...)`.
- `#[cfg(unix)]` + `#[cfg(not(windows))]` — different inner predicates.
- `#[cfg_attr(test, cfg(not(unix)))]` — `cfg_attr` is out of scope at
  v0.
- A single `#[cfg(unix)]` on its own — no pair.

## Language coverage

Rust-only at v0. Python's `if sys.version_info` / typing-style
version guards have no AST-level analogue to `#[cfg]`, so the
detector does not extend cross-language.

Citations:

- `tartler-eurosys-2011` — Tartler, Lohmann, Sincero,
  Schröder-Preikschat. EuroSys 2011. The canonical reference for the
  feature-consistency anomaly family this detector targets.
- `nadi-icse-2014` — Nadi, Berger, Kästner, Czarnecki. ICSE 2014.
  Empirical evidence that contradictory `cfg` predicates recur in
  long-lived codebases.

Both citation keys appear on every finding.

## Configuration

```toml
[detectors.config-interaction]
enabled = false
```

Per-item suppression:

```rust
#[cntrdct::allow(config-interaction)]
#[cfg(feature = "x")]
#[cfg(not(feature = "x"))]
fn intentionally_unbuildable() {}
```

## Limitations

- No SAT solver — the v0 check is structural.
- `cfg_attr` is not unwrapped.
- Predicates that differ only in whitespace inside comments are not
  unified (the canonicaliser collapses runs of whitespace but does
  not strip embedded comments).
- Build-system files (`Cargo.toml`, `build.rs`) are out of scope; this
  detector only walks Rust source.

## Related

- [Four-layer architecture](../concepts/architecture.md)
- [Severity and anomaly classes (P5)](../concepts/severity.md) — Logic
  anomaly class.
- [unreachable-after-terminator](./unreachable-after-terminator.md) —
  the F4b cfg-gated terminator suppression interacts with the same
  attribute family from the opposite side (suppressing false
  positives on the canonical `#[cfg(X)] return ...; #[cfg(not(X))]
  return ...` idiom).
