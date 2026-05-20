# In-source suppressions

Findings can be suppressed at the call site without editing
`cntrdct.toml`. Two equivalent surfaces ship: a Rust attribute and a
Python line comment. Both are recognised by the Q-9 / Q-10 tree-
sitter parser seam and apply at the apply stage after Layer 1
finishes — the detector still runs, but the matching findings are
filtered before they reach SARIF emission.

## Rust — `#[cntrdct::allow(...)]`

```rust
#[cntrdct::allow(clone-drift)]
fn looks_like_a_drifted_clone_but_is_intentional() { /* ... */ }
```

The attribute precedes the item it suppresses. The argument list
takes one or more detector IDs; multiple IDs in a single attribute
are equivalent to stacking multiple attributes.

```rust
#[cntrdct::allow(clone-drift, unreachable-after-terminator)]
fn double_suppressed() { /* ... */ }
```

Empty argument list is the catch-all: every detector is silenced on
that item.

```rust
#[cntrdct::allow()]
fn silence_everything_on_this_function() { /* ... */ }
```

Recognised attribute scopes:

- `function_item` — the most common case.
- `struct_item`, `enum_item`, `impl_item`, `mod_item`,
  `static_item`, `const_item`, `trait_item`, `type_item`,
  `union_item` — anywhere a Rust outer attribute attaches.
- Block-level inner attributes (`#![cntrdct::allow(...)]`) suppress
  the enclosing block.

The suppression range covers the entire item span, so a finding
anywhere inside the function — primary or related — is dropped.

## Python — `# cntrdct: allow(...)`

The Q-9 tree-sitter-python suppression scanner recognises two
positions for the comment:

### Trailing form

```python
do_something(b, a)  # cntrdct: allow(arg-swap)
```

Suppresses findings whose `primary.start_line` equals the comment
line. Use this for narrow, in-line suppression.

### Standalone (whole-line) form

```python
# cntrdct: allow(arg-swap)
def looks_like_a_swap_but_intentional(b, a):
    do_something(b, a)
```

Suppresses findings whose `primary` falls anywhere inside the next
non-comment named sibling — function, class, or top-level statement
— mirroring the Rust attribute-precedes-item shape at line
granularity.

### Catch-all

```python
# cntrdct: allow()
def silence_everything():
    ...
```

Both forms accept the empty argument list as the catch-all.

## What gets matched

The argument tokens between the parentheses are split on commas and
trimmed; each must be a detector ID from `cntrdct::ALL_DETECTOR_IDS`.
Whitespace inside the parentheses is irrelevant. Unknown IDs are
accepted at parse time but have no effect — they simply do not match
any finding.

Test coverage lives in `tests/multilang_config.rs`
(`python_attribute_allow_*` cases) and `tests/suppression.rs` for the
Rust attribute paths. All three surfaces (Rust attribute, Python
standalone, Python trailing) round-trip through the same apply
helper, so the precedence rules in [cntrdct.toml
reference](./cntrdct-toml.md) apply uniformly.

## When to use which surface

| Scenario | Surface |
|---|---|
| One specific call site / function known to be a false positive | In-source comment / attribute. Document the reason inline. |
| Wholesale silencing of a detector for a vendored directory | `cntrdct.toml` `[paths] exclude` |
| Silencing a detector for an entire language | `cntrdct.toml` `[languages.<x>] suppress` |
| Disabling a detector globally | `cntrdct.toml` `[detectors.<id>] enabled = false` |

In-source suppressions are preferred for narrow, justified
exceptions because they keep the rationale next to the code. Bulk
silencing belongs in `cntrdct.toml` where it can be reviewed in one
place.

## See also

- [cntrdct.toml reference](./cntrdct-toml.md) — config-file
  precedence and per-path / per-language knobs.
- [Citation policy (P1)](../concepts/citations.md) — every
  suppression is a claim that the citing paper's prior art does not
  apply to that code; keep the comment alongside the suppression
  honest.
