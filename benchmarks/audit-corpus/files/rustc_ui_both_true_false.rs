// Source: https://github.com/rust-lang/rust/blob/29b7590130c83542a095cdf1323ed0f78eec2bb8/tests/ui/cfg/both-true-false.rs
// License: MIT OR Apache-2.0
// Note: verbatim copy of the rustc UI test for the `cfg.attr.duplicates` reference behaviour pinned at main commit 29b7590130c83542a095cdf1323ed0f78eec2bb8. Each `fn foo()` (upstream lines 7 and 11; audit-corpus lines 11 and 15 after the 3-line header + 1 blank) carries two `#[cfg(...)]` attributes whose predicates are syntactically contradictory (`#[cfg(false)] #[cfg(true)]` and `#[cfg(true)] #[cfg(false)]`), so both items are disabled under every configuration. Both entries are FN against cntrdct's config-interaction detector by docs/spec/config-interaction-v0.md F5: the detector recognises a contradiction only when one predicate is structurally `not(X)` and the other is structurally equal to `X`, while `true` and `false` are atomic primitives without the `not(...)` wrapper. No file-level suppression to strip; the upstream `//~ ERROR cannot find function foo` annotation on the call site stays verbatim and is documentation-only.

/// Test that placing a `cfg(true)` and `cfg(false)` on the same item result in
//. it being disabled.`
//@ reference: cfg.attr.duplicates

#[cfg(false)]
#[cfg(true)]
fn foo() {}

#[cfg(true)]
#[cfg(false)]
fn foo() {}

fn main() {
    foo();  //~ ERROR cannot find function `foo` in this scope
}
