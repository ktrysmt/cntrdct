# unreachable-after-terminator detector v0 spec

Status: active draft, approved for TDD implementation 2026-05-03.

## Background

Hovemeyer & Pugh's FindBugs (OOPSLA 2004) introduced the "UR — Unreachable
code" bug pattern as a high-precision anomaly: code that the compiler can
prove will never execute is, almost without exception, either a logic error
(the programmer expected the preceding statement not to terminate) or
abandoned debug scaffolding. Engler et al. ("Bugs as Deviant Behavior",
SOSP 2001) generalised the same pattern: control-flow contradictions —
code reachable in the AST but unreachable along every dynamic path — are
among the highest-confidence anomaly classes in static analysis.

Rust's own `unreachable_code` lint catches many of these cases at the type
level, but the lint is `warn`-by-default and frequently silenced via
`#[allow(unreachable_code)]` or buried under `cargo build` noise. Surfacing
the same pattern in a SARIF stream alongside Layer 2 ranking and an
optional Layer 3 verdict gives reviewers a separate, citable signal.

## Scope

- Detector: `unreachable-after-terminator`
- Language: Rust
- Granularity: statements inside a `block` node that follow a divergent
  statement in the same block
- Terminator set (deliberately conservative — see Non-goals):
  - `return_expression` (with or without value)
  - `break_expression`
  - `continue_expression`
  - `macro_invocation` whose macro name is one of `panic`, `unreachable`,
    `todo`, `unimplemented`, `abort`, `exit` (`std::process::exit`-style
    macro wrappers all share the diverging contract)

## Functional requirements

### F1 — Input

Accepts `&[ParsedFile]` via `DetectContext`. Files where `language != "rust"`
are skipped without error. Files whose tree-sitter parse root has any error
are skipped silently in v0, mirroring the other detectors. Empty input
returns `Ok(vec![])`.

### F2 — Block discovery

Walk every `block` node in the file (including bodies of `function_item`,
`if_expression`, `else_clause`, `loop_expression`, `while_expression`,
`for_expression`, `match_arm`, and inline `block` expressions). The walker
recurses into inner blocks; each block is analysed independently.

### F3 — Terminator classification

A statement S inside a block is a terminator iff one of:

- S is an `expression_statement` whose first named child is one of
  `return_expression`, `break_expression`, `continue_expression`, or
- S is an `expression_statement` whose first named child is a
  `macro_invocation` whose macro name (the text of its `identifier` /
  `scoped_identifier` child) is in the terminator-macro set, or
- S is itself a bare diverging expression (`return_expression` etc.) when
  it appears as the last expression in a block — in that case there is
  nothing after it, so no Finding is produced.

`#[allow(unreachable_code)]` attached to any ancestor `function_item` or
`block` (inner attribute `#![allow(unreachable_code)]`) suppresses Findings
inside that scope. Detection is by textual substring match on the attribute
source (`unreachable_code` appears inside `allow(...)`); robust attribute
parsing is out of scope for v0.

### F4 — Following-statement detection

For each terminator S found at index i within a block of N statements, every
statement at index i+1..N is "unreachable-after-terminator". The Finding
points at the FIRST such following statement (index i+1); subsequent
statements in the same block are not flagged separately to avoid duplicate
noise on a single contradiction site.

### F4b — cfg-gated terminator suppression (added 2026-05-07)

A statement that would otherwise qualify as a terminator under F3 is NOT a
terminator if it carries a `#[cfg(...)]` attribute. The attribute may be
attached either as the immediately preceding sibling `attribute_item` within
the same block, or as a child `attribute_item` of the `expression_statement`
itself. Successive preceding `attribute_item` siblings are all considered.

Detection key: after stripping the leading `#[` (or `#![`) the FIRST
identifier is exactly `cfg`. The closely-named `cfg_attr(predicate, attr)`
form does NOT make the statement conditional — it conditionally applies an
inner attribute while the statement itself runs unconditionally — and is
therefore NOT a suppressor.

Rationale: in stable Rust, the canonical cross-platform / feature-gated
return idiom is

```rust
#[cfg(feature = "X")]
return foo();
#[cfg(not(feature = "X"))]
return bar();
```

