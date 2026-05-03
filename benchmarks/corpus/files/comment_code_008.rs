// Source: signature pattern adapted from
// https://github.com/dtolnay/thiserror/blob/master/src/lib.rs
// License: MIT OR Apache-2.0
// Note: Pattern C — the doc says "Deprecated" but no `#[deprecated]` attribute
// is attached. Drift documented in Tan et al., iComment SOSP 2007.

/// Deprecated: use `format_v2` instead.
fn format_v1_008(value: i64) -> String {
    format!("{}", value)
}
