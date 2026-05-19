# In-source suppressions

Findings can be suppressed at the call site without editing
`cntrdct.toml`.

## Rust

```rust,ignore
#[cntrdct::allow(clone-drift)]
fn looks_like_a_drifted_clone_but_is_intentional() { /* ... */ }
```

The attribute precedes the item it suppresses. Empty argument list
(`#[cntrdct::allow()]`) is the catch-all.

## Python

Two forms are recognised by the tree-sitter-python suppression scanner
(Q-9):

```python
# cntrdct: allow(arg-swap)
do_something(b, a)
```

```python
do_something(b, a)  # cntrdct: allow(arg-swap)
```

The standalone whole-line form suppresses the next non-comment named
sibling (function, class, or statement) — mirroring the Rust
attribute-precedes-item shape. The trailing form suppresses only the
single comment line. `# cntrdct: allow()` is the catch-all.

All three suppression paths (Rust attribute, Python standalone, Python
trailing) are covered by `tests/multilang_config.rs`.
