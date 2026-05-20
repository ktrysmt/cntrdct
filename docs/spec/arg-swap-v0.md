# arg-swap detector v0 spec

Status: active draft, approved for TDD implementation 2026-05-03.

## Background

PR-Miner (Li & Zhou ESEC/FSE 2005) and "Detecting Argument Selection Defects"
(Rice et al ICSE 2017) detect calls where the programmer reversed argument
order. The Rice approach correlates parameter names from the function
definition with argument names at the call site.

cntrdct's arg-swap v0 follows the Rice approach in its simplest form: resolve
calls to definitions in the **same source file**, compare argument identifier
names against parameter names, and flag any call where a clean swap is the
best match.

## Scope

- Detector: arg-swap
- Language: Rust
- Granularity: top-level `fn` definitions + call sites
- Resolution: same-file only (no cross-crate, no method dispatch resolution)
- Parameter count: exactly 2 (binary functions)
- Argument shape: simple identifiers only

## Functional requirements

### F1 — Input

Accepts `&[ParsedFile]`. Files where `language != "rust"` are skipped.

### F2 — Definition extraction

For each ParsedFile, walk top-level `fn` items. Record:
- function name
- parameter count
- parameter names (in declaration order)

Skip definitions that:
- have parameter count != 2
- have any `_`-prefixed parameter name
- have duplicate parameter names (Rust forbids this anyway)

### F3 — Call-site extraction

Walk all `call_expression` (Rust) and `call` (Python) nodes whose
function operand is a single `identifier` and whose argument list
contains only bare identifier arguments. Record:
- callee name
- argument list as a vector of identifier names (in declaration order)
- argument count
- location

Skip calls where any argument is non-identifier (keyword args,
splats, literals, nested calls).

### F3b — Method calls on `self` / `cls` (added 2026-05-21)

In Python, the function operand may also be an `attribute` node of
the shape `self.<name>` or `cls.<name>`. The attribute's identifier
becomes the callee, mirroring how the corresponding method is
registered in the per-file definition table (F4b). Deeper attribute
chains (`obj.foo`, `self.x.y`) remain out of v0 scope — resolving the
receiver type requires flow analysis. In Rust, method calls
(`obj.method(args)`) are still skipped: tree-sitter-rust's
`method_call_expression` is a distinct node kind from
`call_expression` and Rust's UFCS/trait-resolution adds further
ambiguity.

### F4 — Resolution

For each call site, look up its callee name in the per-file definition table.
- If exactly one definition with matching name and argument count = 2 exists,
  use it.
- If zero or more than one match, skip.

### F4b — Class methods (added 2026-05-21)

In Python, walk into every `class_definition` body and register its
`function_definition` (and `decorated_definition`-wrapped) methods
under their bare name. The implicit `self` or `cls` receiver is the
first parameter of any method per Python convention; F4b drops it
before the `_`-prefix rejection rule (F2) and the arity check (F4),
so a method declared `def foo(self, a, b):` registers with two
parameters `[a, b]` and matches the same call shape that F3b
accepts via `self.foo(args)`.

### F5 — Swap detection

Given a 2-arg call to a 2-arg definition:
- Let `(a0, a1)` = call argument names (lowercased).
- Let `(p0, p1)` = parameter names (lowercased).
- Define `name_matches(a, b)` = either `a == b` (F5a strict path)
  or one of `a` / `b` is a strict prefix of the other AND the
  shorter side is at least `PREFIX_MATCH_MIN_CHARS = 3` characters
  long (F5b prefix path). Equal-length distinct names never match.
- The call is **identity** iff `name_matches(a0, p0) AND
  name_matches(a1, p1)`.
- The call is **swapped** iff `name_matches(a0, p1) AND
  name_matches(a1, p0) AND NOT identity`.

F5b is added 2026-05-21. The matching kind that produced the
swap is recorded as `evidence.raw.match_kind = "strict"` for F5a
or `"prefix"` for F5b. The Rice et al. ICSE 2017 detector — already
cited as `rice-icse-2017` — uses abbreviation-aware matching to
recover swaps where the call site abbreviates the parameter name
(`dst` against `dstfn`, `inf` against `info`, the audit-corpus
`rarfile_set_attrs.py:14` shape).

