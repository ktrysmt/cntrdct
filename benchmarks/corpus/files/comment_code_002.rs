// Source: signature pattern adapted from
// https://github.com/dtolnay/anyhow/blob/master/src/context.rs (with_context style)
// License: MIT OR Apache-2.0
// Note: Pattern A — the doc comment claims "may fail" but the return type is
// `String` (no Result / Option). Drift documented in Tan et al., iComment SOSP 2007.

/// May fail when the input is empty; the caller should treat the returned
/// string as best-effort.
fn label_002(input: &str) -> String {
    input.to_string()
}
