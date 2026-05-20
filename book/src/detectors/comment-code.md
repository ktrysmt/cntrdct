# comment-code

Flags a doc comment whose rendered prose claims a behaviour the
implementation does not exhibit. The contradiction is between the
documented contract and the actual code.

| Property | Value |
|---|---|
| Detector ID | `comment-code` |
| Languages | Rust, Python |
| IEEE 1044-2009 class | Documentation |
| Default severity | Note |
| Spec | [`docs/spec/comment-code-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/comment-code-v0.md) |

## Three patterns

The Tan et al. iComment (SOSP 2007) and aComment (PLDI 2011) papers
laid out three structural patterns that a pattern-based detector can
match without NLP. cntrdct ships all three:

| Pattern | Trigger phrase | Constraint |
|---|---|---|
| A — Result / Option claim | `returns err`, `may fail`, `fallible`, `returns option`, `may return none` | function's `return_type` does not contain `Result` or `Option` |
| B — Panic claim | `panic` substring (matches `panic` and `panics`) | function body contains none of `panic!`, `unwrap`, `expect(`, `unreachable!`, `assert!`, `assert_eq!`, `assert_ne!`, `todo!`, `unimplemented!`, `debug_assert` |
| C — Deprecated claim | `deprecated` substring | function does not carry a `#[deprecated]` attribute |

### Pattern A example

```rust
/// Returns Err on bad input.
fn parse(input: &str) -> i32 {   // <- flagged: not Result/Option
    input.parse().unwrap_or(0)
}
```

### Pattern B example

```rust
/// Panics if `x` is zero.
fn safe_divide(x: i32, y: i32) -> i32 {   // <- flagged: no panic site
    if x == 0 { return 0; }
    y / x
}
```

### Pattern C example

```rust
/// # Deprecated
/// Use `bar` instead.
fn foo() {}                       // <- flagged: no #[deprecated] attr
```

## Python extensions

Python ships the same three patterns, with two suppression rules
added in response to wild-corpus residuals:

- F5b — `:raises:` factory suppression. A docstring that mentions a
  raise but whose function returns a *call expression* (factory shape,
  e.g. attrs's `instance_of(type) -> _InstanceOfValidator(type)`) is
  reporting the returned object's behaviour, not the factory's. The
  raise-pattern is suppressed.
- F5c — `.. deprecated::` directive subject. reST directives whose
  body starts with `*` or backtick deprecate a *parameter*, not the
  function. Function-level directives still fire.

## Citation grounding

- Rust: confirmed via `tan-sosp-2007` (iComment Pattern A / B / C
  taxonomy) and `tan-pldi-2011` (aComment). Both are emitted on every
  finding.
- Python: unconfirmed per
  [`docs/surveys/comment-code-python-2026-05.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/surveys/comment-code-python-2026-05.md).
  P1 is satisfied by the Rust citations under the citations-policy
  grandfather clause for cross-language extensions.

## Audit signal

The Q-14 recall-audit harness closed `comment-code` at **34 / 0 /
1.00** across twenty-three permissive-licensed upstreams, saturating
all three Tan SOSP 2007 patterns. This is the strongest single recall
signal cntrdct currently publishes; the other five detectors all sit
at 0.00 recall_upper_bound by design (they exercise narrower v0 scope
choices that the audit corpus deliberately overshoots — see
[`docs/spec/recall-audit-v0.md`](https://github.com/ktrysmt/cntrdct/blob/master/docs/spec/recall-audit-v0.md)).

## Configuration

```toml
[detectors.comment-code]
enabled = true
severity = "Warning"   # raise above the default Note
```

In-source:

- Rust: `#[cntrdct::allow(comment-code)]` on the function.
- Python: `# cntrdct: allow(comment-code)` immediately before the
  function definition (Q-9 suppression scanner).

## Related

- [Citation policy (P1)](../concepts/citations.md)
- [Severity and anomaly classes (P5)](../concepts/severity.md) — the
  Documentation anomaly class is unique to this detector at v0.