### F6 — Finding shape

Each detected swap produces one Finding:
- `detector_id = "arg-swap"`
- `primary` = location of the swapped call
- `related = [location of the definition]`
- `message` = `"call argument order swapped relative to definition of `<name>`"`
- `raw_severity = Warning`
- `evidence.citation_keys` includes at least one of `li-zhou-fse-2005`,
  `rice-icse-2017`
- `evidence.raw` carries: `callee_name`, `parameter_names`, `argument_names`

### F7 — Output stability

Findings sorted by `(primary.file, primary.start_line)` lexicographically.

### F8 — Anomaly class

Every Finding emitted by arg-swap sets `anomaly_class = AnomalyClass::Interface`
(IEEE 1044-2009 §5.4). Rationale: a swapped-argument call is a violation of the
callee's parameter contract — the canonical "Interface" anomaly — not a logic
bug inside either function body. Surfaced in SARIF as
`result.properties.anomalyClass = "Interface"`.

## Non-functional requirements

### N1 — Determinism (P3)

Identical input produces identical output.

### N2 — Citation (P1)

Enforced by `register_detector`.

### N3 — No side effects

`detect()` performs no I/O.

### N4 — Robustness (N5 from clone-drift mirror)

Files that fail to parse are skipped silently in v0.

## Test plan

| ID | Description | Expected |
|---|---|---|
| T1 | def `fn copy(dst, src)` + call `copy(src, dst)` | 1 Finding |
| T2 | def `fn copy(dst, src)` + call `copy(d, s)` | 0 Findings (no name match) |
| T3 | def `fn copy(dst, src)` + call `copy(dst, src)` | 0 Findings (correct order) |
| T4 | def `fn one(x)` + call `one(x)` | 0 Findings (single-arg out of scope) |
| T5 | def `fn three(a, b, c)` + call `three(c, b, a)` | 0 Findings (3-ary out of scope) |
| T6 | call with literal arg `copy(x, 42)` | 0 Findings |
| T7 | call to unknown function | 0 Findings |
| T8 | T1 fixture | every Finding has citation_key in {li-zhou-fse-2005, rice-icse-2017} |
| T9 | T1 fixture run twice | identical output |
| T10 | empty input | 0 Findings, no error |
| T11 | def + call in different files (same scan) | 1 Finding (cross-file resolution within scan) |
| T25 | Python class method declared with `self`, called via `self.copy(src, dst)` | 1 Finding, `match_kind = "strict"` (F3b + F4b) |
| T26 | rarfile shape: `self._set_attrs(dst, inf)` against `def _set_attrs(self, info, dstfn)` | 1 Finding, `match_kind = "prefix"` (F5b) |
| T27 | call `copy(s, d)` against def `copy(dst, src)` (single-letter args) | 0 Findings (F5b prefix floor) |
| T28 | call `copy(tar, src)` against def `copy(target_buf, source_buf)` (identity by prefix) | 0 Findings (F5b identity precedence) |
| T29 | classmethod with `cls` receiver | 1 Finding (F4b drops `cls` like `self`) |

## Non-goals (v0)

- N-ary swap detection (3+ params)
- Rust method calls (`obj.method(a, b)`) and Python attribute chains
  beyond the single-segment `self.<name>` / `cls.<name>` shape
  admitted by F3b
- Cross-crate / cross-module resolution beyond same scan
- Type-aware matching (params with same type are inherently swappable)
- Fuzzy name matching beyond the strict-prefix shape admitted by F5b
  (no Levenshtein, no edit distance, no semantic embeddings)

## References (P1)

- `li-zhou-fse-2005` — Li & Zhou, "PR-Miner: Automatically Extracting Implicit
  Programming Rules and Detecting Violations in Large Software Code", ESEC/FSE 2005
- `rice-icse-2017` — Rice, Aftandilian, Jaspan, Johnston, Pradel, Arroyo-Paredes,
  "Detecting Argument Selection Defects", ICSE 2017
