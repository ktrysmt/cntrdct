// Source: signature pattern adapted from
// https://github.com/rust-lang/rust/blob/master/library/std/src/sync/mod.rs
// License: MIT OR Apache-2.0
// Note: Pattern C — the doc says "Deprecated" but no `#[deprecated]`
// attribute is attached. Drift documented in Tan et al., iComment SOSP 2007.

/// Deprecated since 0.4.0. Will be removed in 1.0.
fn legacy_init_010() -> u8 {
    0
}
