# config-interaction detector v0 spec

Status: active draft, approved for TDD implementation 2026-05-03.

## Background

Tartler et al., "Feature consistency in compile-time-configurable system
software" (EuroSys 2011), catalogue a family of variability anomalies in
software with rich preprocessor / build-system configuration: dead code
that is unreachable under any feature combination, items whose
configuration predicates contradict each other, and references that
escape their guard. Nadi et al. (ICSE 2014) reinforced the picture by
empirically mining configuration constraints from Linux and confirming
that contradictory `cfg` predicates appear regularly in long-lived
codebases.

Rust uses `#[cfg(...)]` attributes for the same role. When a single item
carries two cfg attributes that are syntactic negations of each other —
for example, `#[cfg(feature = "x")]` and `#[cfg(not(feature = "x"))]` —
the AND of those guards is unsatisfiable and the item is dead under
every configuration. `rustc` does not emit a warning for this case in
v0; the item simply never compiles.

cntrdct's `config-interaction` v0 surfaces this contradiction at the
AST level. Detection is deliberately conservative — it only reports
pairs whose inner predicates are structurally identical. Anything that
requires SAT-style reasoning over `all` / `any` predicates is out of
scope for v0.

## Scope

- Detector: `config-interaction`
- Language: Rust
- Granularity: top-level items (`function_item`, `struct_item`,
  `enum_item`, `mod_item`, `impl_item`, `trait_item`, `static_item`,
  `const_item`, `type_item`, `union_item`)
- Pattern: an item bears two `#[cfg(...)]` attributes whose predicates
  are structurally negations of each other
- Out of scope (v0): `cfg_attr`, `#[cfg(all(...))]` /
  `#[cfg(any(...))]` reasoning, cross-item reasoning, build-system
  configuration files, predicate satisfiability beyond literal pair
  contradiction

## Functional requirements

### F1 — Input

Accepts `&[ParsedFile]` via `DetectContext`. Files where
`language != "rust"` are skipped without error. Files whose tree-sitter
parse root has any error are skipped silently in v0, mirroring the
other detectors. Empty input returns `Ok(vec![])`.

### F2 — Item discovery

Walk every top-level item under the file root. Items inside `impl`,
`mod`, or `trait` blocks are also visited so a contradictory cfg pair
nested in an `impl` block still surfaces.

### F3 — cfg attribute extraction

For each visited item, walk the immediately preceding sibling
`attribute_item` nodes and the item's own inner-attribute children.
Keep an attribute when:

- its meta-item path is exactly `cfg` (single segment), AND
- it has exactly one direct argument (the predicate)

Reject `cfg_attr`, attributes with no arguments, and attributes whose
path is not `cfg`. Inner attributes inside the item body are walked the
same way; this lets the detector see `mod` items whose body holds an
inner `#![cfg(...)]`.

### F4 — Predicate canonicalization

The predicate text is the substring of the source covering the
predicate's tree-sitter node, with the following normalization:

- leading and trailing ASCII whitespace removed
- runs of internal ASCII whitespace collapsed to a single space
- `\r` and `\t` treated as whitespace
- comments inside the predicate left as-is (the detector does not strip
  them; predicates with embedded comments are vanishingly rare in
  practice and skipping them is a non-goal)

Two predicates are structurally equal iff their canonicalized texts are
byte-equal.

### F5 — Contradiction detection

For each item with two or more cfg attributes whose predicates are kept
under F3, perform pairwise comparison. Two predicates `a` and `b` are a
contradictory pair iff one of:

(F5a) `a` is of the form `not(X)` and `X` is structurally equal to `b`,
or `b` is of the form `not(X)` and `X` is structurally equal to `a`.
The shared inner predicate `X` is reported as
`evidence.raw.inner_predicate` and the kind is `not-pair`.

(F5b — added 2026-05-21) one predicate canonicalises to `true` and the
other canonicalises to `false`. Multiple cfg attributes on a single
item compose conjunctively per rustc's `cfg.attr.duplicates` reference
behaviour, so `#[cfg(true)] #[cfg(false)]` reduces to
`cfg(true) AND cfg(false) = cfg(false)` and the item is disabled under
every configuration. Reported as `contradiction_kind = true-false` with
`inner_predicate = "true vs false"`. Surface for the rustc UI test
`tests/ui/cfg/both-true-false.rs` (audit-corpus
`rustc_ui_both_true_false.rs` lines 11 and 15).

Self-symmetric pairs (where both are `not(X)` of each other) cannot
arise — `not(not(X))` differs syntactically from `X` and is therefore
out of scope for v0. Pairs that share `not(...)` wrappers but with
different inner predicates are not flagged. The detector emits at most
one Finding per item, even when more than one contradictory pair exists
(the pair with the lexicographically smaller `(line, column)` of the
first attribute is reported; the others are noted in
`evidence.raw.additional_pairs`).

### F6 — Finding shape

Each detected contradiction produces one Finding:

- `detector_id = "config-interaction"`
- `primary` = location of the item itself
- `related = [location of the first cfg attribute, location of the
  second cfg attribute]`
