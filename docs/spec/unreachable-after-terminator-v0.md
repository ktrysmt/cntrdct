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

Accepts `&[IrFile]` via `DetectContext` (R-1 / ir-v0.md F4 override).
Files whose `language` is not in `supported_languages()` are skipped
without error. Files whose IR conversion recovered
(`IrFile.parse_recovered == true`) are skipped silently in v0,
mirroring the other detectors. Empty input returns `Ok(vec![])`.

### F2 — Block discovery

The detector consumes IR semantically (R-1.c'' Path b; no `raw_tree()`
reparse). Starting from each `IrFn.body`, recurse through every
[`IrBlock`] the converter materialises — `IrIfStmt.{consequence,
alternative}`, `IrWhileStmt.body`, `IrLoopStmt.body`, `IrForStmt.body`,
`IrMatchStmt` arm bodies, `IrWithStmt.body`, `IrTryStmt.{body, handlers,
orelse, finalbody}`, and `IrExprKind::Block` (e.g. a `let` RHS block) —
analysing each block independently. This reaches the same block set the
v0.5.x raw walk visited, so the T1 pinning stays byte-identical.

### F3 — Terminator classification

A statement S inside a block is a terminator classified directly from
its [`IrStmtKind`]:

- Rust: `Return` → `return`, `Break` → `break`, `Continue` → `continue`,
  `DivergentCall { kind }` → the macro name (`panic` / `unreachable` /
  `todo` / `unimplemented` / `abort` / `exit`), `If` / `Match` whose
  pre-computed `terminator` is `BranchMerge { .. }` → `if-branches-diverge`
  / `match-arms-diverge`, and `Loop` with `has_break_to_self == false` →
  `loop-no-break`. `assert!` is intentionally NOT a terminator.
- Python: `Return` → `return`, `Raise` → `raise`, `Break` → `break`,
  `Continue` → `continue`, `DivergentCall { kind }` → the exit-call name
  (`sys.exit` / `sys.abort` / `os._exit` / `exit` / `quit`), and `Assert`
  of the literal `false` → `assert`. Python has no branch-merge.

`IrStmtKind::HoistedItem` is excluded from the Rust statement stream
(F4c). The block-level rule reports the statement immediately following
the first non-cfg-gated terminator (`IrStmt.location` for both the
follower and the terminator).

Suppression (Rust only): `#[allow(unreachable_code)]` on the enclosing
function ([`IrFn.decorators`]) or on any statement of the block
([`IrStmt.attributes`], the IR home for the block's direct
`attribute_item` / inner `#![...]` children) suppresses the block-level
findings in that scope (threaded down to nested blocks). Detection is a
textual substring match on the attribute source (`unreachable_code`); the
F4d rules are not gated by suppression, matching v0.5.x. Python carries
no detector-internal suppression — the `# cntrdct: allow(...)` form is
handled by `crate::config::apply`.

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

### F4d — Compound divergence (added 2026-05-21)

The v0 detector recognised only `expression_statement` whose first
child is a terminator (return / break / continue / panic-macro).
Audit corpus inspection against rustc's `tests/ui/reachable/` suite
showed five expected findings that the AST-local F3 / F4 walker
cannot reach — they require classifying compound expressions as
divergent. F4d adds four sub-rules. Each is conservative: a compound
expression diverges only when every sub-expression that contributes
to its value already diverges.

(F4d-i) Branch-merge if / match. A bare `if_expression` (or
`match_expression`) that appears as a block statement is itself a
terminator iff every branch's body diverges:

```rust
if cond { return; } else { return; }
bar();   // F4d-i: unreachable after if-branches-diverge
```

`if cond { return }` WITHOUT an else clause is conditional and does
NOT diverge (pinned by t41). A `match` whose arms include at least
one non-divergent arm (`_ => fallback()`) likewise does NOT diverge
(t43). Else-if chains recurse through the `alternative` field.

(F4d-ii) Call-argument divergence. `call_expression` arguments
evaluate left-to-right. A `return_expression` (or other divergent
expression) at argument position `i`:

- Flags `arguments[i+1]` as unreachable when an `i+1` argument
  exists (mirrors rustc's `expr_call.rs#L13` shape).
- Flags the entire `call_expression` when the divergent argument is
  the only / last one (rustc `expr_call.rs#L18`); the function body
  is never invoked because argument evaluation diverges first.

`macro_invocation` is deliberately excluded: tree-sitter-rust does
NOT re-parse macro token trees as Rust expressions, so
`panic!(return)`-style cases are not visible at the AST level.
Re-parsing macro inputs is preregistered for a separate scope lift.

(F4d-iii) Divergent return / break carrier.
`return EXPR` (or `break EXPR`) where `EXPR` evaluation diverges
flags the outer `return` / `break` as unreachable — control never
reaches the surrounding transfer because `EXPR` already diverged.
The canonical shape is the nested-return idiom rustc reports as the
"2nd-innermost return is unreachable" (`expr_return.rs#L10`).

(F4d-iv) Divergent if-condition. `if COND { ... }` where `COND` is
a `block` (or other expression) that diverges flags the consequence
block as unreachable — the condition never produces a value, so the
body is never selected. Pinned by t47 against rustc's
`expr_if.rs#L7` shape.

(F4d-v) Loop without targeting break (added 2026-05-21). A bare
`loop_expression` diverges when no `break_expression` in its body
targets the same `loop_expression`. Targeting is resolved
syntactically:

- An unlabelled `break;` targets the innermost enclosing loop-like
  construct (`loop_expression`, `while_expression`, `for_expression`).
  It only exits the candidate `loop_expression` when no other
  loop-like construct lies between the candidate and the break.
- A labelled `break 'name;` targets the loop-like construct that
  carries the same label.
- `closure_expression` and `async_block` introduce a hard scope
  boundary: a `break` inside one of them cannot syntactically target
  a loop on the outside (Rust forbids it), so the walk does not
  descend into them when computing target reachability.

The classifier emits `terminator_kind = "loop-no-break"`. Examples:

```rust
loop { return; } bar();   // F4d-v: loop body diverges; bar unreachable
'outer: loop { loop { break 'outer; } } bar();   // 0 Findings — outer has a labelled break targeting it
loop { 'middle: loop { loop { break 'middle; } } } bar();   // F4d-v: outermost has no break targeting it
loop { break; } bar();   // 0 Findings — innermost-rule break targets self
```

Pinned by t50, t51, t52, t53 against rustc's `expr_loop.rs#L7,L17,L29`
shapes. The F4d non-goal for `loop { ... }` no-break (formerly t48) is
retired by F4d-v; the equivalent assertion now lives at t50 with the
opposite expected count.

### Divergent-expression classifier

F4d-i / F4d-ii / F4d-iii / F4d-iv / F4d-v share a single recursive
classifier `rust_expression_diverges(expr) -> Option<terminator_kind>`:

- `return_expression` / `break_expression` / `continue_expression`
  diverge directly.
- `macro_invocation` diverges iff its macro name is in
  `TERMINATOR_MACROS`.
- `block` diverges iff one of its statements is a terminator under
  F3 / F4d-i, OR its tail expression diverges (recursive).
- `if_expression` diverges iff F4d-i fires (consequence and every
  alternative diverge).
- `match_expression` diverges iff every arm's value expression
  diverges.
- `loop_expression` diverges iff F4d-v fires (no `break` in the body
  targets this loop).

Recursion follows the finite AST hierarchy and always terminates.

### F4e — Python constant-condition branch (added 2026-05-21)

Python's audit-corpus FN review against CodeQL's `UnreachableCode`
query test fixture (`codeql_python_unreachable_test.py`) showed four
expected findings that the AST-local F3 walker cannot reach because
they require evaluating the branch condition at parse time, not
following a terminator:

- `while False:` or `while 0:` — the body is unreachable.
- `if False: BODY` — `BODY` is unreachable; any `elif` / `else`
  branches remain reachable.
- `if True: BODY else: OTHER` — `OTHER` is unreachable; subsequent
  `elif` arms (which are syntactic sugar for a nested `else: if`) are
  also unreachable, but `BODY` is reachable.

The constant-condition classifier `python_constant_condition(node)`
recognises a closed set of literal forms in v0:

- `false` and `true` (the `False` / `True` keyword tokens).
- `integer` literal whose numeric text is exactly `0` (falsy) or any
  non-zero integer (truthy).
- `none` (the `None` keyword; falsy).
- `string` whose surface text (with surrounding quotes stripped) is
  empty (`""` / `''`; falsy).

All other expression shapes — identifier references, attribute access,
calls, arithmetic, parenthesised expressions, conjunctions —
fall back to "indeterminate" and produce no F4e finding. Constant
folding of `not False`, `0 == 0`, `True and True`, container literals
(`[]`, `{}`, `()`, `set()`), and the `while 0.0:` / `if None:` float
shape are explicit non-goals.

F4e fires emit `terminator_kind = "constant-false-while"`,
`"constant-false-if"`, or `"constant-true-if-else"` (so calibration
priors can stratify by sub-rule) and carry the original `if` / `while`
keyword node as the related location. Pinned by t55 - t59 against the
CodeQL fixture shapes.

F4e carve-outs (matching CodeQL's UnreachableCode test fixture
explicit non-findings, `codeql_python_unreachable_test.py` lines
96-97 and 114-116):

- Type-checking import guard. `if False:` is silently treated as a
  non-finding when its block contains only `import_statement`,
  `import_from_statement`, or `future_import_statement` named
  children. This is the pre-PEP-484 fallback for the modern
  `if typing.TYPE_CHECKING:` idiom and produces no runtime code; the
  body is unreachable by design.
- Generator-marker idiom. `if False:` is silently treated as a
  non-finding when its block contains exactly one statement whose
  inner expression is a `yield` (`yield_expression`). The idiom
  forces the surrounding `def` to be a generator function without
  ever executing the yield (CodeQL `ODASA-6783`).

Both carve-outs apply only to the F4e-ii (`constant-false-if`)
sub-rule; the F4e-i (`constant-false-while`) and F4e-iii
(`constant-true-if-else`) sub-rules never short-circuit.

### F4f — Python unreachable `except` handler (added 2026-06-03, R-5)

F4f extends the "unreachable code" anomaly class to Python exception
handling: an `except` clause that can never execute because an earlier
handler in the same `try` already catches the same exception class or one
of its superclasses. Python tests `except` clauses top-to-bottom and runs
the first whose type matches (or is a superclass of) the raised exception,
so a later handler for a subclass — or a duplicate — is dead code.

```python
try:
    risky()
except Exception:      # catches everything under Exception
    handle()
except ValueError:     # F4f: unreachable — ValueError ⊆ Exception
    never_runs()
```

Unlike F4d / F4e, F4f needs the exception CLASS HIERARCHY, which is
language-specific. It therefore ships as a SEPARATE language-specific
detector, `python-unreachable-except`, under
`src/detectors/lang/python_unreachable_except.rs` (the first detector in the
post-R-1 `src/detectors/lang/` tier). It is documented here
because it belongs to the same "unreachable code" anomaly class, but it is
NOT part of the cross-cutting `unreachable-after-terminator` detector and
reads the raw tree-sitter tree directly (Pattern B, ir-v0.md §F5).

Rule (ordering / subsumption, v0): for each `try_statement`, examine its
`except_clause` handlers in source order. A handler H is unreachable iff
EVERY exception type it catches is provably a subclass-or-equal of a type
caught by an earlier handler.

- Caught types: `except E:` / `except E as e:` → `{E}`; `except (A, B):` →
  `{A, B}` (unreachable only when ALL elements are covered — partial
  coverage leaves the handler reachable for the uncovered element);
  bare `except:` → `{BaseException}` (the universal root).
- Subclass resolution chains the embedded CPython builtin hierarchy
  (`data/python-builtin-exceptions.json`) with same-file user classes
  (`class Foo(Bar): ...`). `BaseException` (or bare `except:`) covers
  everything unconditionally; `Exception` covers everything that resolves
  under `Exception`.
- INDETERMINATE relationships (a name not resolvable from
  builtins ∪ same-file classes — e.g. an imported exception type) are never
  treated as a subclass, so an unknown type produces no false positive
  (precision-first, matching the conservative philosophy of F3 / F4d).

Carve-outs / non-goals (preregistered for F4f):
- PEP 654 `except*` exception groups: a `try` carrying any
  `except_group_clause` is skipped whole (not analysed) in v0.
- Body raise-set inference (flagging a handler for an exception the `try`
  body provably cannot raise) is NOT done — it requires inter-procedural
  raise inference and is low precision.
- Cross-module / imported user-defined exception hierarchies are not
  resolved (indeterminate → not flagged).

Finding shape (`python-unreachable-except`): `anomaly_class = Logic`,
`raw_severity = Warning`; `primary` = the unreachable handler's type
expression; `related = [the covering handler's type]`; `message =
"except handler is unreachable; <child> is already caught by <ancestor> on
line <N>"`; `evidence.citation_keys ⊇ {hovemeyer-pugh-oopsla-2004,
de-padua-shang-icpc-2017}`; `evidence.raw` carries `caught_type`,
`covering_type`, `covering_line`. Python coverage is
`LanguageCitationStatus::Unconfirmed` per `docs/spec/citations-policy.md`
(survey: `docs/surveys/python-unreachable-except-python-2026-06.md`).

### F4d / F4e non-goals (preregistered)

The following remain explicit non-goals; lifting them requires a
separate spec extension with its own corpus pre-registration:

- `while cond { ... }` (Rust) where `cond` is a constant true
  (constant folding for Rust expressions; spec-mirror of F4e but on
  the Rust side).
- Python `except` handler reachability based on the exception type:
  the ordering / subsumption case is now implemented as the separate
  `python-unreachable-except` detector (see F4f). Body raise-set inference
  (a handler for an exception the `try` body cannot raise) remains a
  non-goal.
- Macro argument re-parsing (`panic!(return, x)` etc.).
- Python `while 0.0:` / `while None:` / `while []:` /
  `while "":` constant-falsy shapes beyond the four literal kinds
  named in F4e (integer / boolean / `None` / empty string). Reserved
  for a v1 widening once the FP rate of v0 F4e is measured.

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
| T40 | `fn f() { if true { return; } else { return; } bar(); }` | 1 Finding on `bar()`, terminator_kind = `if-branches-diverge` (F4d-i) |
| T41 | `fn f() { if true { return; } bar(); }` (no else) | 0 Findings (F4d-i requires every branch divergent) |
| T42 | `match x { 0 => return, 1 => return, _ => return } bar();` | 1 Finding on `bar()`, terminator_kind = `match-arms-diverge` (F4d-i) |
| T43 | `match x { 0 => return, _ => 1 }; bar();` (one fallthrough arm) | 0 Findings (F4d-i requires every arm divergent) |
| T44 | `foo(return, 22)` | 1 Finding on `22` (F4d-ii: subsequent arg unreachable) |
| T45 | `bar(return)` | 1 Finding on the call (F4d-ii: only-arg divergent, call never invokes) |
| T46 | `let _x: () = { return { return; } };` | ≥ 1 Finding on the outer return (F4d-iii: nested return) |
| T47 | `if { return } { bar(); }` | ≥ 1 Finding on the consequence block (F4d-iv: divergent condition) |
| T50 | `loop { return; } bar();` | 1 Finding on `bar()`, terminator_kind = `loop-no-break` (F4d-v: retires T48) |
| T51 | `loop { break; } bar();` | 0 Findings (F4d-v: innermost-rule break exits the loop) |
| T52 | `'outer: loop { loop { break 'outer; } } bar();` | 0 Findings (F4d-v: outer has a labelled break targeting it) |
| T53 | `loop { 'middle: loop { loop { break 'middle; } } } bar();` | 1 Finding, terminator_kind = `loop-no-break` (outer has no targeting break) |
| T54 | closure inside loop with bare `break;`: `loop { let _c = || { loop { break; } }; } bar();` | ≥ 1 Finding (F4d-v: closures are a hard break-target boundary) |
| T55 | `def f(): while False: x = 1` | 1 Finding on `x = 1`, terminator_kind = `constant-false-while` (F4e-i) |
| T56 | `def f(): while 0: x = 1` | 1 Finding, terminator_kind = `constant-false-while` |
| T57 | `def f(): if False: x = 1` | 1 Finding on `x = 1`, terminator_kind = `constant-false-if` (F4e-ii) |
| T58 | `def f(): if True: x = 1 else: y = 2` | 1 Finding on `y = 2`, terminator_kind = `constant-true-if-else` (F4e-iii) |
| T59 | `if False: from typing import Any` | 0 Findings (F4e-ii carve-out: type-check import idiom) |
| T60 | `def gen(): if False: yield None` | 0 Findings (F4e-ii carve-out: generator-marker idiom, CodeQL ODASA-6783) |
| T61 | `def f(x): if x: a = 1 else: b = 2` | 0 Findings (F4e silent for non-literal conditions) |

## Tunable constants (v0 defaults)

- `TERMINATOR_MACROS = ["panic", "unreachable", "todo", "unimplemented", "abort", "exit"]`
- `SUPPRESSION_TOKEN = "unreachable_code"` (substring match in `allow(...)`)

Exposed as `pub const` for visibility; not user-tunable from CLI in v0.

## Non-goals (v0)

- Inter-procedural reachability (e.g. helper functions that always panic)
- Rust-side `while cond { ... }` constant folding (constant-folding for
  Rust expressions is preregistered separately; F4d-v handles only the
  bare `loop { ... }` shape).
- Python `except` handler reachability based on the exception type:
  the ordering / subsumption case is now implemented as the separate
  `python-unreachable-except` detector (see F4f). Body raise-set inference
  (a handler for an exception the `try` body cannot raise) remains a
  non-goal.
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
