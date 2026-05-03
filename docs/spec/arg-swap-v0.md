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

Walk all `call_expression` nodes whose function operand is a single
`identifier` (qualified paths and method calls are out of scope). Record:
- callee name
- argument list as a vector of `Option<String>` (Some(name) if the argument
  is a simple identifier, None otherwise)
- argument count
- location

Skip calls where any argument is `None` (non-identifier).

### F4 — Resolution

For each call site, look up its callee name in the per-file definition table.
- If exactly one definition with matching name and argument count = 2 exists,
  use it.
- If zero or more than one match, skip.

### F5 — Swap detection

Given a 2-arg call to a 2-arg definition:
- Let `(a0, a1)` = call argument names (lowercased).
- Let `(p0, p1)` = parameter names (lowercased).
- The call is **swapped** iff `a0 == p1 && a1 == p0` AND NOT
  (`a0 == p0 && a1 == p1`). The second clause excludes cases where the user
  used the same names in the same order.

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

## Non-goals (v0)

- N-ary swap detection (3+ params)
- Method calls (`obj.method(a, b)`) and qualified paths
- Cross-crate / cross-module resolution beyond same scan
- Type-aware matching (params with same type are inherently swappable)
- Substring / fuzzy name matching

## References (P1)

- `li-zhou-fse-2005` — Li & Zhou, "PR-Miner: Automatically Extracting Implicit
  Programming Rules and Detecting Violations in Large Software Code", ESEC/FSE 2005
- `rice-icse-2017` — Rice, Aftandilian, Jaspan, Johnston, Pradel, Arroyo-Paredes,
  "Detecting Argument Selection Defects", ICSE 2017