- `message = "item carries cfg pair `cfg(<P>)` and `cfg(not(<P>))` —
  unsatisfiable under any configuration"`
- `raw_severity = Severity::Warning`
- `anomaly_class = AnomalyClass::Logic`
- `evidence.citation_keys` includes both `tartler-eurosys-2011` and
  `nadi-icse-2014`
- `evidence.raw` carries:
  - `inner_predicate`: the canonicalized inner predicate string `<P>`
  - `attribute_lines`: `[line_a, line_b]` (1-based)
  - `additional_pairs`: number of further contradictory pairs on the
    same item (≥ 0)

### F7 — Output stability

Findings sorted by `(primary.file, primary.start_line, primary.start_col)`.

### F8 — Determinism (P3)

Identical input produces byte-identical `Vec<Finding>`. No clock, RNG,
filesystem, or LLM access from `detect`.

## Non-functional requirements

### N1 — Citation (P1)

Both `tartler-eurosys-2011` and `nadi-icse-2014` `Citation` structs are
exposed from `citations()`. Enforced by `register_detector` and the
workspace-wide `citations_consistency` test.

### N2 — No side effects

`detect()` performs no I/O or logging.

### N3 — Performance (target only, not gated)

10K LOC processed in under 5 s on a single core. The walk is
O(items × attrs²) per item; v0 caps `attrs²` at the literal attribute
count (typically ≤ 4 in practice).

### N4 — Robustness

Files that fail to parse are skipped silently. Items with zero or one
cfg attribute are skipped without finding.

## Test plan

| ID | Description | Expected |
|---|---|---|
| T1 | `#[cfg(feature = "x")] #[cfg(not(feature = "x"))] fn f() {}` | 1 Finding, primary on `fn f`, both attribute lines in `related`, `inner_predicate == "feature = \"x\""` |
| T2 | `#[cfg(unix)] #[cfg(not(unix))] struct S;` | 1 Finding |
| T3 | `#[cfg(unix)] fn f() {}` (single cfg) | 0 Findings |
| T4 | `#[cfg(feature = "a")] #[cfg(feature = "b")] fn f() {}` (no `not`) | 0 Findings |
| T5 | `#[cfg(not(unix))] #[cfg(unix)] fn f() {}` (order reversed) | 1 Finding |
| T6 | `#[cfg(all(unix, x))] #[cfg(not(all(unix, x)))] fn f() {}` | 1 Finding (predicates structurally equal, one is `not` of the other) |
| T7 | `#[cfg(unix)] #[cfg(not(windows))] fn f() {}` (different inner predicates) | 0 Findings |
| T8 | T1 fixture | every Finding has `evidence.citation_keys ⊇ {tartler-eurosys-2011, nadi-icse-2014}` |
| T9 | T1 fixture run twice | identical `Vec<Finding>` |
| T10 | empty input | 0 Findings, no error |
| T11 | non-rust file (language="javascript") | skipped silently |
| T12 | `cfg_attr(unix, cfg(not(unix)))` (cfg_attr nested) | 0 Findings (cfg_attr out of scope) |
| T13 | item with three contradictory cfg attributes (`P`, `not(P)`, `not(P)`) | 1 Finding, `additional_pairs == 1` |
| T14 | every Finding sets `anomaly_class = Logic` |  |
| T15 | `#[cfg(false)] #[cfg(true)] fn f() {}` | 1 Finding, `contradiction_kind == "true-false"` (F5b) |
| T16 | `#[cfg(true)] #[cfg(false)] fn f() {}` (order reversed) | 1 Finding (F5b is symmetric) |
| T17 | single `#[cfg(true)] fn f() {}` or single `#[cfg(false)]` | 0 Findings (≥ 2 attrs required, F5 surface unchanged) |
| T18 | existing F5a fixture | `contradiction_kind == "not-pair"` (F5b refactor must not leak into F5a evidence) |

## Non-goals (v0)

- Predicate satisfiability beyond literal pair contradiction
  (`#[cfg(all(P, not(P)))]`, `#[cfg(any(...))]` reasoning)
- `cfg_attr` rewriting
- Cross-item reasoning ("function `g` references `f` whose cfg is
  strictly narrower")
- Build-system configuration files (`Cargo.toml`, `build.rs`)
- Suggesting deletion / quick-fixes
- Whitespace / comment differences inside predicates (mostly absent in
  practice)

## References (P1)

- `tartler-eurosys-2011` — B. Tartler, D. Lohmann, J. Sincero,
  W. Schröder-Preikschat, "Feature consistency in compile-time-
  configurable system software: facing the Linux 10,000 feature
  problem", EuroSys 2011. Canonical reference for the dead-block /
  inconsistent-feature anomaly class this detector implements at AST
  level.
- `nadi-icse-2014` — S. Nadi, T. Berger, C. Kästner, K. Czarnecki,
  "Mining configuration constraints: Static analyses and empirical
  results", ICSE 2014. Empirical evidence that contradictory `cfg`
  predicates recur in production code and motivate dedicated detection.
