# comment-code detector v0 spec

Status: active draft, approved for TDD implementation 2026-05-03.

## Background

iComment (Tan, Yuan, Krishna, Zhou; SOSP 2007) and aComment (Tan, Zhou, Padioleau;
PLDI 2011) showed that comment text is a rich source of latent specification:
when the rendered prose disagrees with the implementation, it is often the
implementation that is wrong (a real bug) and sometimes the comment (a "bad
comment"). Either way, the disagreement is itself worth surfacing.

cntrdct's comment-code v0 takes the simplest possible cut at this idea: three
hard-coded English phrase patterns checked against three structural facts about
the function. No NLP, no learned templates — pattern-based pre-filter only.
Future iterations may add NLP via a Layer 3 adjudicator (P3); v0 keeps detection
deterministic and dependency-light per design constraint P3.

## Scope

- Detector: comment-code
- Language: Rust
- Granularity: top-level `fn` items with a `///` doc comment block immediately
  preceding the item
- Resolution: same-file only (the doc comment, the signature, and the body all
  live in one `function_item` node)

## Functional requirements

### F1 — Input

Accepts `&[ParsedFile]`. Files where `language != "rust"` are skipped. Files
that fail to parse (root has any error) are skipped silently, mirroring the
other detectors.

### F2 — Doc-comment extraction

For each top-level `function_item`, walk the immediately preceding sibling
`line_comment` nodes whose text starts with `///`. Concatenate the text of
those lines (after stripping the `///` prefix and one optional leading space)
into a single rendered doc string. Lowercase the rendered string for pattern
matching. Module-level `//!` comments and inner `//` comments are ignored.

If the rendered doc string is empty, the function is skipped (T7).

### F3 — Pattern A: Result/Option claim without matching return type

Trigger phrases (case-insensitive substring):
`returns err`, `returns result`, `may fail`, `fallible`, `returns option`,
`may return none`.

Constraint: the function's return type, taken textually from the
`return_type` field of `function_item`, does NOT contain the substring
`Result` or `Option`. If the function has no `return_type` field (i.e. unit
return), the constraint is satisfied (the doc claim still mismatches).

### F4 — Pattern B: panic claim without panicking constructs

Trigger: rendered doc string contains the substring `panic` (matches both
`panic` and `panics`).

Constraint: the body source text contains NONE of the substrings
`panic!`, `unwrap`, `expect(`, `unreachable!`, `assert!`, `assert_eq!`,
`assert_ne!`, `todo!`, `unimplemented!`, `debug_assert`. The body source
text is the literal substring of the file source for the `body` field of
`function_item`.

### F5 — Pattern C: deprecated claim without `#[deprecated]`

Trigger: rendered doc string contains the substring `deprecated`.

Constraint: the function does NOT have an attribute whose name path begins
with `deprecated` (e.g. `#[deprecated]`, `#[deprecated(note = "...")]`).

### F6 — Finding shape

Each detected mismatch produces one Finding:
- `detector_id = "comment-code"`
- `primary` = location of the `function_item`
- `related = []` (no related location for v0)
- `message` = `"doc comment claims '<trigger phrase>' but implementation does not match"`
- `raw_severity = Severity::Note` (lower than Warning since pattern-based)
- `anomaly_class = AnomalyClass::Documentation`
- `evidence.citation_keys = vec!["tan-sosp-2007", "tan-pldi-2011"]`
- `evidence.raw = {"pattern": "A"|"B"|"C", "trigger": <phrase>}`

### F7 — Output stability

Findings sorted by `(primary.file, primary.start_line)` lexicographically.

### F8 — Anomaly class

Every Finding emitted by comment-code sets
`anomaly_class = AnomalyClass::Documentation` (IEEE 1044-2009 §5.4). Rationale:
the contradiction is between documentation and implementation; the standard's
`Description`/`Documentation` classes are merged in cntrdct's enum under
`Documentation`.

## Non-functional requirements

### N1 — Determinism (P3)

Identical input produces identical output. No clock, no random, no I/O.

### N2 — Citation (P1)

The detector exposes both `tan-sosp-2007` and `tan-pldi-2011` `Citation`
structs from `citations()`. Enforced by `register_detector` and by the
workspace-wide `citations_consistency` test.

### N3 — No side effects

`detect()` performs no I/O.

### N4 — Robustness

Files that fail to parse are skipped silently in v0. Functions with no doc
comment, or with only `//`/`//!` comments, are skipped.

## Test plan

| ID | Description | Expected |
|---|---|---|
| T1 | doc says "Returns Err on failure", return type is `i32` | 1 Finding (Pattern A) |
| T2 | doc says "Returns Err on failure", return type is `Result<i32, E>` | 0 Findings |
| T3 | doc says "Panics if x is zero", body has no panicking construct | 1 Finding (Pattern B) |
| T4 | doc says "Panics if x is zero", body uses `unwrap()` | 0 Findings |
| T5 | doc says "Deprecated: use bar instead", no `#[deprecated]` attr | 1 Finding (Pattern C) |
| T6 | doc says "Deprecated: use bar instead", `#[deprecated]` present | 0 Findings |
| T7 | fn with no doc comment, body otherwise innocuous | 0 Findings |
| T8 | T1 fixture | every Finding has citation_keys ⊆ {tan-sosp-2007, tan-pldi-2011} |
| T9 | T1 fixture run twice | identical output |

## Non-goals (v0)

- Real NLP on doc text (synonym handling, negation detection).
- Multi-line claim parsing (e.g. distinguishing "panics on overflow" from
  "does not panic").
- Method docs on `impl` blocks; only top-level `fn` items.
- Cross-function reasoning (e.g. helper that calls a panicking fn).

## References (P1)

- `tan-sosp-2007` — L. Tan, D. Yuan, G. Krishna, Y. Zhou,
  "/*iComment: Bugs or Bad Comments?*/", SOSP 2007.
- `tan-pldi-2011` — L. Tan, Y. Zhou, Y. Padioleau,
  "aComment: Mining Annotations from Comments and Code to Detect
  Interrupt-related Concurrency Bugs", PLDI 2011.