Exactly one branch is active per cfg evaluation; both branches are NOT
simultaneously present in any compiled binary. The v0 detector treated them
as sequential statements, producing 100% false positives on the wild Rust β
corpus for this pattern (10/10 findings). The suppression in F4b is the
narrowest fix: only the terminator side is muted; an unconditional
terminator followed by a cfg-gated statement still fires (the follower IS
unreachable in any build where its cfg evaluates to true).

### F4c — Hoisted item suppression (added 2026-05-07)

Item declarations that appear inside a block (`function_item`,
`function_signature_item`, `mod_item`, `foreign_mod_item`, `struct_item`,
`union_item`, `enum_item`, `type_item`, `const_item`, `static_item`,
`trait_item`, `impl_item`, `use_declaration`, `extern_crate_declaration`,
`associated_type`, `macro_definition`) are NOT executable statements: the
Rust compiler hoists them so their textual position carries no runtime
ordering. They are filtered out of the block's statement list before F4
runs, alongside attributes, comments, and `empty_statement` (the bare `;`).

Concretely, a `fn` declared after a `return` does not constitute reachable-
or unreachable-code:

```rust
fn outer() {
    return helper();

    #[cold]
    fn helper() {}   // hoisted; not unreachable
}
```

Found surfacing in `semver__identifier.rs:377` on the wild Rust β corpus.
After F4c there are 0 findings of this shape on the corpus. F4c does NOT
mask genuine unreachable-after-terminator: an executable statement
appearing after the items still fires (T37).

### F5 — Finding shape

For each block where F4 fires, emit one Finding:

- `detector_id = "unreachable-after-terminator"`
- `primary` = location of the first unreachable statement (i+1)
- `related = [location of the terminator statement (i)]`
- `message = "statement is unreachable; preceded by <terminator-kind> on line <N>"`
  - `<terminator-kind>` is one of `return`, `break`, `continue`, or the
    macro name (e.g. `panic`)
  - `<N>` is the 1-based line of the terminator statement
- `raw_severity = Severity::Warning`
- `anomaly_class = AnomalyClass::Logic`
- `evidence.citation_keys` MUST include at least one of
  `hovemeyer-pugh-oopsla-2004`, `engler-sosp-2001`
- `evidence.raw` carries:
  - `terminator_kind`: string (one of the above)
  - `terminator_line`: u32 (1-based)
  - `following_count`: u32 — number of statements rendered unreachable by
    this terminator (≥ 1)

### F6 — Output stability

Findings sorted by `(primary.file, primary.start_line, primary.start_col)`.

### F7 — Anomaly class

`AnomalyClass::Logic`. The contradiction is between the program's static
control-flow graph and the implementation's textual layout — the canonical
"Logic" anomaly per IEEE 1044-2009 §5.4.

### F8 — Determinism (P3)

Identical input produces byte-identical `Vec<Finding>`. No clock, RNG,
filesystem, or LLM access from `detect`.

## Non-functional requirements

### N1 — Citation (P1)

Both `hovemeyer-pugh-oopsla-2004` and `engler-sosp-2001` `Citation`
structs are exposed from `citations()`. Enforced by `register_detector`
and the workspace-wide `citations_consistency` test.

### N2 — No side effects

`detect()` performs no I/O or logging.

### N3 — Performance (target only, not gated)

10K LOC processed in under 5s on a single core. Block walking is O(nodes);
no quadratic patterns.

### N4 — Robustness

Files that fail to parse are skipped silently. Empty blocks are skipped.
Functions / blocks consisting solely of attributes are skipped.

## Test plan

| ID | Description | Expected |
|---|---|---|
| T1 | `fn f() { return; bar(); }` | 1 Finding, primary on `bar()`, terminator_kind = `return` |
| T2 | `fn f() { return; }` (terminator is the only statement) | 0 Findings |
| T3 | `fn f() { panic!("x"); let x = 1; }` | 1 Finding, terminator_kind = `panic` |
| T4 | `fn f() { unreachable!(); foo(); bar(); }` (three follow) | 1 Finding (only the first follower flagged), `evidence.raw.following_count == 2` |
| T5 | `fn f(xs: &[i32]) { for x in xs { if *x == 0 { continue; foo(); } } }` | 1 Finding inside the inner block, terminator_kind = `continue` |
| T6 | `fn f() { #[allow(unreachable_code)] { return; bar(); } }` | 0 Findings (suppressed by allow) |
| T7 | `fn f() { if cond() { return 1; } bar(); }` (terminator only in inner block) | 0 Findings (no statement follows in the same block) |
| T8 | T1 fixture | every Finding's `evidence.citation_keys` ⊇ one of the recognized keys |
| T9 | T1 fixture run twice | identical `Vec<Finding>` |
| T10 | empty input | 0 Findings, no error |
| T11 | non-rust file (language="javascript") | skipped silently |
| T12 | `fn f() { todo!(); bar(); }` | 1 Finding, terminator_kind = `todo` |
| T13 | `fn f() { let x = 1; bar(); baz(); }` (no terminator) | 0 Findings |
| T14 | T1 with `#![allow(unreachable_code)]` inner attribute on the function block | 0 Findings (suppressed) |
| T15 | every Finding has `anomaly_class = Logic` |
| T29 | `#[cfg(unix)] return foo(); bar();` | 0 Findings (F4b: cfg-gated terminator) |
| T30 | `#[cfg(not(test))] panic!(); let x = 1;` | 0 Findings (F4b: cfg(not(...)) also matches) |
| T31 | complementary pair: `#[cfg(X)] return a();` followed by `#[cfg(not(X))] return b();` | 0 Findings (the wild-corpus β idiom) |
| T32 | `#[cfg_attr(test, cold)] return foo(); bar();` | 1 Finding (cfg_attr is NOT cfg; statement runs unconditionally) |
| T33 | `#[cfg(unix)] panic!(); let x = 1;` | 0 Findings (F4b applies to macro terminators too) |
| T34 | `return foo(); #[cfg(test)] debug();` (cfg on FOLLOWER) | 1 Finding (terminator is unconditional; follower IS unreachable when its cfg is true) |
| T35 | `fn outer() { return helper(); #[cold] fn helper() {} }` | 0 Findings (F4c: hoisted fn item) |
| T36 | hoisted items only after `return`: `const`, `static`, `use`, `struct`, `enum`, `type`, `mod`, `impl`, `trait` | 0 Findings (F4c) |
| T37 | `fn outer() { return; fn helper() {} bar(); }` | 1 Finding on `bar()` (items skipped, executable stmt still flags) |

## Tunable constants (v0 defaults)

- `TERMINATOR_MACROS = ["panic", "unreachable", "todo", "unimplemented", "abort", "exit"]`
- `SUPPRESSION_TOKEN = "unreachable_code"` (substring match in `allow(...)`)

Exposed as `pub const` for visibility; not user-tunable from CLI in v0.

## Non-goals (v0)

- Inter-procedural reachability (e.g. helper functions that always panic)
- Branch-merging analysis (`if cond { return 1 } else { return 2 } bar();`
  — v0 only sees the outer block as not having a terminator since the
  inner blocks each have their own; the FindBugs UR pattern handles this
  via dataflow, which is out of scope for the AST-only v0)
- `loop { ... }` without `break` (control-flow analysis required)
- `match` arms whose every branch diverges (ditto)
- Cross-language: only Rust in v0 (mirrors clone-drift / arg-swap /
  comment-code)
- Attribute parsing beyond substring match
- Suggesting deletion / quick-fixes

## References (P1)

- `hovemeyer-pugh-oopsla-2004` — D. Hovemeyer, W. Pugh, "Finding Bugs is
  Easy", OOPSLA 2004 (ACM SIGPLAN Notices 39(12)). Defines the "UR —
  Unreachable code" bug pattern category that this detector implements
  at the AST level.
- `engler-sosp-2001` — D. Engler, D.Y. Chen, S. Hallem, A. Chou, B. Chelf,
  "Bugs as Deviant Behavior: A General Approach to Inferring Errors in
  Systems Code", SOSP 2001. Establishes the broader principle that
  control-flow contradictions are high-confidence anomaly signals.
